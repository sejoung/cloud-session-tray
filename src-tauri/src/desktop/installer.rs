use crate::state::AppState;
use std::{sync::Arc, time::Duration};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncWriteExt;

const INSTALLER_LIMIT: u64 = 256 * 1024 * 1024;

#[tauri::command]
pub(super) async fn install_cli(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let _guard = state
        .installer
        .try_lock()
        .map_err(|_| "설치 파일을 이미 다운로드하고 있습니다.")?;
    let (filename, magic): (&str, &[u8]) = if cfg!(target_os = "macos") {
        ("AWSCLIV2.pkg", b"xar!")
    } else if cfg!(windows) {
        ("AWSCLIV2-User.msi", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1")
    } else {
        return Err("자동 설치는 macOS와 Windows에서 지원됩니다.".into());
    };
    let failure =
        "AWS 공식 설치 파일을 다운로드할 수 없습니다. 연결을 확인하고 다시 시도해 주세요.";
    let invalid_file = "AWS에서 받은 설치 파일을 확인할 수 없습니다.";
    let storage_error = "설치 파일을 저장할 수 없습니다. 디스크 공간과 폴더 권한을 확인해 주세요.";
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| storage_error)?
        .join("installers");
    tokio::fs::create_dir_all(&cache)
        .await
        .map_err(|_| storage_error)?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(240))
        .build()
        .map_err(|_| failure)?;
    let mut response = client
        .get(format!("https://awscli.amazonaws.com/{filename}"))
        .send()
        .await
        .map_err(|_| failure)?;
    if !response.status().is_success() {
        return Err(failure.into());
    }
    let length = response.content_length();
    if length.is_some_and(|length| length > INSTALLER_LIMIT) {
        return Err(invalid_file.into());
    }
    let temp = tempfile::NamedTempFile::new_in(&cache).map_err(|_| storage_error)?;
    let mut file = tokio::fs::File::from_std(temp.reopen().map_err(|_| storage_error)?);
    let mut header = Vec::with_capacity(magic.len());
    let mut size = 0_u64;
    while let Some(bytes) = response.chunk().await.map_err(|_| failure)? {
        size += bytes.len() as u64;
        if size > INSTALLER_LIMIT {
            return Err(invalid_file.into());
        }
        if header.len() < magic.len() {
            header.extend_from_slice(&bytes[..bytes.len().min(magic.len() - header.len())]);
        }
        file.write_all(&bytes).await.map_err(|_| storage_error)?;
    }
    if header != magic || length.is_some_and(|length| size != length) {
        return Err(invalid_file.into());
    }
    file.sync_all().await.map_err(|_| storage_error)?;
    drop(file);
    let path = cache.join(filename);
    temp.persist(&path).map_err(|_| storage_error)?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|_| {
            "OS 설치 창을 열 수 없습니다. 기존 실행 파일을 선택하거나 공식 문서에서 설치해 주세요."
                .into()
        })
}
