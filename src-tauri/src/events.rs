//! UI notifications: woken by the router, throttled to <= 30 Hz,
//! nothing emitted while the window is hidden or minimized.

use crate::app_state::{persist_active, AppState};
use crate::lock;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn window_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

pub fn spawn_emitter(app: AppHandle, rx: Receiver<()>) {
    std::thread::Builder::new()
        .name("midipr-ui".into())
        .spawn(move || {
            // Blocking recv: zero CPU while idle.
            while rx.recv().is_ok() {
                std::thread::sleep(Duration::from_millis(33));
                while rx.try_recv().is_ok() {}
                persist_active(&app);
                if window_visible(&app) {
                    let snap = lock(&app.state::<AppState>().router).snapshot();
                    let _ = app.emit("snapshot", snap);
                }
            }
        })
        .expect("spawn ui emitter");
}
