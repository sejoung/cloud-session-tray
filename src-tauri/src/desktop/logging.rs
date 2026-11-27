use super::localization;
use tauri::{AppHandle, Manager, Wry, plugin::TauriPlugin};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
use tauri_plugin_opener::OpenerExt;

pub(super) fn initialize(app: &AppHandle) {
    if app.plugin(plugin()).is_err() {
        eprintln!("Could not initialize the Cloud Session Tray log file.");
    }
}

fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_log::Builder::new()
        .targets([Target::new(TargetKind::LogDir {
            file_name: Some("cloud-session-tray".into()),
        })])
        .level(log::LevelFilter::Info)
        .filter(|record| record.target().starts_with("cloud_session_tray::"))
        .max_file_size(1024 * 1024)
        .rotation_strategy(RotationStrategy::KeepSome(3))
        .build()
}

pub(super) fn operation<T>(name: &str, result: Result<T, String>) -> Result<T, String> {
    match &result {
        Ok(_) => log::info!("{name} completed"),
        Err(error) => log::warn!("{name} failed: code={}", localization::error_key(error)),
    }
    result
}

#[tauri::command]
pub(super) fn open_log_folder(app: AppHandle) -> Result<(), String> {
    let error = "errors.logFolder";
    let folder = app.path().app_log_dir().map_err(|_| error)?;
    std::fs::create_dir_all(&folder).map_err(|_| error)?;
    app.opener()
        .open_path(folder.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|_| error.into())
}
