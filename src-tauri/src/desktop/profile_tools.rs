use super::{
    dialogs::{file_dialog, selected_file},
    localization::text,
    logging::operation,
    tray::refresh_tray,
};
use crate::{
    model::{SHARE_FILE_LIMIT, SharedProfile},
    state::{AppState, ConnectionCheck, ToolConnection},
};
use std::{io::Write, path::Path, sync::Arc};
use tauri::{AppHandle, Manager, State};
use tokio::io::AsyncReadExt;

#[tauri::command]
pub(super) async fn tool_connection(
    state: State<'_, Arc<AppState>>,
    profile: String,
) -> Result<ToolConnection, String> {
    state.tool_connection(profile)
}

#[tauri::command]
pub(super) async fn test_connection(
    state: State<'_, Arc<AppState>>,
    profile: String,
) -> Result<ConnectionCheck, String> {
    operation(
        "External tool identity check",
        state.test_connection(&profile).await,
    )
}

#[tauri::command]
pub(super) async fn prepare_shared_profile(
    state: State<'_, Arc<AppState>>,
    profile: String,
    task: String,
) -> Result<SharedProfile, String> {
    state.share_profile(&profile, task)
}

#[tauri::command]
pub(super) async fn save_shared_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    shared: SharedProfile,
) -> Result<bool, String> {
    shared.validate()?;
    let language = state.language();
    let dialog = file_dialog(&app, text(language, "native.exportShare"))
        .add_filter(text(language, "native.shareFile"), &["json"])
        .set_file_name("cst-profile.json");
    let Some(path) = selected_file(dialog, true).await? else {
        return Ok(false);
    };
    for reserved in [
        state.files.config.clone(),
        state.files.credentials.clone(),
        app.path()
            .app_config_dir()
            .map_err(|_| "errors.settingsPath")?
            .join("settings.json"),
    ] {
        if same_path(&path, &reserved) {
            return Err("errors.sharePathConflict".into());
        }
    }
    operation("Profile share export", write_share(&path, &shared))?;
    Ok(true)
}

#[tauri::command]
pub(super) async fn open_shared_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<SharedProfile>, String> {
    let language = state.language();
    let dialog = file_dialog(&app, text(language, "native.importShare"))
        .add_filter(text(language, "native.shareFile"), &["json"]);
    let Some(path) = selected_file(dialog, false).await? else {
        return Ok(None);
    };
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| "errors.shareOpen")?;
    let mut bytes = Vec::new();
    file.take((SHARE_FILE_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| "errors.shareRead")?;
    if bytes.len() > SHARE_FILE_LIMIT {
        return Err("errors.shareSize".into());
    }
    let shared: SharedProfile =
        serde_json::from_slice(&bytes).map_err(|_| "errors.invalidShare")?;
    shared.validate()?;
    log::info!("Profile share file validated");
    Ok(Some(shared))
}

#[tauri::command]
pub(super) async fn import_shared_sso(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    shared: SharedProfile,
) -> Result<String, String> {
    let id = operation(
        "Shared Identity Center profile import",
        state.import_sso(shared).await,
    )?;
    refresh_tray(&app);
    Ok(id)
}

fn same_path(first: &Path, second: &Path) -> bool {
    first == second
        || first
            .canonicalize()
            .ok()
            .zip(second.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

fn write_share(path: &Path, shared: &SharedProfile) -> Result<(), String> {
    let error = "errors.shareWrite";
    let directory = path.parent().ok_or(error)?;
    let mut staged = tempfile::NamedTempFile::new_in(directory).map_err(|_| error)?;
    serde_json::to_writer_pretty(&mut staged, shared).map_err(|_| error)?;
    staged.write_all(b"\n").map_err(|_| error)?;
    staged.as_file().sync_all().map_err(|_| error)?;
    staged.persist(path).map_err(|_| error)?;
    Ok(())
}
