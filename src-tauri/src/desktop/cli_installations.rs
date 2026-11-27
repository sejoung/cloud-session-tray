use super::{localization::text, logging::operation};
use crate::state::AppState;
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[tauri::command]
pub(super) async fn set_terminal_path(
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), String> {
    operation(
        "AWS CLI terminal PATH configuration",
        state.set_terminal_path(enabled).await,
    )
}

#[tauri::command]
pub(super) async fn remove_cli(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    path: PathBuf,
) -> Result<bool, String> {
    let installation = state.removable_cli(&path)?;
    let language = state.language();
    let message = format!(
        "{}\n\nAWS CLI v{}\n{}",
        text(language, "native.removeCliBody"),
        installation.cli.version,
        path.display()
    );
    let mut dialog = app
        .dialog()
        .message(message)
        .title(text(language, "native.removeCliTitle"))
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            text(language, "common.remove").into(),
            text(language, "common.cancel").into(),
        ));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    let (send, receive) = tokio::sync::oneshot::channel();
    dialog.show(move |approved| {
        let _ = send.send(approved);
    });
    if !receive.await.map_err(|_| "errors.fileDialog")? {
        return Ok(false);
    }
    operation(
        "AWS CLI installation removal",
        state.remove_cli(&installation).await,
    )?;
    Ok(true)
}
