use super::core::AppState;
use crate::{cli, model::IamRegistration, profiles::ProfileView};
use std::env;

impl AppState {
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

    pub async fn register_iam(
        &self,
        registration: IamRegistration,
    ) -> Result<Vec<ProfileView>, String> {
        registration.validate()?;
        let cli = self.current_cli()?;
        {
            let _guard = self.aws_files.lock().await;
            let name = registration.name.clone();
            let output_format = {
                let profiles = self.files.read()?;
                profiles.validate_registration(&name)?;
                if profiles.has_output_format(&name) {
                    ""
                } else {
                    "json"
                }
            };
            if self
                .sessions()
                .iter()
                .any(|session| session.login.profile() == registration.name)
            {
                return Err("errors.profileManaged".into());
            }
            cli::configure_iam(&cli, &self.files, registration, output_format)
                .await
                .map_err(|issue| {
                    match issue {
                        cli::Issue::Prompt => "errors.configurePrompt",
                        _ => issue.key(),
                    }
                    .to_string()
                })?;
            self.files.read()?.validate_iam_source(&name)?;
        }
        self.profiles().await
    }
}
