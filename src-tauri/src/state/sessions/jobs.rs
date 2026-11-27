use super::{
    checks::verify,
    renewals::{renew_mfa, renew_sso},
};
use crate::{
    cli::Cancel,
    model::{Login, Phase},
    state::core::{AppState, Job},
};
use std::sync::Arc;
use zeroize::Zeroizing;

impl AppState {
    pub fn start(self: &Arc<Self>, id: &str, action: Action) -> Result<(), String> {
        let cli = self.current_cli()?;
        let profiles = self.files.read()?;
        let mut data = self.data();
        if data.closing {
            return Err("errors.appClosing".into());
        }
        let session = data
            .settings
            .sessions
            .iter()
            .find(|session| session.id == id)
            .cloned()
            .ok_or("errors.sessionNotFound")?;
        if let Some(job) = data.jobs.get(id) {
            if matches!(action, Action::Sso) && job.shared_sso {
                // A shared login already includes this profile's own STS check.
                return Ok(());
            }
            return Err("errors.sessionBusy".into());
        }
        let mut sessions = vec![session.clone()];
        let phase = match (&action, &session.login) {
            (Action::Check, _) => Phase::Checking,
            (Action::Mfa(otp), Login::IamMfa(_)) => {
                if !cli.mfa_supported {
                    return Err("errors.mfaCliRequired".into());
                }
                if otp.len() != 6 || !otp.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("errors.invalidOtp".into());
                }
                Phase::Queued
            }
            (Action::Sso, Login::Sso { profile }) => {
                profiles.validate(&session.login)?;
                let group = profiles
                    .sso_group(profile)
                    .ok_or("errors.ssoSessionConfig")?;
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
                    return Err("errors.ssoGroupBusy".into());
                }
                Phase::SigningIn
            }
            _ => return Err("errors.authenticationMethod".into()),
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
        let operation = action.name();
        log::info!(
            "Session operation started: operation={operation}, sessions={}",
            sessions.len()
        );
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
            log::info!(
                "Session operation finished: operation={operation}, canceled={}",
                cancel.requested()
            );
        });
        Ok(())
    }

    pub fn cancel(&self, id: &str) {
        if let Some(job) = self.data().jobs.get(id) {
            log::info!("Session cancellation requested");
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

impl Action {
    fn name(&self) -> &'static str {
        match self {
            Self::Check => "check_identity",
            Self::Mfa(_) => "renew_mfa",
            Self::Sso => "sign_in_sso",
        }
    }
}
