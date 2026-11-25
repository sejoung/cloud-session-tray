use super::{catalog::Profiles, paths::Files};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
};

impl Files {
    pub(super) fn append_config(&self, current: &Profiles, entry: &str) -> Result<(), String> {
        let read_error = "AWS 설정 파일을 읽을 수 없습니다. 경로와 권한을 확인해 주세요.";
        let save_error =
            "AWS 설정 파일에 프로필을 추가할 수 없습니다. 폴더와 파일 권한을 확인해 주세요.";
        let changed = "다른 도구에서 AWS 파일을 수정했습니다. 프로필을 다시 읽고 가져와 주세요.";
        let path = match self.config.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => self.config.clone(),
            Err(_) => return Err(read_error.into()),
        };
        let (contents, digest) = match fs::read(&path) {
            Ok(contents) => {
                let digest = format!("{:x}", Sha256::digest(&contents));
                (contents, digest)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (Vec::new(), "missing".into()),
            Err(_) => return Err(read_error.into()),
        };
        if digest != current.config_digest {
            return Err(changed.into());
        }
        let directory = path.parent().ok_or(save_error)?;
        fs::create_dir_all(directory).map_err(|_| save_error)?;
        let mut staged = tempfile::NamedTempFile::new_in(directory).map_err(|_| save_error)?;
        if let Ok(metadata) = fs::metadata(&path) {
            let permissions = metadata.permissions();
            if permissions.readonly() {
                return Err(save_error.into());
            }
            staged
                .as_file()
                .set_permissions(permissions)
                .map_err(|_| save_error)?;
        }
        staged.write_all(&contents).map_err(|_| save_error)?;
        if !contents.is_empty() {
            staged.write_all(b"\n\n").map_err(|_| save_error)?;
        }
        staged.write_all(entry.as_bytes()).map_err(|_| save_error)?;
        staged.as_file().sync_all().map_err(|_| save_error)?;
        if self.read()?.revision() != current.revision() {
            return Err(changed.into());
        }
        staged.persist(path).map_err(|_| save_error)?;
        Ok(())
    }
}
