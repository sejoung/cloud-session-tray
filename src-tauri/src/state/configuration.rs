use super::core::AppState;
use crate::{
    cli,
    model::{IamRegistration, Preferences, Session, SessionDraft, valid_task},
    profiles::ProfileView,
};

impl AppState {
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
    pub fn save_session(&self, draft: SessionDraft) -> Result<String, String> {
        let SessionDraft {
            id,
            label,
            login,
            task,
        } = draft;
        if !valid_task(&task) {
            return Err("errors.taskLength".into());
        }
        let profiles = self.files.read()?;
        profiles.validate(&login)?;
        let label = label.trim();
        if label.is_empty() || label.chars().count() > 64 || label.chars().any(char::is_control) {
            return Err("errors.labelLength".into());
        }
        let mut data = self.data();
        let existing = match &id {
            Some(id) => {
                if data.jobs.contains_key(id) {
                    return Err("errors.editWhileBusy".into());
                }
                Some(
                    data.settings
                        .sessions
                        .iter()
                        .find(|session| &session.id == id)
                        .ok_or("errors.sessionNotFound")?,
                )
            }
            None => None,
        };
        if data.settings.sessions.iter().any(|session| {
            Some(&session.id) != id.as_ref() && session.login.profile() == login.profile()
        }) {
            return Err("errors.profileAlreadyAdded".into());
        }
        let changed = existing.is_some_and(|session| session.login != login);
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let session = Session {
            id: id.clone(),
            label: label.into(),
            task,
            login,
        };
        self.update_settings(&mut data, |settings| {
            if let Some(index) = settings
                .sessions
                .iter()
                .position(|current| current.id == id)
            {
                settings.sessions[index] = session;
            } else {
                settings.sessions.push(session);
            }
            if changed {
                settings.checks.remove(&id);
                settings.expiries.remove(&id);
            }
        })?;
        if changed {
            data.activity.remove(&id);
        }
        Ok(id)
    }

    pub fn remove_session(&self, id: &str) -> Result<(), String> {
        let mut data = self.data();
        if data.jobs.contains_key(id) {
            return Err("errors.removeWhileBusy".into());
        }
        self.update_settings(&mut data, |settings| {
            settings.sessions.retain(|session| session.id != id);
            settings.checks.remove(id);
            settings.expiries.remove(id);
        })?;
        data.activity.remove(id);
        Ok(())
    }

    pub fn save_preferences(&self, preferences: Preferences) -> Result<(), String> {
        if !(1..=1440).contains(&preferences.notify_minutes) {
            return Err("errors.invalidNotificationTime".into());
        }
        let mut data = self.data();
        self.update_settings(&mut data, |settings| settings.preferences = preferences)
    }
}
