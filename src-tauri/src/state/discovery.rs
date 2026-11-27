use super::core::AppState;
use crate::{cli, model::Cli, profiles::ProfileView};
use std::{env, path::PathBuf};

impl AppState {
    pub async fn rescan(&self) -> Option<Cli> {
        let _guard = self.discovery.lock().await;
        let preferred = {
            let mut data = self.data();
            data.scanning = true;
            data.settings.cli_path.clone()
        };
        let found = cli::discover(preferred.as_deref(), &self.home).await;
        if let Some(cli) = &found {
            log::info!(
                "AWS CLI detected: version={}, mfa_supported={}",
                cli.version,
                cli.mfa_supported
            );
        } else {
            log::info!("AWS CLI discovery completed without an executable");
        }
        let mut data = self.data();
        data.scanning = false;
        data.profiles_error = None;
        data.cli = found.clone();
        found
    }

    pub async fn choose_cli(&self, path: PathBuf) -> Result<(), String> {
        let _guard = self.discovery.lock().await;
        let found = cli::inspect(&path).await.ok_or("errors.invalidCli")?;
        let mut data = self.data();
        self.update_settings(&mut data, |settings| settings.cli_path = Some(path))?;
        data.cli = Some(found);
        data.profiles_error = None;
        Ok(())
    }

    pub async fn reset_cli(&self) -> Result<(), String> {
        {
            let mut data = self.data();
            self.update_settings(&mut data, |settings| settings.cli_path = None)?;
        }
        self.rescan().await;
        Ok(())
    }

    pub async fn profiles(&self) -> Result<Vec<ProfileView>, String> {
        let cli = self.current_cli()?;
        let result = async {
            let names = cli::list(&cli, &self.files)
                .await
                .map_err(|issue| issue.key().to_string())?;
            let profiles = self.files.read()?;
            let preferred = ["AWS_DEFAULT_PROFILE", "AWS_PROFILE"]
                .iter()
                .find_map(|key| env::var(key).ok().filter(|name| !name.is_empty()))
                .unwrap_or_else(|| "default".into());
            Ok(names
                .into_iter()
                .map(|name| profiles.public(name, &preferred))
                .collect())
        }
        .await;
        self.data().profiles_error = result.as_ref().err().cloned();
        result
    }

    pub async fn mfa_devices(&self, source: &str) -> Result<Vec<String>, String> {
        let cli = self.current_cli()?;
        let profiles = self.files.read()?;
        profiles.validate_iam_source(source)?;
        profiles.ensure_history_disabled(source)?;
        cli::mfa_devices(&cli, &self.files, source)
            .await
            .map_err(|issue| {
                if issue == cli::Issue::Denied {
                    "errors.mfaListPermission".into()
                } else {
                    issue.key().into()
                }
            })
    }

    pub async fn wizard(&self) -> Result<Vec<ProfileView>, String> {
        let cli = self.current_cli()?;
        {
            let mut data = self.data();
            if data.wizard || data.closing {
                return Err("errors.ssoWizardBusy".into());
            }
            data.wizard = true;
        }
        let result = async {
            let _guard = self.aws_files.lock().await;
            self.files.read()?.ensure_history_disabled("default")?;
            cli::configure_sso(&cli, &self.files).await
        }
        .await;
        self.data().wizard = false;
        // Failed or interrupted configuration may still have saved part of a profile.
        let profiles = self.profiles().await;
        result.and(profiles)
    }
}
