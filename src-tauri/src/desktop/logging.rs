use super::localization;
use tauri::{AppHandle, Wry, plugin::TauriPlugin};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

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
