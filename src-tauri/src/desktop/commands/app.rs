use crate::{
    desktop::{logging::operation, tray::refresh_tray},
    model::{Language, Preferences},
    state::{AppState, Dashboard},
};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

const DOCS_PREFIX: &str = "https://docs.aws.amazon.com/cli/latest/userguide/";

#[tauri::command]
pub(in crate::desktop) async fn dashboard(
    state: State<'_, Arc<AppState>>,
) -> Result<Dashboard, String> {
    Ok(state.dashboard())
}

#[tauri::command]
pub(in crate::desktop) async fn save_preferences(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    preferences: Preferences,
) -> Result<(), String> {
    operation("App preferences save", state.save_preferences(preferences))?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
pub(in crate::desktop) async fn set_system_language(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    language: Language,
) -> Result<(), String> {
    state.set_system_language(language);
    refresh_tray(&app);
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::desktop) enum Documentation {
    Installation,
    Uninstallation,
    Sso,
    Iam,
}

#[tauri::command]
pub(in crate::desktop) fn open_docs(app: AppHandle, topic: Documentation) -> Result<(), String> {
    let page = match topic {
        Documentation::Installation => "getting-started-install.html",
        Documentation::Uninstallation => "uninstall.html",
        Documentation::Sso => "cli-configure-sso.html",
        Documentation::Iam => "cli-configure-files.html",
    };
    app.opener()
        .open_url(format!("{DOCS_PREFIX}{page}"), None::<&str>)
        .map_err(|_| "errors.openAwsDocs".into())
}

#[tauri::command]
pub(in crate::desktop) fn open_log_folder(app: AppHandle) -> Result<(), String> {
    let error = "errors.logFolder";
    let folder = app.path().app_log_dir().map_err(|_| error)?;
    std::fs::create_dir_all(&folder).map_err(|_| error)?;
    app.opener()
        .open_path(folder.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|_| error.into())
}
