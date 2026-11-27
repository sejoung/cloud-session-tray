use super::{
    dialogs::{file_dialog, selected_file},
    localization::text,
    logging::operation,
    tray::refresh_tray,
};
use crate::{
    model::{IamRegistration, Language, Login, Preferences, SessionDraft},
    profiles::ProfileView,
    state::{Action, AppState, Dashboard},
};
use std::{path::PathBuf, sync::Arc};
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
    path: Option<PathBuf>,
) -> Result<bool, String> {
    if let Some(path) = path {
        operation(
            "AWS CLI executable selection",
            state.choose_detected_cli(path).await,
        )?;
        return Ok(true);
    }
    let dialog = file_dialog(&app, text(state.language(), "native.chooseCli"));
    #[cfg(windows)]
    let dialog = dialog.add_filter("AWS CLI", &["exe"]);
    let Some(path) = selected_file(dialog, false).await? else {
        return Ok(false);
    };
    operation("AWS CLI executable selection", state.choose_cli(path).await)?;
    Ok(true)
}

#[tauri::command]
pub(super) async fn reset_cli(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.reset_cli().await
}

#[tauri::command]
pub(super) async fn profiles(state: State<'_, Arc<AppState>>) -> Result<Vec<ProfileView>, String> {
    operation("AWS profile listing", state.profiles().await)
}

#[tauri::command]
pub(super) async fn mfa_devices(
    state: State<'_, Arc<AppState>>,
    source: String,
) -> Result<Vec<String>, String> {
    operation("MFA device lookup", state.mfa_devices(&source).await)
}

#[tauri::command]
pub(super) async fn register_iam(
    state: State<'_, Arc<AppState>>,
    registration: IamRegistration,
) -> Result<Vec<ProfileView>, String> {
    operation(
        "IAM profile registration",
        state.register_iam(registration).await,
    )
}

#[tauri::command]
pub(super) async fn configure_sso(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProfileView>, String> {
    operation("Identity Center profile setup", state.wizard().await)
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
    let id = operation(
        "Session configuration save",
        state.save_session(SessionDraft {
            id,
            label,
            login,
            task: task.unwrap_or_default(),
        }),
    )?;
    refresh_tray(&app);
    Ok(id)
}

#[tauri::command]
pub(super) async fn remove_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    operation("Session removal", state.remove_session(&id))?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
pub(super) async fn save_preferences(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    preferences: Preferences,
) -> Result<(), String> {
    operation("App preferences save", state.save_preferences(preferences))?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
pub(super) async fn set_system_language(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    language: Language,
) -> Result<(), String> {
    state.set_system_language(language);
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
pub(super) async fn check_session(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    operation("Session status request", state.start(&id, Action::Check))
}

#[tauri::command]
pub(super) async fn renew_mfa(
    state: State<'_, Arc<AppState>>,
    id: String,
    otp: String,
) -> Result<(), String> {
    operation(
        "IAM MFA renewal request",
        state.start(&id, Action::Mfa(Zeroizing::new(otp))),
    )
}

#[tauri::command]
pub(super) async fn renew_sso(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    operation(
        "Identity Center sign-in request",
        state.start(&id, Action::Sso),
    )
}

#[tauri::command]
pub(super) fn cancel_session(state: State<'_, Arc<AppState>>, id: String) {
    state.cancel(&id);
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Documentation {
    Installation,
    Uninstallation,
    Sso,
    Iam,
}

#[tauri::command]
pub(super) fn open_docs(app: AppHandle, topic: Documentation) -> Result<(), String> {
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
