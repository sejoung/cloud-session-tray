use super::{
    catalog::Profiles,
    ini::{self, Ini},
    paths::Files,
};

impl Files {
    pub fn read(&self) -> Result<Profiles, String> {
        let (config_file, config_digest) =
            ini::read_file(&self.config, "errors.configRead", "errors.configIni")?;
        let (credentials, credentials_digest) = ini::read_file(
            &self.credentials,
            "errors.credentialsRead",
            "errors.credentialsIni",
        )?;
        let mut config = Ini::new();
        let mut sso_sessions = Ini::new();
        for (name, section) in config_file {
            if name == "default" {
                config.insert(name, section);
                continue;
            }
            if name.starts_with("profile") || name.starts_with("sso-session") {
                let words = shell_words::split(&name).map_err(|_| "errors.configSections")?;
                if let [kind, name] = words.as_slice() {
                    let map = match kind.as_str() {
                        "profile" => &mut config,
                        "sso-session" => &mut sso_sessions,
                        _ => continue,
                    };
                    if map.insert(name.clone(), section).is_some() {
                        return Err("errors.duplicateConfigSections".into());
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
