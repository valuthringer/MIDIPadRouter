//! Port enumeration, opening, and the hot-plug watchdog thread.

use super::router::{LaunchpadStatus, MidiOut, Router};
use crate::devices::PortMatch;
use crate::lock;
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Serialize;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CLIENT: &str = "MIDIPadRouter";

struct Out(MidiOutputConnection);

impl MidiOut for Out {
    fn send(&mut self, msg: &[u8]) -> bool {
        self.0.send(msg).is_ok()
    }
}

#[derive(Serialize, Default)]
pub struct PortList {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

pub fn list_ports() -> PortList {
    let inputs = MidiInput::new(CLIENT)
        .map(|m| {
            m.ports()
                .iter()
                .filter_map(|p| m.port_name(p).ok())
                .collect()
        })
        .unwrap_or_default();
    let outputs = MidiOutput::new(CLIENT)
        .map(|m| {
            m.ports()
                .iter()
                .filter_map(|p| m.port_name(p).ok())
                .collect()
        })
        .unwrap_or_default();
    PortList { inputs, outputs }
}

fn open_in<F>(name: &str, virtual_port: bool, cb: F) -> Option<MidiInputConnection<()>>
where
    F: FnMut(u64, &[u8], &mut ()) + Send + 'static,
{
    let mut mi = MidiInput::new(CLIENT).ok()?;
    mi.ignore(Ignore::All); // SysEx, clock and active sensing are never routed
    #[cfg(unix)]
    if virtual_port {
        use midir::os::unix::VirtualInput;
        return mi.create_virtual(name, cb, ()).ok();
    }
    let _ = virtual_port;
    let port = mi
        .ports()
        .into_iter()
        .find(|p| mi.port_name(p).ok().as_deref() == Some(name))?;
    mi.connect(&port, "midipr-in", cb, ()).ok()
}

fn open_out(name: &str, virtual_port: bool) -> Option<Box<dyn MidiOut>> {
    let mo = MidiOutput::new(CLIENT).ok()?;
    #[cfg(unix)]
    if virtual_port {
        use midir::os::unix::VirtualOutput;
        return mo
            .create_virtual(name)
            .ok()
            .map(|c| Box::new(Out(c)) as Box<dyn MidiOut>);
    }
    let _ = virtual_port;
    let port = mo
        .ports()
        .into_iter()
        .find(|p| mo.port_name(p).ok().as_deref() == Some(name))?;
    mo.connect(&port, "midipr-out")
        .ok()
        .map(|c| Box::new(Out(c)) as Box<dyn MidiOut>)
}

struct Wanted {
    id: String,
    to: String,
    from: String,
    has_out: bool,
}

/// Owns the input connections; outputs live in the router (hot path).
/// Input connections are always dropped with the router lock released,
/// since closing one waits for its callback, which takes that lock.
pub struct Watchdog {
    router: Arc<Mutex<Router>>,
    lp_in: Option<(String, MidiInputConnection<()>)>,
    app_ins: Vec<(String, String, MidiInputConnection<()>)>,
}

impl Watchdog {
    /// One pass: drop vanished ports, open missing ones. Returns true when all is connected.
    fn reconcile(&mut self) -> bool {
        let ports = list_ports();
        let (lp_has_out, virt, wanted) = {
            let r = lock(&self.router);
            let wanted: Vec<Wanted> = r
                .targets
                .iter()
                .filter(|t| t.enabled)
                .map(|t| Wanted {
                    id: t.id.clone(),
                    to: t.port_to_app.clone(),
                    from: t.port_from_app.clone(),
                    has_out: t.out.is_some(),
                })
                .collect();
            (r.launchpad.is_some(), r.virtual_ports, wanted)
        };
        let device = crate::devices::DEVICE;
        let find = |names: &[String]| {
            names
                .iter()
                .find(|n| device.classify_port(n) == PortMatch::Supported)
                .cloned()
        };
        let (lp_in_name, lp_out_name) = (find(&ports.inputs), find(&ports.outputs));
        let unsupported = ports
            .inputs
            .iter()
            .any(|n| device.classify_port(n) == PortMatch::Unsupported);

        // --- Launchpad ---
        let lp_ok = lp_has_out
            && self
                .lp_in
                .as_ref()
                .is_some_and(|(n, _)| Some(n) == lp_in_name.as_ref());
        let mut new_lp = None;
        if !lp_ok {
            // Close old handles first: Windows (WinMM) devices are single-client.
            lock(&self.router).launchpad = None;
            self.lp_in = None;
            let mut out = None;
            if let (Some(i), Some(o)) = (&lp_in_name, &lp_out_name) {
                let r = self.router.clone();
                let conn = open_in(i, false, move |_, m, _| lock(&r).on_launchpad(m));
                if let (Some(conn), Some(o)) = (conn, open_out(o, false)) {
                    self.lp_in = Some((i.clone(), conn));
                    out = Some(o);
                }
            }
            new_lp = Some(out);
        }

        // --- Targets ---
        let present = |list: &[String], n: &str| virt || list.iter().any(|x| x == n);
        self.app_ins.retain(|(id, port, _)| {
            wanted.iter().any(|w| &w.id == id && &w.from == port) && present(&ports.inputs, port)
        });
        let mut opened = Vec::new();
        let mut lost = Vec::new();
        for w in &wanted {
            let has_in = self.app_ins.iter().any(|(id, ..)| id == &w.id);
            if !w.from.is_empty() && !has_in && present(&ports.inputs, &w.from) {
                let (r, id) = (self.router.clone(), w.id.clone());
                let conn = open_in(&w.from, virt, move |_, m, _| {
                    let mut g = lock(&r);
                    if let Some(i) = g.index_of(&id) {
                        g.on_app(i, m);
                    }
                });
                if let Some(c) = conn {
                    self.app_ins.push((w.id.clone(), w.from.clone(), c));
                }
            }
            let out_present = !w.to.is_empty() && present(&ports.outputs, &w.to);
            if !w.has_out && out_present {
                if let Some(o) = open_out(&w.to, virt) {
                    opened.push((w.id.clone(), w.to.clone(), o));
                }
            } else if w.has_out && !out_present {
                lost.push(w.id.clone());
            }
        }

        // --- Install under the lock ---
        let mut r = lock(&self.router);
        if let Some(out) = new_lp {
            r.launchpad_status = match (&out, &lp_in_name, unsupported) {
                (Some(_), _, _) => LaunchpadStatus::Ok,
                // Present but not openable: another program holds it.
                (None, Some(_), _) => LaunchpadStatus::Busy,
                (None, None, true) => LaunchpadStatus::Unsupported,
                (None, None, false) => LaunchpadStatus::Missing,
            };
            r.launchpad = out;
            r.repaint();
        }
        for (id, port, o) in opened {
            if let Some(t) = r
                .targets
                .iter_mut()
                .find(|t| t.id == id && t.port_to_app == port)
            {
                t.out = Some(o);
            }
        }
        for id in lost {
            if let Some(t) = r.targets.iter_mut().find(|t| t.id == id) {
                t.out = None;
            }
        }
        let ins = &self.app_ins;
        for t in r.targets.iter_mut() {
            t.in_ok = ins
                .iter()
                .any(|(id, p, _)| id == &t.id && p == &t.port_from_app);
        }
        r.ping_ui();
        r.launchpad.is_some()
            && r.targets.iter().filter(|t| t.enabled).all(|t| {
                (t.port_to_app.is_empty() || t.out.is_some())
                    && (t.port_from_app.is_empty() || t.in_ok)
            })
    }
}

/// Reconcile ports on start, on every wake-up (config change, dead port) and
/// periodically: every 2 s then 5 s while something is missing, every 5 s otherwise
/// (see DECISIONS.md: a replugged device is invisible to midir otherwise).
pub fn spawn_watchdog(router: Arc<Mutex<Router>>, wake: Receiver<()>) {
    std::thread::Builder::new()
        .name("midipr-ports".into())
        .spawn(move || {
            let mut wd = Watchdog {
                router,
                lp_in: None,
                app_ins: Vec::new(),
            };
            let mut misses = 0u32;
            loop {
                let ok = wd.reconcile();
                misses = if ok { 0 } else { misses + 1 };
                let secs = if ok || misses > 5 { 5 } else { 2 };
                match wake.recv_timeout(Duration::from_secs(secs)) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => while wake.try_recv().is_ok() {},
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .expect("spawn port watchdog");
}
