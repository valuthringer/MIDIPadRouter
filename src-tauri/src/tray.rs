//! Notification-area icon: active target, switch target/profile, quit.

use crate::app_state::{apply_config, show_window, AppState};
use crate::config::Mode;
use crate::lock;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

const ID: &str = "main";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut b = TrayIconBuilder::with_id(ID)
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| on_menu(app, e.id.as_ref()))
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let st = app.state::<AppState>();
    let cfg = lock(&st.config).clone();
    let active = lock(&st.router).active;
    let en = cfg.settings.language == "en";
    let tr = |fr: &'static str, e: &'static str| if en { e } else { fr };
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "show",
        tr("Afficher", "Show"),
        true,
        None::<&str>,
    )?)?;
    let p = cfg.active();
    if p.mode == Mode::Switch {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        for (i, t) in p.targets.iter().enumerate() {
            let item = CheckMenuItem::with_id(
                app,
                format!("target:{i}"),
                &t.name,
                t.enabled,
                i == active,
                None::<&str>,
            )?;
            menu.append(&item)?;
        }
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let sub = Submenu::with_id(app, "profiles", tr("Profils", "Profiles"), true)?;
    for name in cfg.profiles.keys() {
        let item = CheckMenuItem::with_id(
            app,
            format!("profile:{name}"),
            name,
            true,
            *name == cfg.active_profile,
            None::<&str>,
        )?;
        sub.append(&item)?;
    }
    menu.append(&sub)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        tr("Quitter", "Quit"),
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(ID) else {
        return;
    };
    if let Ok(m) = menu(app) {
        let _ = tray.set_menu(Some(m));
    }
    let st = app.state::<AppState>();
    let (mode, names, en) = {
        let c = lock(&st.config);
        let p = c.active();
        let names: Vec<String> = p.targets.iter().map(|t| t.name.clone()).collect();
        (p.mode, names, c.settings.language == "en")
    };
    let active = lock(&st.router).active;
    let what = match mode {
        Mode::Switch => names.get(active).cloned().unwrap_or_default(),
        Mode::Combined => (if en { "Combined" } else { "Combiné" }).to_string(),
    };
    let _ = tray.set_tooltip(Some(format!("MIDIPadRouter — {what}")));
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => show_window(app),
        "quit" => app.exit(0),
        _ => {
            let st = app.state::<AppState>();
            if let Some(i) = id.strip_prefix("target:").and_then(|s| s.parse().ok()) {
                lock(&st.router).set_active(i);
            } else if let Some(name) = id.strip_prefix("profile:") {
                let mut c = lock(&st.config).clone();
                c.active_profile = name.to_string();
                if apply_config(app, c).is_ok() {
                    let _ = app.emit("config-changed", ());
                }
            }
            refresh(app); // CheckMenuItem toggles itself; redraw the truth
        }
    }
}
