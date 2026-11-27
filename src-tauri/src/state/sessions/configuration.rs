use crate::{
    model::{Session, SessionDraft, valid_task},
    state::core::AppState,
};

impl AppState {
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
}
