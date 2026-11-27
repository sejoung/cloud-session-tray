mod cli_installations;
mod cli_release;
mod commands;
mod dialogs;
mod installer;
mod localization;
mod logging;
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
        .manage(cli_release::CliReleaseCache::default())
        .invoke_handler(tauri::generate_handler![
            commands::dashboard,
            commands::rescan_cli,
            commands::choose_cli,
            commands::reset_cli,
            cli_installations::remove_cli,
            cli_installations::set_terminal_path,
            commands::profiles,
            commands::mfa_devices,
            commands::register_iam,
            commands::configure_sso,
            commands::save_session,
            commands::remove_session,
            commands::save_preferences,
            commands::set_system_language,
            commands::check_session,
            commands::renew_mfa,
            commands::renew_sso,
            commands::cancel_session,
            commands::open_docs,
            cli_release::latest_cli_version,
            installer::install_cli,
            logging::open_log_folder,
            profile_tools::prepare_shared_profile,
            profile_tools::save_shared_profile,
            profile_tools::open_shared_profile,
            profile_tools::import_shared_sso,
        ])
        .setup(|app| {
            logging::initialize(app.handle());
            log::info!(
                "Cloud Session Tray started: version={}",
                app.package_info().version
            );
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
                        let language = state.language();
                        if app_handle
                            .notification()
                            .builder()
                            .title(localization::text(language, "native.expiryTitle"))
                            .body(localization::expiry_body(
                                language,
                                &label,
                                &time.to_string(),
                            ))
                            .show()
                            .is_ok()
                        {
                            log::info!("Session expiration notification sent");
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
