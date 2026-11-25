use super::{
    dialogs::{file_dialog, selected_file},
    tray::refresh_tray,
};
use crate::{
    model::{IamRegistration, Login, Preferences, SessionDraft},
    profiles::ProfileView,
    state::{Action, AppState, Dashboard},
};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use zeroize::Zeroizing;

const DOCS_PREFIX: &str = "https://docs.aws.amazon.com/cli/latest/userguide/";

#[tauri::command]
pub(super) async fn dashboard(state: State<'_, Arc<AppState>>) -> Result<Dashboard, String> {
    Ok(state.dashboard())
}

#[tauri::command]
pub(super) async fn rescan_cli(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.rescan().await;
    Ok(())
}

#[tauri::command]
pub(super) async fn choose_cli(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    let dialog = file_dialog(&app, "AWS CLI 실행 파일 선택");
    #[cfg(windows)]
    let dialog = dialog.add_filter("AWS CLI", &["exe"]);
    let Some(path) = selected_file(dialog, false).await? else {
        return Ok(false);
    };
    state.choose_cli(path).await?;
    Ok(true)
}

#[tauri::command]
pub(super) async fn reset_cli(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.reset_cli().await
}

#[tauri::command]
pub(super) async fn profiles(state: State<'_, Arc<AppState>>) -> Result<Vec<ProfileView>, String> {
    state.profiles().await
}

#[tauri::command]
pub(super) async fn mfa_devices(
    state: State<'_, Arc<AppState>>,
    source: String,
) -> Result<Vec<String>, String> {
    state.mfa_devices(&source).await
}

#[tauri::command]
pub(super) async fn register_iam(
    state: State<'_, Arc<AppState>>,
    registration: IamRegistration,
) -> Result<Vec<ProfileView>, String> {
    state.register_iam(registration).await
}

#[tauri::command]
pub(super) async fn configure_sso(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProfileView>, String> {
    state.wizard().await
}

#[tauri::command]
pub(super) async fn save_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: Option<String>,
    label: String,
    login: Login,
    task: Option<String>,
) -> Result<String, String> {
    let id = state.save_session(SessionDraft {
        id,
        label,
        login,
        task: task.unwrap_or_default(),
    })?;
    refresh_tray(&app);
    Ok(id)
}

#[tauri::command]
pub(super) async fn remove_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state.remove_session(&id)?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
pub(super) async fn save_preferences(
    state: State<'_, Arc<AppState>>,
    preferences: Preferences,
) -> Result<(), String> {
    state.save_preferences(preferences)
}

#[tauri::command]
pub(super) async fn check_session(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state.start(&id, Action::Check)
}

#[tauri::command]
pub(super) async fn renew_mfa(
    state: State<'_, Arc<AppState>>,
    id: String,
    otp: String,
) -> Result<(), String> {
    state.start(&id, Action::Mfa(Zeroizing::new(otp)))
}

#[tauri::command]
pub(super) async fn renew_sso(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.start(&id, Action::Sso)
}

#[tauri::command]
pub(super) fn cancel_session(state: State<'_, Arc<AppState>>, id: String) {
    state.cancel(&id);
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Documentation {
    Installation,
    Sso,
    Iam,
}

#[tauri::command]
pub(super) fn open_docs(app: AppHandle, topic: Documentation) -> Result<(), String> {
    let page = match topic {
        Documentation::Installation => "getting-started-install.html",
        Documentation::Sso => "cli-configure-sso.html",
        Documentation::Iam => "cli-configure-files.html",
    };
    app.opener()
        .open_url(format!("{DOCS_PREFIX}{page}"), None::<&str>)
        .map_err(|_| "기본 브라우저에서 AWS 문서를 열 수 없습니다.".into())
}
