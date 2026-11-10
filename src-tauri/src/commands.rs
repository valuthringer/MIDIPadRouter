//! Commands exposed to the front-end. No MIDI traffic goes through here.

use crate::app_state::{apply_config, AppState};
use crate::config::{self, err, Config, ConfigError, ProfileFile, CURRENT_VERSION};
use crate::lock;
use crate::midi::ports::{list_ports, PortList};
use crate::midi::router::Snapshot;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullState {
    config: Config,
    ports: PortList,
    snapshot: Snapshot,
    platform: &'static str,
    warning: Option<ConfigError>,
    shortcut_errors: Vec<String>,
}

#[tauri::command]
pub fn get_state(state: State<AppState>) -> FullState {
    FullState {
        config: lock(&state.config).clone(),
        ports: list_ports(),
        snapshot: lock(&state.router).snapshot(),
        platform: std::env::consts::OS,
        warning: lock(&state.warning).take(),
        shortcut_errors: lock(&state.shortcut_errors).clone(),
    }
}

#[tauri::command]
pub fn get_snapshot(state: State<AppState>) -> Snapshot {
    lock(&state.router).snapshot()
}

#[tauri::command]
pub fn get_ports() -> PortList {
    list_ports()
}

/// Returns the shortcuts that could not be registered.
#[tauri::command]
pub fn save_config(app: AppHandle, config: Config) -> Result<Vec<String>, ConfigError> {
    apply_config(&app, config)
}

#[tauri::command]
pub fn set_active(state: State<AppState>, index: usize) {
    lock(&state.router).set_active(index);
}

#[tauri::command]
pub fn set_preview(state: State<AppState>, on: bool) {
    lock(&state.router).set_preview(on);
}

#[tauri::command]
pub fn set_monitor(state: State<AppState>, on: bool) {
    lock(&state.router).set_monitor(on);
}

fn io(e: impl std::fmt::Display) -> ConfigError {
    err("io", e.to_string())
}

#[tauri::command]
pub async fn export_profile(app: AppHandle, name: String) -> Result<bool, ConfigError> {
    let profile = {
        let st = app.state::<AppState>();
        let c = lock(&st.config);
        let p = c.profiles.get(&name).cloned();
        p
    }
    .ok_or_else(|| err("unknownProfile", &name))?;
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name(format!("{name}.json"))
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = path.into_path().map_err(io)?;
    let file = ProfileFile {
        version: CURRENT_VERSION,
        name,
        profile,
    };
    config::write_atomic(&path, &serde_json::to_string_pretty(&file).map_err(io)?)?;
    Ok(true)
}

/// Returns the name of the imported (and now active) profile.
#[tauri::command]
pub async fn import_profile(app: AppHandle) -> Result<Option<String>, ConfigError> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(path.into_path().map_err(io)?).map_err(io)?;
    let file: ProfileFile = serde_json::from_str(&text).map_err(|e| err("parse", e.to_string()))?;
    if file.version > CURRENT_VERSION {
        return Err(err("futureVersion", file.version.to_string()));
    }
    file.profile.validate()?;
    let mut c = lock(&app.state::<AppState>().config).clone();
    let base = match file.name.trim() {
        "" => "Import".to_string(),
        n => n.to_string(),
    };
    let mut name = base.clone();
    let mut n = 2;
    while c.profiles.contains_key(&name) {
        name = format!("{base} ({n})");
        n += 1;
    }
    c.profiles.insert(name.clone(), file.profile);
    c.active_profile.clone_from(&name);
    apply_config(&app, c)?;
    Ok(Some(name))
}

/// Opens one of the known help pages in the default browser.
#[tauri::command]
pub fn open_help(app: AppHandle, page: String) {
    let url = match page.as_str() {
        "loopmidi" => "https://www.tobias-erichsen.de/software/loopmidi.html",
        "midiservices" => "https://microsoft.github.io/MIDI/",
        _ => return,
    };
    let _ = app.opener().open_url(url, None::<&str>);
}
