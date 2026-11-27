use super::core::AppState;
use crate::{
    cli::{self, Cancel, Issue},
    model::{Check, Cli, Login, Session, Status},
    profiles::Kind,
};
use chrono::Utc;

pub(super) async fn verify(
    state: &AppState,
    cli: &Cli,
    session: &Session,
    cancel: &Cancel,
) -> Option<Status> {
    let profile = session.login.profile();
    let loaded = state.files.read();
    let fingerprint = loaded
        .as_ref()
        .ok()
        .map(|profiles| profiles.fingerprint(&state.files, profile));
    let result = async {
        let profiles = loaded.map_err(|message| (Status::Unknown, message))?;
        profiles
            .ensure_history_disabled(profile)
            .map_err(|error| (Status::Unknown, error))?;
        match (&session.login, profiles.kind(profile)) {
            (Login::IamMfa(_), Kind::IamSession) if profiles.target_allowed(profile) => {}
            (Login::Sso { .. }, Kind::Sso) => {}
            (Login::IamMfa(_), Kind::Unconfigured) => {
                return Err((Status::NeedsRenewal, "errors.noTemporaryCredentials".into()));
            }
            _ => {
                return Err((Status::NeedsRenewal, "errors.invalidAuthentication".into()));
            }
        }
        cli::identity(cli, &state.files, profile, cancel)
            .await
            .map_err(|issue| (issue.status(), issue.key().into()))
    }
    .await;
    if cancel.requested() {
        state.activity(&session.id, Issue::Cancelled.key().into());
        return None;
    }
    let current = state
        .files
        .read()
        .ok()
        .map(|profiles| profiles.fingerprint(&state.files, profile));
    let (status, identity, message) = if current != fingerprint {
        (
            Status::Unknown,
            None,
            Some("errors.filesChangedDuringCheck".into()),
        )
    } else {
        match result {
            Ok(identity) => (Status::Available, Some(identity), None),
            Err((status, message)) => (status, None, Some(message)),
        }
    };
    let mut data = state.data();
    if let Some(code) = &message {
        log::warn!("AWS identity check did not verify credentials: code={code}");
    } else {
        log::info!("AWS identity verified");
    }
    let check = Check {
        status,
        checked_at: Utc::now(),
        identity,
        message,
        fingerprint: current,
    };
    if let Err(error) = state.update_settings(&mut data, |settings| {
        if settings
            .expiries
            .get(&session.id)
            .is_some_and(|expiry| check.fingerprint.as_ref() != Some(&expiry.fingerprint))
        {
            settings.expiries.remove(&session.id);
        }
        settings.checks.insert(session.id.clone(), check);
    }) {
        log::warn!("Could not persist the AWS identity check: code={error}");
        data.activity.insert(session.id.clone(), vec![error]);
        return None;
    }
    Some(status)
}
