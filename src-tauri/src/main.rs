// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod commands;
mod config;
mod devices;
mod events;
mod midi;
mod tray;

use std::sync::{Mutex, MutexGuard};
use tauri::RunEvent;
use tauri_plugin_global_shortcut::ShortcutState;

/// Lock that survives a poisoned mutex: a panic elsewhere must not stop the routing.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn main() {
    tauri::Builder::default()
        // A second instance would fight for the Launchpad: focus the first one instead.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            app_state::show_window(app)
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, sc, ev| {
                    if ev.state() == ShortcutState::Pressed {
                        app_state::on_shortcut(app, sc);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| app_state::init(app.handle()))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_snapshot,
            commands::get_ports,
            commands::save_config,
            commands::set_active,
            commands::set_preview,
            commands::set_monitor,
            commands::export_profile,
            commands::import_profile,
            commands::open_help,
        ])
        .build(tauri::generate_context!())
        .expect("error while building MIDIPadRouter")
        .run(|_, e| {
            // Closing the window destroys the WebView (frees ~200 MB) but the router
            // keeps running in the tray. Only "Quit" (explicit exit code) stops the app.
            if let RunEvent::ExitRequested {
                api, code: None, ..
            } = e
            {
                api.prevent_exit();
            }
        });
}
