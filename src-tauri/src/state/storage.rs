use crate::model::Settings;
use std::{fs, io, path::Path};

pub(super) fn load_settings(path: &Path, legacy_path: &Path) -> Result<Settings, &'static str> {
    let read_error = "앱 설정 파일을 읽을 수 없습니다.";
    let (bytes, migrate) = match fs::read(path) {
        Ok(bytes) => (bytes, false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::read(legacy_path) {
            Ok(bytes) => (bytes, true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(_) => return Err(read_error),
        },
        Err(_) => return Err(read_error),
    };
    let settings =
        serde_json::from_slice(&bytes).map_err(|_| "앱 설정 파일의 형식을 읽을 수 없습니다.")?;
    if migrate {
        save_settings(path, &settings)
            .map_err(|_| "이전 앱 설정을 새 위치에 저장할 수 없습니다.")?;
    }
    Ok(settings)
}

pub(super) fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::other("missing settings directory"))?;
    fs::create_dir_all(directory)?;
    let file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(file.as_file(), settings)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
