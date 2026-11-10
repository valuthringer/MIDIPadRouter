//! Shared state, config application and its side effects (shortcuts, tray, autostart).

use crate::config::{self, Config, ConfigError};
use crate::midi::{ports, router::Router};
use crate::{devices::DEVICE, events, lock, tray};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

// Lock order when nesting: config -> router. Never the reverse.
pub struct AppState {
    pub config: Mutex<Config>,
    pub path: PathBuf,
    pub router: Arc<Mutex<Router>>,
    shortcuts: Mutex<Vec<(Shortcut, String)>>,
    pub shortcut_errors: Mutex<Vec<String>>,
    /// Config load problem, shown once by the UI.
    pub warning: Mutex<Option<ConfigError>>,
}

fn virtual_ports(c: &Config) -> bool {
    cfg!(unix) && c.settings.virtual_ports
}

pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let path = app.path().app_config_dir()?.join("config.json");
    let (cfg, warning) = config::load_or_default(&path);
    let (ui_tx, ui_rx) = mpsc::sync_channel(1);
    let (wd_tx, wd_rx) = mpsc::sync_channel(1);
    let router = Arc::new(Mutex::new(Router::new(DEVICE, Some(ui_tx), Some(wd_tx))));
    lock(&router).apply(cfg.active(), virtual_ports(&cfg));
    let start_minimized = cfg.settings.start_minimized;
    app.manage(AppState {
        config: Mutex::new(cfg),
        path,
        router: router.clone(),
        shortcuts: Default::default(),
        shortcut_errors: Default::default(),
        warning: Mutex::new(warning),
    });
    ports::spawn_watchdog(router, wd_rx);
    events::spawn_emitter(app.clone(), ui_rx);
    tray::create(app)?;
    sync_side_effects(app);
    if !start_minimized {
        show_window(app);
    }
    Ok(())
}

/// Validate, save, then apply a config coming from the UI or the tray.
/// `activeTarget` is owned by the backend (pads, shortcuts, tray change it).
pub fn apply_config(app: &AppHandle, mut new: Config) -> Result<Vec<String>, ConfigError> {
    persist_active(app);
    let st = app.state::<AppState>();
    {
        let mut cur = lock(&st.config);
        for (name, p) in new.profiles.iter_mut() {
            if let Some(old) = cur.profiles.get(name) {
                p.active_target.clone_from(&old.active_target);
            }
            if !p.targets.iter().any(|t| t.id == p.active_target) {
                p.active_target = p.targets.first().map(|t| t.id.clone()).unwrap_or_default();
            }
        }
        new.validate()?;
        config::save(&st.path, &new)?;
        lock(&st.router).apply(new.active(), virtual_ports(&new));
        *cur = new;
    }
    sync_side_effects(app);
    let errors = lock(&st.shortcut_errors).clone();
    Ok(errors)
}

/// Store the router's active target in the config (after a pad/shortcut/tray switch).
pub fn persist_active(app: &AppHandle) {
    let st = app.state::<AppState>();
    let id = {
        let r = lock(&st.router);
        r.targets.get(r.active).map(|t| t.id.clone())
    };
    let Some(id) = id else { return };
    {
        let mut c = lock(&st.config);
        let p = c.active_mut();
        if p.active_target == id || !p.targets.iter().any(|t| t.id == id) {
            return;
        }
        p.active_target = id;
        let _ = config::save(&st.path, &c);
    }
    tray::refresh(app);
}

fn sync_side_effects(app: &AppHandle) {
    let st = app.state::<AppState>();
    let (shortcuts, autostart) = {
        let c = lock(&st.config);
        // Shortcuts only switch targets: keep the keys free for other programs in combined mode.
        let p = c.active();
        let shortcuts = match p.mode {
            crate::config::Mode::Switch => p.shortcuts.clone(),
            crate::config::Mode::Combined => Default::default(),
        };
        (shortcuts, c.settings.autostart)
    };
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut map = Vec::new();
    let mut errors = Vec::new();
    for (id, acc) in shortcuts {
        if acc.trim().is_empty() {
            continue;
        }
        match acc.parse::<Shortcut>() {
            Ok(sc) if gs.register(sc).is_ok() => map.push((sc, id)),
            _ => errors.push(acc), // invalid or already taken by another program
        }
    }
    *lock(&st.shortcuts) = map;
    *lock(&st.shortcut_errors) = errors;

    let al = app.autolaunch();
    if al.is_enabled().unwrap_or(!autostart) != autostart {
        let _ = if autostart { al.enable() } else { al.disable() };
    }
    tray::refresh(app);
}

pub fn on_shortcut(app: &AppHandle, sc: &Shortcut) {
    let st = app.state::<AppState>();
    let id = lock(&st.shortcuts)
        .iter()
        .find(|(s, _)| s == sc)
        .map(|(_, id)| id.clone());
    if let Some(id) = id {
        let mut r = lock(&st.router);
        if let Some(i) = r.index_of(&id) {
            r.set_active(i);
        }
    }
}

/// Show the window, creating it if it was closed (it is destroyed on close to save memory).
pub fn show_window(app: &AppHandle) {
    let w = match app.get_webview_window("main") {
        Some(w) => w,
        None => {
            let Some(cfg) = app.config().app.windows.first() else {
                return;
            };
            match tauri::WebviewWindowBuilder::from_config(app, cfg).and_then(|b| b.build()) {
                Ok(w) => w,
                Err(_) => return,
            }
        }
    };
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
}
