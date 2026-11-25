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
            self.files
                .read()?
                .validate_registration(&registration.name)?;
            if self
                .sessions()
                .iter()
                .any(|session| session.login.profile() == registration.name)
            {
                return Err("이 앱에서 사용 중인 프로필입니다. 다른 이름을 지정해 주세요.".into());
            }
            let name = registration.name.clone();
            cli::configure_iam(&cli, &self.files, registration).await
                .map_err(|issue| match issue {
                    cli::Issue::Prompt => "AWS CLI의 프로필 입력 요청을 확인할 수 없습니다. CLI를 업데이트해 주세요.",
                    _ => issue.text(),
                }.to_string())?;
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
            return Err("작업 메모를 4000자 이내로 입력해 주세요.".into());
        }
        let profiles = self.files.read()?;
        profiles.validate(&login)?;
        let label = label.trim();
        if label.is_empty() || label.chars().count() > 64 || label.chars().any(char::is_control) {
            return Err("표시 이름을 1~64자로 입력해 주세요.".into());
        }
        let mut data = self.data();
        let existing = match &id {
            Some(id) => {
                if data.jobs.contains_key(id) {
                    return Err("진행 중인 작업이 끝나면 프로필을 수정할 수 있습니다.".into());
                }
                Some(
                    data.settings
                        .sessions
                        .iter()
                        .find(|session| &session.id == id)
                        .ok_or("저장된 프로필을 찾을 수 없습니다.")?,
                )
            }
            None => None,
        };
        if data.settings.sessions.iter().any(|session| {
            Some(&session.id) != id.as_ref() && session.login.profile() == login.profile()
        }) {
            return Err("이 AWS 프로필은 이미 세션 목록에 있습니다.".into());
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
            return Err("진행 중인 작업이 끝나면 프로필을 제거할 수 있습니다.".into());
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
            return Err("알림 시점을 만료 1~1440분 전으로 지정해 주세요.".into());
        }
        let mut data = self.data();
        self.update_settings(&mut data, |settings| settings.preferences = preferences)
    }
}
