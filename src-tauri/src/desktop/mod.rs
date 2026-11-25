mod commands;
mod dialogs;
mod installer;
mod profile_tools;
mod tray;

use crate::state::AppState;
use std::{sync::Arc, time::Duration};
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_notification::NotificationExt;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_window(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::dashboard,
            commands::rescan_cli,
            commands::choose_cli,
            commands::reset_cli,
            commands::profiles,
            commands::mfa_devices,
            commands::register_iam,
            commands::configure_sso,
            commands::save_session,
            commands::remove_session,
            commands::save_preferences,
            commands::check_session,
            commands::renew_mfa,
            commands::renew_sso,
            commands::cancel_session,
            commands::open_docs,
            installer::install_cli,
            profile_tools::tool_connection,
            profile_tools::test_connection,
            profile_tools::prepare_shared_profile,
            profile_tools::save_shared_profile,
            profile_tools::open_shared_profile,
            profile_tools::import_shared_sso,
        ])
        .setup(|app| {
            let state = Arc::new(AppState::new(
                app.path().home_dir()?,
                app.path().app_config_dir()?,
                tauri::async_runtime::handle().inner().clone(),
            )?);
            app.manage(state.clone());
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray::setup(app, &state)?;
            let scanner = state.clone();
            tauri::async_runtime::spawn(async move {
                scanner.rescan().await;
            });
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_secs(10));
                loop {
                    tick.tick().await;
                    for (id, label, expiry) in state.notifications() {
                        let time = expiry.at.with_timezone(&chrono::Local).format("%H:%M");
                        if app_handle
                            .notification()
                            .builder()
                            .title("AWS 세션 만료 예정")
                            .body(format!("{label} 세션이 {time}에 만료됩니다."))
                            .show()
                            .is_ok()
                        {
                            state.mark_notified(&id, &expiry);
                        }
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("start Cloud Session Tray")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                let state = app.state::<Arc<AppState>>();
                if state.has_jobs() {
                    api.prevent_exit();
                    state.begin_exit();
                    let app = app.clone();
                    let state = Arc::clone(&state);
                    tauri::async_runtime::spawn(async move {
                        while state.has_jobs() {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        app.exit(0);
                    });
                }
            }
        });
}
