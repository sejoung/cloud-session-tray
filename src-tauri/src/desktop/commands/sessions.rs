use crate::{
    desktop::{logging::operation, tray::refresh_tray},
    model::{Login, SessionDraft},
    state::{Action, AppState},
};
use std::sync::Arc;
use tauri::{AppHandle, State};
use zeroize::Zeroizing;

#[tauri::command]
pub(in crate::desktop) async fn save_session(
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
pub(in crate::desktop) async fn remove_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    operation("Session removal", state.remove_session(&id))?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
pub(in crate::desktop) async fn check_session(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    operation("Session status request", state.start(&id, Action::Check))
}

#[tauri::command]
pub(in crate::desktop) async fn renew_mfa(
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
pub(in crate::desktop) async fn renew_sso(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    operation(
        "Identity Center sign-in request",
        state.start(&id, Action::Sso),
    )
}

#[tauri::command]
pub(in crate::desktop) fn cancel_session(state: State<'_, Arc<AppState>>, id: String) {
    state.cancel(&id);
}
