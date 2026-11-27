use super::localization::{error_key, text};
use crate::{
    model::Login,
    state::{Action, AppState},
};
use std::sync::Arc;
use tauri::{
    App, AppHandle, Emitter, Manager,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
};

const TRAY_ID: &str = "cloud-session-tray";

pub(super) fn setup(app: &App, state: &AppState) -> tauri::Result<()> {
    let image = if cfg!(target_os = "macos") {
        include_bytes!("../../icons/tray-template.png").as_slice()
    } else {
        include_bytes!("../../icons/tray-windows.png").as_slice()
    };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(image)?)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Cloud Session Tray")
        .menu(&tray_menu(app.handle(), state)?)
        .on_menu_event(|app, event| tray_action(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

pub(super) fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn tray_menu(app: &AppHandle, state: &AppState) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    let language = state.language();
    menu.append(&MenuItem::with_id(
        app,
        "open",
        text(language, "native.open"),
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    for session in state.sessions() {
        let check = MenuItem::with_id(
            app,
            format!("check:{}", session.id),
            text(language, "common.checkStatus"),
            true,
            None::<&str>,
        )?;
        let renew = MenuItem::with_id(
            app,
            format!("renew:{}", session.id),
            text(language, "common.renew"),
            true,
            None::<&str>,
        )?;
        // Native menus treat ampersands as mnemonic markers.
        let title = session.label.replace('&', "&&");
        menu.append(&Submenu::with_items(app, title, true, &[&check, &renew])?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        text(language, "native.quit"),
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

pub(super) fn refresh_tray(app: &AppHandle) {
    let app_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app_handle.tray_by_id(TRAY_ID)
            && let Ok(menu) = tray_menu(&app_handle, &app_handle.state::<Arc<AppState>>())
        {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

fn tray_action(app: &AppHandle, menu_id: &str) {
    let state = app.state::<Arc<AppState>>();
    if menu_id == "open" {
        show_window(app);
        return;
    }
    if menu_id == "quit" {
        log::info!("Quit requested from the system tray");
        app.exit(0);
        return;
    }
    let Some((action, id)) = menu_id.split_once(':') else {
        return;
    };
    show_window(app);
    let _ = app.emit_to("main", "select-session", id);
    let result = match action {
        "check" => state.start(id, Action::Check),
        "renew" => match state
            .sessions()
            .into_iter()
            .find(|session| session.id == id)
        {
            Some(session) if matches!(session.login, Login::IamMfa(_)) => {
                let _ = app.emit_to("main", "request-otp", id);
                return;
            }
            Some(_) => state.start(id, Action::Sso),
            None => return,
        },
        _ => return,
    };
    if let Err(error) = result {
        log::warn!("Tray session request failed: code={}", error_key(&error));
        let _ = app.emit_to("main", "session-error", error);
    }
}
