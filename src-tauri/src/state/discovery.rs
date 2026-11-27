use super::core::AppState;
use crate::{
    cli::{self, Installation},
    model::Cli,
    profiles::ProfileView,
};
use std::{
    env,
    path::{Path, PathBuf},
};

impl AppState {
    pub async fn rescan(&self) -> Option<Cli> {
        let _guard = self.discovery.lock().await;
        self.scan_cli().await
    }

    async fn scan_cli(&self) -> Option<Cli> {
        let preferred = {
            let mut data = self.data();
            data.scanning = true;
            data.settings.cli_path.clone()
        };
        let installations =
            cli::discover(preferred.as_deref(), &self.home, &self.terminal_alias()).await;
        let found = installations
            .first()
            .map(|installation| installation.cli.clone());
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
        data.installations = installations;
        self.sync_terminal_path(&mut data);
        found
    }

    pub async fn choose_cli(&self, path: PathBuf) -> Result<(), String> {
        let _guard = self.discovery.lock().await;
        self.set_cli(path).await
    }

    pub async fn choose_detected_cli(&self, path: PathBuf) -> Result<(), String> {
        let _guard = self.discovery.lock().await;
        if !self
            .data()
            .installations
            .iter()
            .any(|entry| entry.cli.path == path)
        {
            return Err("errors.cliInstallationChanged".into());
        }
        self.set_cli(path).await
    }

    async fn set_cli(&self, path: PathBuf) -> Result<(), String> {
        if path == self.terminal_alias() {
            return Err("errors.invalidCli".into());
        }
        let found = cli::inspect(&path).await.ok_or("errors.invalidCli")?;
        let installation = Installation::detect(found.clone(), &self.home);
        let mut data = self.data();
        self.update_settings(&mut data, |settings| settings.cli_path = Some(path))?;
        data.installations
            .retain(|entry| !entry.same_executable(&installation));
        data.installations.insert(0, installation);
        data.cli = Some(found);
        data.profiles_error = None;
        self.sync_terminal_path(&mut data);
        Ok(())
    }

    pub async fn reset_cli(&self) -> Result<(), String> {
        let _guard = self.discovery.lock().await;
        {
            let mut data = self.data();
            self.update_settings(&mut data, |settings| settings.cli_path = None)?;
        }
        self.scan_cli().await;
        Ok(())
    }

    pub fn removable_cli(&self, path: &Path) -> Result<Installation, String> {
        let data = self.data();
        if !data.jobs.is_empty() || data.wizard {
            return Err("errors.cliUninstallBusy".into());
        }
        let installation = data
            .installations
            .iter()
            .find(|entry| entry.cli.path == path)
            .ok_or("errors.cliInstallationChanged")?;
        let active = data
            .installations
            .first()
            .filter(|active| active.cli.path.is_file())
            .ok_or("errors.cliUseAnother")?;
        let active = Installation::detect(active.cli.clone(), &self.home);
        if !installation.can_remove_while_using(&active) {
            return Err("errors.cliUseAnother".into());
        }
        if installation.contains(&self.files.config)
            || installation.contains(&self.files.credentials)
        {
            return Err("errors.cliUninstallAwsFiles".into());
        }
        Ok(installation.clone())
    }

    pub async fn remove_cli(&self, approved: &Installation) -> Result<(), String> {
        let _guard = self.discovery.lock().await;
        let path = &approved.cli.path;
        let current = cli::inspect(path)
            .await
            .map(|cli| Installation::detect(cli, &self.home))
            .ok_or("errors.cliInstallationChanged")?;
        if !approved.matches(&current) || !self.removable_cli(path)?.matches(&current) {
            return Err("errors.cliInstallationChanged".into());
        }
        let result = current.uninstall(&self.home).await;
        self.scan_cli().await;
        result
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
