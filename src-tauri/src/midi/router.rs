//! Hot path: pad -> target routing and LED -> Launchpad filtering.
//! Called from the midir callbacks under one short mutex; no allocation, no I/O
//! other than MIDI sends, front-end only notified through a bounded channel.

use crate::config::{parse_key_id, Mode, Profile};
use crate::devices::{AppMsg, DeviceProfile, Key, KEY_COUNT};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::mpsc::SyncSender;

/// Output side of a MIDI port. Mockable for tests.
pub trait MidiOut: Send {
    /// `false` when the port is gone.
    fn send(&mut self, msg: &[u8]) -> bool;
}

pub const MONITOR_LEN: usize = 50;

pub struct Target {
    pub id: String,
    pub port_to_app: String,
    pub port_from_app: String,
    pub enabled: bool,
    pub out: Option<Box<dyn MidiOut>>,
    /// Input from the application is open (set by the port watchdog).
    pub in_ok: bool,
    /// Last LED value the application wrote, per key (kept even when not shown).
    pub leds: [u8; KEY_COUNT],
    rapid: Option<Key>,
    activity: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchpadStatus {
    Ok,
    Missing,
    Busy,
    Unsupported,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub from_launchpad: bool,
    pub target: Option<u8>,
    pub bytes: [u8; 3],
    pub len: u8,
    pub routed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSnap {
    pub to_app: bool,
    pub from_app: bool,
    pub activity: bool,
    pub leds: Vec<u8>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub active: usize,
    pub launchpad: LaunchpadStatus,
    pub preview: bool,
    pub targets: Vec<TargetSnap>,
    pub monitor: Option<Vec<Frame>>,
}

pub struct Router {
    pub device: &'static dyn DeviceProfile,
    pub mode: Mode,
    pub active: usize,
    pub switch_key: Option<Key>,
    /// Precomputed owner (target index) of every key, combined mode.
    pub owners: [Option<u8>; KEY_COUNT],
    pub targets: Vec<Target>,
    pub launchpad: Option<Box<dyn MidiOut>>,
    pub launchpad_status: LaunchpadStatus,
    pub virtual_ports: bool,
    pub preview: bool,
    monitor: Option<VecDeque<Frame>>,
    /// Wakes the UI emitter (throttled there).
    ui: Option<SyncSender<()>>,
    /// Wakes the port watchdog (a port died or the config changed).
    watchdog: Option<SyncSender<()>>,
}

fn ping(tx: &Option<SyncSender<()>>) {
    if let Some(tx) = tx {
        let _ = tx.try_send(()); // full = already pending, fine
    }
}

impl Router {
    pub fn new(
        device: &'static dyn DeviceProfile,
        ui: Option<SyncSender<()>>,
        watchdog: Option<SyncSender<()>>,
    ) -> Self {
        Router {
            device,
            mode: Mode::Switch,
            active: 0,
            switch_key: None,
            owners: [None; KEY_COUNT],
            targets: Vec::new(),
            launchpad: None,
            launchpad_status: LaunchpadStatus::Missing,
            virtual_ports: false,
            preview: false,
            monitor: None,
            ui,
            watchdog,
        }
    }

    pub fn ping_ui(&self) {
        ping(&self.ui);
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.targets.iter().position(|t| t.id == id)
    }

    /// Apply a (validated) profile. LED memory and open outputs survive when the
    /// target id (and port) is unchanged.
    pub fn apply(&mut self, p: &Profile, virtual_ports: bool) {
        let mut old = std::mem::take(&mut self.targets);
        self.targets = p
            .targets
            .iter()
            .map(|c| {
                let prev = old
                    .iter()
                    .position(|t| t.id == c.id)
                    .map(|i| old.swap_remove(i));
                let mut t = Target {
                    id: c.id.clone(),
                    port_to_app: c.port_to_app.clone(),
                    port_from_app: c.port_from_app.clone(),
                    enabled: c.enabled,
                    out: None,
                    in_ok: false,
                    leds: [0; KEY_COUNT],
                    rapid: None,
                    activity: false,
                };
                if let Some(prev) = prev {
                    t.leds = prev.leds;
                    if c.enabled && virtual_ports == self.virtual_ports {
                        if prev.port_to_app == c.port_to_app {
                            t.out = prev.out;
                        }
                        t.in_ok = prev.in_ok && prev.port_from_app == c.port_from_app;
                    }
                }
                t
            })
            .collect();
        self.virtual_ports = virtual_ports;
        self.mode = p.mode;
        self.active = self.index_of(&p.active_target).unwrap_or(0);
        self.switch_key = p.switch_key.and_then(|k| k.key());
        self.owners = [None; KEY_COUNT];
        for (key, id) in &p.assignment {
            if let (Some(k), Some(i)) = (parse_key_id(key), self.index_of(id)) {
                self.owners[k] = Some(i as u8);
            }
        }
        self.repaint();
        ping(&self.watchdog);
        ping(&self.ui);
    }

    // ---- Launchpad -> applications ----

    pub fn on_launchpad(&mut self, msg: &[u8]) {
        let Some((key, pressed)) = self.device.parse_input(msg) else {
            return;
        };
        if Some(key) == self.switch_key {
            // Reserved key: never forwarded.
            if pressed && self.mode == Mode::Switch {
                self.cycle_active();
            }
            self.log(true, None, msg, false);
            return;
        }
        let dest = match self.mode {
            Mode::Switch => Some(self.active),
            Mode::Combined => self.owners[key].map(usize::from),
        };
        let routed = dest.is_some_and(|i| self.send_to_app(i, msg));
        self.log(true, dest.map(|i| i as u8), msg, routed);
        ping(&self.ui);
    }

    fn send_to_app(&mut self, i: usize, msg: &[u8]) -> bool {
        let Some(t) = self.targets.get_mut(i).filter(|t| t.enabled) else {
            return false;
        };
        let Some(out) = t.out.as_mut() else {
            return false;
        };
        if out.send(msg) {
            t.activity = true;
            return true;
        }
        t.out = None; // port vanished: this target goes red, the others keep working
        ping(&self.watchdog);
        false
    }

    // ---- applications -> Launchpad ----

    pub fn on_app(&mut self, i: usize, msg: &[u8]) {
        let Some(t) = self.targets.get_mut(i) else {
            return;
        };
        t.activity = true;
        let parsed = self.device.parse_app(msg, &mut t.rapid);
        let routed = match parsed {
            AppMsg::Led(k, v, second) => {
                let a = self.set_led(i, k, v);
                let b = second.is_some_and(|(k2, v2)| self.set_led(i, k2, v2));
                a || b
            }
            AppMsg::Reset => self.reset_target(i),
            AppMsg::Forward => {
                let shown = self.mode == Mode::Combined || i == self.active;
                !self.preview && shown && self.send_lp(msg)
            }
            AppMsg::Block => false,
        };
        self.log(false, Some(i as u8), msg, routed);
        ping(&self.ui);
    }

    /// Is the LED of key `k` written by target `i` currently shown on the hardware?
    fn visible(&self, i: usize, k: Key) -> bool {
        if self.preview || Some(k) == self.switch_key {
            return false;
        }
        match self.mode {
            Mode::Switch => i == self.active,
            Mode::Combined => self.owners[k] == Some(i as u8),
        }
    }

    fn set_led(&mut self, i: usize, k: Key, v: u8) -> bool {
        self.targets[i].leds[k] = v;
        self.visible(i, k) && {
            let m = self.device.led_msg(k, v);
            self.send_lp(&m)
        }
    }

    /// Emulate a reset on the target's buffer instead of resetting the whole device.
    fn reset_target(&mut self, i: usize) -> bool {
        self.targets[i].leds = [0; KEY_COUNT];
        let mut any = false;
        for k in 0..KEY_COUNT {
            if self.visible(i, k) {
                let m = self.device.led_msg(k, 0);
                any |= self.send_lp(&m);
            }
        }
        any
    }

    fn send_lp(&mut self, msg: &[u8]) -> bool {
        let Some(lp) = self.launchpad.as_mut() else {
            return false;
        };
        if lp.send(msg) {
            return true;
        }
        self.lost_launchpad();
        false
    }

    fn lost_launchpad(&mut self) {
        self.launchpad = None;
        self.launchpad_status = LaunchpadStatus::Missing;
        ping(&self.watchdog);
        ping(&self.ui);
    }

    // ---- state changes ----

    fn cycle_active(&mut self) {
        let n = self.targets.len();
        if let Some(next) = (1..=n)
            .map(|d| (self.active + d) % n)
            .find(|&i| self.targets[i].enabled)
        {
            self.set_active(next);
        }
    }

    pub fn set_active(&mut self, i: usize) {
        if i < self.targets.len() && i != self.active {
            self.active = i;
            self.repaint();
            ping(&self.ui);
        }
    }

    pub fn set_preview(&mut self, on: bool) {
        self.preview = on;
        self.repaint();
        ping(&self.ui);
    }

    pub fn set_monitor(&mut self, on: bool) {
        self.monitor = on.then(|| VecDeque::with_capacity(MONITOR_LEN));
    }

    /// What the hardware should show right now.
    pub fn frame(&self) -> [u8; KEY_COUNT] {
        let mut f = [0u8; KEY_COUNT];
        for (k, v) in f.iter_mut().enumerate() {
            let owner = match self.mode {
                Mode::Switch => Some(self.active),
                Mode::Combined => self.owners[k].map(usize::from),
            };
            *v = match (owner, self.preview) {
                (Some(i), true) => self.device.preview_color(i),
                (Some(i), false) => self.targets.get(i).map_or(0, |t| t.leds[k]),
                (None, _) => 0,
            };
        }
        if let Some(sk) = self.switch_key {
            // The switch key shows the active target's colour in switch mode.
            f[sk] = match self.mode {
                Mode::Switch => self.device.preview_color(self.active),
                Mode::Combined => 0,
            };
        }
        f
    }

    /// Clear the hardware and replay the visible state.
    pub fn repaint(&mut self) {
        let frame = self.frame();
        let Some(lp) = self.launchpad.as_mut() else {
            return;
        };
        let mut ok = true;
        self.device.frame(&frame, &mut |m| ok &= lp.send(m));
        if !ok {
            self.lost_launchpad();
        }
    }

    fn log(&mut self, from_launchpad: bool, target: Option<u8>, msg: &[u8], routed: bool) {
        if let Some(m) = self.monitor.as_mut() {
            if m.len() == MONITOR_LEN {
                m.pop_front();
            }
            let mut bytes = [0u8; 3];
            let len = msg.len().min(3);
            bytes[..len].copy_from_slice(&msg[..len]);
            m.push_back(Frame {
                from_launchpad,
                target,
                bytes,
                len: len as u8,
                routed,
            });
        }
    }

    /// State for the UI. Clears the activity flags.
    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            active: self.active,
            launchpad: self.launchpad_status,
            preview: self.preview,
            targets: self
                .targets
                .iter_mut()
                .map(|t| TargetSnap {
                    to_app: t.out.is_some(),
                    from_app: t.in_ok,
                    activity: std::mem::take(&mut t.activity),
                    leds: t.leds.to_vec(),
                })
                .collect(),
            monitor: self.monitor.as_ref().map(|m| m.iter().copied().collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Integration-style tests with fake MIDI ports.
    use super::*;
    use crate::config::Config;
    use crate::devices::launchpad_mini_mk1::{LED_OFF, RAPID, RESET};
    use crate::devices::DEVICE;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    type Log = Arc<Mutex<Vec<Vec<u8>>>>;

    struct Fake {
        log: Log,
        alive: Arc<Mutex<bool>>,
    }
    impl MidiOut for Fake {
        fn send(&mut self, msg: &[u8]) -> bool {
            if !*self.alive.lock().unwrap() {
                return false;
            }
            self.log.lock().unwrap().push(msg.to_vec());
            true
        }
    }
    fn fake() -> (Box<dyn MidiOut>, Log, Arc<Mutex<bool>>) {
        let log: Log = Default::default();
        let alive = Arc::new(Mutex::new(true));
        let f = Fake {
            log: log.clone(),
            alive: alive.clone(),
        };
        (Box::new(f), log, alive)
    }
    fn take(l: &Log) -> Vec<Vec<u8>> {
        std::mem::take(&mut *l.lock().unwrap())
    }

    struct Rig {
        r: Router,
        lp: Log,
        apps: Vec<(Log, Arc<Mutex<bool>>)>,
    }

    fn rig(mode: Mode) -> Rig {
        let mut c = Config::default();
        c.active_mut().mode = mode;
        let mut r = Router::new(DEVICE, None, None);
        r.apply(c.active(), false);
        let (lp_out, lp, _) = fake();
        r.launchpad = Some(lp_out);
        let apps = r
            .targets
            .iter_mut()
            .map(|t| {
                let (o, log, alive) = fake();
                t.out = Some(o);
                (log, alive)
            })
            .collect();
        Rig { r, lp, apps }
    }

    const PAD_LEFT: [u8; 3] = [0x90, 0, 127]; // owned by A in the default profile
    const PAD_RIGHT: [u8; 3] = [0x90, 7, 127]; // owned by B
    const SWITCH: [u8; 3] = [0xB0, 111, 127];

    #[test]
    fn switch_mode_routes_everything_to_active_only() {
        let mut g = rig(Mode::Switch);
        g.r.on_launchpad(&PAD_LEFT);
        g.r.on_launchpad(&PAD_RIGHT);
        assert_eq!(
            take(&g.apps[0].0),
            vec![PAD_LEFT.to_vec(), PAD_RIGHT.to_vec()]
        );
        assert!(take(&g.apps[1].0).is_empty());
    }

    #[test]
    fn switch_key_is_reserved_and_replays_led_state() {
        let mut g = rig(Mode::Switch);
        // B lights pad 5 while hidden: buffered, not shown.
        g.r.on_app(1, &[0x90, 5, 0x3C]);
        assert!(take(&g.lp).is_empty());
        let t = Instant::now();
        g.r.on_launchpad(&SWITCH);
        g.r.on_launchpad(&[0xB0, 111, 0]);
        assert!(t.elapsed().as_millis() < 100);
        assert_eq!(g.r.active, 1);
        assert!(take(&g.apps[0].0).is_empty() && take(&g.apps[1].0).is_empty());
        let sent = take(&g.lp);
        assert_eq!(sent[0], RESET);
        assert_eq!(sent.len(), 41);
        assert_eq!(sent[3], [RAPID, LED_OFF, 0x3C]); // keys 4,5
                                                     // Now B's LEDs go straight through, A's are only buffered.
        g.r.on_app(1, &[0x90, 0, 0x0F]);
        g.r.on_app(0, &[0x90, 1, 0x0F]);
        assert_eq!(take(&g.lp), vec![vec![0x90, 0, 0x0F]]);
        assert_eq!(g.r.targets[0].leds[1], 0x0F);
    }

    #[test]
    fn switch_skips_disabled_targets() {
        let mut g = rig(Mode::Switch);
        g.r.targets[1].enabled = false;
        g.r.on_launchpad(&SWITCH);
        assert_eq!(g.r.active, 0);
    }

    #[test]
    fn combined_mode_key_reaches_only_its_owner() {
        let mut g = rig(Mode::Combined);
        for row in 0..8u8 {
            for col in 0..9u8 {
                g.r.on_launchpad(&[0x90, row * 16 + col, 127]);
            }
        }
        for cc in 104..=111u8 {
            g.r.on_launchpad(&[0xB0, cc, 127]);
        }
        let a = take(&g.apps[0].0);
        let b = take(&g.apps[1].0);
        let owner = |m: &Vec<u8>| {
            let k = DEVICE.parse_input(m).unwrap().0;
            g.r.owners[k]
        };
        assert!(a.iter().all(|m| owner(m) == Some(0)));
        assert!(b.iter().all(|m| owner(m) == Some(1)));
        assert_eq!(a.len() + b.len(), 79); // 80 keys minus the reserved switch key
    }

    #[test]
    fn combined_mode_filters_leds_by_owner() {
        let mut g = rig(Mode::Combined);
        g.r.on_app(1, &[0x90, 0, 0x3C]); // B writes on A's key: buffered only
        g.r.on_app(0, &[0x90, 0, 0x0F]);
        g.r.on_app(1, &[0x90, 7, 0x3C]);
        assert_eq!(take(&g.lp), vec![vec![0x90, 0, 0x0F], vec![0x90, 7, 0x3C]]);
        assert_eq!(g.r.targets[1].leds[0], 0x3C);
    }

    #[test]
    fn app_reset_only_clears_its_own_keys() {
        let mut g = rig(Mode::Combined);
        g.r.on_app(0, &[0x90, 0, 0x0F]);
        take(&g.lp);
        g.r.on_app(1, &RESET);
        let sent = take(&g.lp);
        assert!(!sent.contains(&RESET.to_vec()));
        assert!(sent.iter().all(|m| {
            let k = DEVICE.parse_input(m).unwrap().0;
            g.r.owners[k] == Some(1) && m[2] == LED_OFF
        }));
        assert_eq!(g.r.targets[0].leds[0], 0x0F);
    }

    #[test]
    fn layout_and_buffer_messages_never_reach_the_launchpad() {
        let mut g = rig(Mode::Switch);
        for m in [[0xB0, 0, 2], [0xB0, 0, 1], [0xB0, 0, 0x31], [0xB0, 0, 0x7F]] {
            g.r.on_app(0, &m);
        }
        assert!(take(&g.lp).is_empty());
        g.r.on_app(0, &[0xB0, 0, 0x28]);
        assert_eq!(take(&g.lp), vec![vec![0xB0, 0, 0x28]]);
    }

    #[test]
    fn rapid_update_from_app_becomes_individual_messages() {
        let mut g = rig(Mode::Combined);
        g.r.on_app(0, &[RAPID, 0x3C, 0x0F]); // keys 0 (A) and 1 (A)
        assert_eq!(take(&g.lp), vec![vec![0x90, 0, 0x3C], vec![0x90, 1, 0x0F]]);
    }

    #[test]
    fn lost_target_port_does_not_affect_others() {
        let mut g = rig(Mode::Combined);
        *g.apps[1].1.lock().unwrap() = false; // B's port disappears
        g.r.on_launchpad(&PAD_RIGHT);
        assert!(g.r.targets[1].out.is_none());
        g.r.on_launchpad(&PAD_LEFT);
        assert_eq!(take(&g.apps[0].0), vec![PAD_LEFT.to_vec()]);
        let s = g.r.snapshot();
        assert!(s.targets[0].to_app && !s.targets[1].to_app);
    }

    #[test]
    fn lost_launchpad_is_reported() {
        let mut g = rig(Mode::Switch);
        g.r.launchpad = Some(Box::new(Fake {
            log: Default::default(),
            alive: Arc::new(Mutex::new(false)),
        }));
        g.r.launchpad_status = LaunchpadStatus::Ok;
        g.r.on_app(0, &[0x90, 0, 0x3C]);
        assert!(g.r.launchpad.is_none());
        assert_eq!(g.r.launchpad_status, LaunchpadStatus::Missing);
    }

    #[test]
    fn apply_keeps_led_memory() {
        let mut g = rig(Mode::Switch);
        g.r.on_app(1, &[0x90, 5, 0x3C]);
        let mut c = Config::default();
        c.active_mut().mode = Mode::Combined;
        g.r.apply(c.active(), false);
        assert_eq!(g.r.targets[1].leds[5], 0x3C);
        assert!(g.r.targets[1].out.is_some());
    }

    #[test]
    fn preview_shows_owner_colours_and_suspends_leds() {
        let mut g = rig(Mode::Combined);
        g.r.set_preview(true);
        let f = g.r.frame();
        assert_eq!(f[0], DEVICE.preview_color(0));
        assert_eq!(f[7], DEVICE.preview_color(1));
        take(&g.lp);
        g.r.on_app(0, &[0x90, 0, 0x0F]);
        assert!(take(&g.lp).is_empty());
        g.r.set_preview(false);
        assert_eq!(g.r.frame()[0], 0x0F);
    }

    #[test]
    fn monitor_is_bounded() {
        let mut g = rig(Mode::Switch);
        g.r.set_monitor(true);
        for _ in 0..200 {
            g.r.on_launchpad(&PAD_LEFT);
        }
        assert_eq!(g.r.snapshot().monitor.unwrap().len(), MONITOR_LEN);
    }

    #[test]
    fn routing_latency_under_1ms() {
        let mut g = rig(Mode::Combined);
        let n = 10_000u32;
        let t = Instant::now();
        for i in 0..n {
            g.r.on_launchpad(&[0x90, (i % 8) as u8, 127]);
            g.r.on_app(0, &[0x90, 0, (i % 64) as u8]);
        }
        let per_msg = t.elapsed() / (2 * n);
        println!("routing cost per message: {per_msg:?}");
        assert!(per_msg.as_micros() < 1000);
    }
}
