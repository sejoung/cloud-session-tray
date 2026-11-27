use crate::model::Settings;
use std::{fs, io, path::Path};

pub(in crate::state) fn load_settings(
    path: &Path,
    legacy_path: &Path,
) -> Result<Settings, &'static str> {
    let read_error = "errors.settingsRead";
    let (bytes, migrate) = match fs::read(path) {
        Ok(bytes) => (bytes, false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::read(legacy_path) {
            Ok(bytes) => (bytes, true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(_) => return Err(read_error),
        },
        Err(_) => return Err(read_error),
    };
    let settings = serde_json::from_slice(&bytes).map_err(|_| "errors.settingsFormat")?;
    if migrate {
        save_settings(path, &settings).map_err(|_| "errors.settingsMigration")?;
    }
    Ok(settings)
}

pub(in crate::state) fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
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
