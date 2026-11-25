use super::{
    catalog::Profiles,
    ini::{self, Ini},
    paths::Files,
};

impl Files {
    pub fn read(&self) -> Result<Profiles, String> {
        let (config_file, config_digest) = ini::read_file(&self.config, "AWS 설정 파일")?;
        let (credentials, credentials_digest) =
            ini::read_file(&self.credentials, "AWS 자격 증명 파일")?;
        let mut config = Ini::new();
        let mut sso_sessions = Ini::new();
        for (name, section) in config_file {
            if name == "default" {
                config.insert(name, section);
                continue;
            }
            if name.starts_with("profile") || name.starts_with("sso-session") {
                let words = shell_words::split(&name)
                    .map_err(|_| "AWS 설정 파일의 섹션 이름을 확인해 주세요.")?;
                if let [kind, name] = words.as_slice() {
                    let map = match kind.as_str() {
                        "profile" => &mut config,
                        "sso-session" => &mut sso_sessions,
                        _ => continue,
                    };
                    if map.insert(name.clone(), section).is_some() {
                        return Err(
                            "AWS 설정 파일에 이름이 겹치는 프로필 또는 SSO 세션이 있습니다.".into(),
                        );
                    }
                }
            }
        }
        Ok(Profiles {
            config,
            credentials,
            sso_sessions,
            revision: format!("{config_digest}/{credentials_digest}"),
            config_digest,
        })
    }
}
