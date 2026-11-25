use super::{
    checks::verify,
    core::{AppState, Job},
    renewals::{renew_mfa, renew_sso},
};
use crate::{
    cli::Cancel,
    model::{Login, Phase},
};
use std::sync::Arc;
use zeroize::Zeroizing;

impl AppState {
    pub fn start(self: &Arc<Self>, id: &str, action: Action) -> Result<(), String> {
        let cli = self.current_cli()?;
        let profiles = self.files.read()?;
        let mut data = self.data();
        if data.closing {
            return Err("앱을 종료하고 있습니다.".into());
        }
        let session = data
            .settings
            .sessions
            .iter()
            .find(|session| session.id == id)
            .cloned()
            .ok_or("저장된 프로필을 찾을 수 없습니다.")?;
        if let Some(job) = data.jobs.get(id) {
            if matches!(action, Action::Sso) && job.shared_sso {
                // A shared login already includes this profile's own STS check.
                return Ok(());
            }
            return Err("이 프로필에서 진행 중인 작업이 있습니다.".into());
        }
        let mut sessions = vec![session.clone()];
        let phase = match (&action, &session.login) {
            (Action::Check, _) => Phase::Checking,
            (Action::Mfa(otp), Login::IamMfa(_)) => {
                if !cli.mfa_supported {
                    return Err("IAM MFA 갱신을 지원하는 AWS CLI v2로 업데이트해 주세요.".into());
                }
                if otp.len() != 6 || !otp.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("6자리 OTP를 입력해 주세요.".into());
                }
                Phase::Queued
            }
            (Action::Sso, Login::Sso { profile }) => {
                profiles.validate(&session.login)?;
                let group = profiles
                    .sso_group(profile)
                    .ok_or("SSO 세션 설정을 확인해 주세요.")?;
                sessions.extend(
                    data.settings
                        .sessions
                        .iter()
                        .filter(|other| {
                            other.id != id
                                && matches!(other.login, Login::Sso { .. })
                                && profiles.sso_group(other.login.profile()).as_ref()
                                    == Some(&group)
                        })
                        .cloned(),
                );
                if sessions
                    .iter()
                    .any(|session| data.jobs.contains_key(&session.id))
                {
                    return Err(
                        "같은 SSO 세션을 사용하는 프로필에서 상태 확인이 진행 중입니다.".into(),
                    );
                }
                Phase::SigningIn
            }
            _ => return Err("AWS 프로필의 인증 방식을 확인해 주세요.".into()),
        };

        let cancel = Cancel::new();
        for session in &sessions {
            data.activity.remove(&session.id);
            data.jobs.insert(
                session.id.clone(),
                Job {
                    cancel: cancel.clone(),
                    phase,
                    shared_sso: matches!(action, Action::Sso),
                },
            );
        }
        drop(data);
        let state = Arc::clone(self);
        self.runtime.spawn(async move {
            match action {
                Action::Check => {
                    verify(&state, &cli, &session, &cancel).await;
                }
                Action::Mfa(otp) => renew_mfa(&state, &cli, &session, otp, &cancel).await,
                Action::Sso => renew_sso(&state, &cli, &sessions, &cancel).await,
            }
            let mut data = state.data();
            for session in sessions {
                data.jobs.remove(&session.id);
            }
        });
        Ok(())
    }

    pub fn cancel(&self, id: &str) {
        if let Some(job) = self.data().jobs.get(id) {
            job.cancel.cancel();
        }
    }

    pub fn begin_exit(&self) {
        let mut data = self.data();
        data.closing = true;
        for job in data.jobs.values() {
            job.cancel.cancel();
        }
    }

    pub fn has_jobs(&self) -> bool {
        !self.data().jobs.is_empty()
    }
}

pub enum Action {
    Check,
    Mfa(Zeroizing<String>),
    Sso,
}
