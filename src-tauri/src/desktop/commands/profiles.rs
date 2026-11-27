use crate::{
    desktop::logging::operation, model::IamRegistration, profiles::ProfileView, state::AppState,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(in crate::desktop) async fn profiles(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProfileView>, String> {
    operation("AWS profile listing", state.profiles().await)
}

#[tauri::command]
pub(in crate::desktop) async fn mfa_devices(
    state: State<'_, Arc<AppState>>,
    source: String,
) -> Result<Vec<String>, String> {
    operation("MFA device lookup", state.mfa_devices(&source).await)
}

#[tauri::command]
pub(in crate::desktop) async fn register_iam(
    state: State<'_, Arc<AppState>>,
    registration: IamRegistration,
) -> Result<Vec<ProfileView>, String> {
    operation(
        "IAM profile registration",
        state.register_iam(registration).await,
    )
}

#[tauri::command]
pub(in crate::desktop) async fn configure_sso(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProfileView>, String> {
    operation("Identity Center profile setup", state.wizard().await)
}
