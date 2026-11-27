use std::path::PathBuf;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

pub(super) fn file_dialog(app: &AppHandle, title: &str) -> FileDialogBuilder<Wry> {
    let dialog = app.dialog().file().set_title(title);
    match app.get_webview_window("main") {
        Some(window) => dialog.set_parent(&window),
        None => dialog,
    }
}

pub(super) async fn selected_file(
    dialog: FileDialogBuilder<Wry>,
    save: bool,
) -> Result<Option<PathBuf>, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let selected = move |path| {
        let _ = send.send(path);
    };
    if save {
        dialog.save_file(selected);
    } else {
        dialog.pick_file(selected);
    }
    let path = receive.await.map_err(|_| "errors.fileDialog")?;
    path.map(|file| {
        file.into_path()
            .map_err(|_| "errors.localFileRequired".into())
    })
    .transpose()
}
