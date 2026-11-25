use serde::Serialize;
use std::{
    env, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Files {
    pub config: PathBuf,
    pub credentials: PathBuf,
}

impl Files {
    pub fn new(home: &Path) -> io::Result<Self> {
        Ok(Self {
            config: profile_path("AWS_CONFIG_FILE", home, "config")?,
            credentials: profile_path("AWS_SHARED_CREDENTIALS_FILE", home, "credentials")?,
        })
    }
}

fn profile_path(key: &str, home: &Path, default: &str) -> io::Result<PathBuf> {
    let Some(path) = env::var_os(key).filter(|value| !value.is_empty()) else {
        return Ok(home.join(".aws").join(default));
    };
    let path = PathBuf::from(path);
    if let Ok(relative) = path.strip_prefix("~") {
        return Ok(home.join(relative));
    }
    std::path::absolute(path)
}
