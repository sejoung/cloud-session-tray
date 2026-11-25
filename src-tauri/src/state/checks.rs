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
                return Err((
                    Status::NeedsRenewal,
                    "아직 대상 프로필에 임시 자격 증명이 없습니다.".into(),
                ));
            }
            _ => {
                return Err((
                    Status::NeedsRenewal,
                    "AWS 프로필의 인증 설정을 확인해 주세요.".into(),
                ));
            }
        }
        cli::identity(cli, &state.files, profile, cancel)
            .await
            .map_err(|issue| (issue.status(), issue.text().into()))
    }
    .await;
    if cancel.requested() {
        state.activity(&session.id, Issue::Cancelled.text().into());
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
            Some("확인 중 AWS 프로필 파일이 변경되었습니다. 다시 확인해 주세요.".into()),
        )
    } else {
        match result {
            Ok(identity) => (Status::Available, Some(identity), None),
            Err((status, message)) => (status, None, Some(message)),
        }
    };
    let mut data = state.data();
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
        data.activity.insert(session.id.clone(), error);
        return None;
    }
    Some(status)
}
