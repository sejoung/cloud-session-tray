pub(in crate::desktop) mod installer;
pub(in crate::desktop) mod release;

use crate::{
    desktop::{
        dialogs::{file_dialog, selected_file},
        localization::text,
        logging::operation,
    },
    state::AppState,
};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[tauri::command]
pub(in crate::desktop) async fn rescan_cli(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.rescan().await;
    Ok(())
}

#[tauri::command]
pub(in crate::desktop) async fn choose_cli(
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
pub(in crate::desktop) async fn reset_cli(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.reset_cli().await
}

#[tauri::command]
pub(in crate::desktop) async fn set_terminal_path(
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), String> {
    operation(
        "AWS CLI terminal PATH configuration",
        state.set_terminal_path(enabled).await,
    )
}

#[tauri::command]
pub(in crate::desktop) async fn remove_cli(
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
