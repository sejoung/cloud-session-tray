use super::{checks::verify, core::AppState, renewal_region::prepare_region};
use crate::{
    cli::{self, Cancel, Issue},
    model::{Cli, Expiry, Login, Phase, Session, Status},
    profiles::Kind,
};
use zeroize::Zeroizing;

pub(super) async fn renew_mfa(
    state: &AppState,
    cli: &Cli,
    session: &Session,
    otp: Zeroizing<String>,
    cancel: &Cancel,
) {
    let Login::IamMfa(login) = &session.login else {
        return;
    };
    let target = &login.target;
    let guard = tokio::select! {
        _ = cancel.wait() => {
            state.activity(&session.id, Issue::Cancelled.text().into());
            return;
        }
        guard = state.aws_files.lock() => guard,
    };
    let before = match (|| {
        let profiles = state.files.read()?;
        profiles.validate(&session.login)?;
        profiles.ensure_history_disabled(&login.source)?;
        Ok::<_, String>(profiles)
    })() {
        Ok(profiles) => profiles,
        Err(error) => {
            state.activity(&session.id, error);
            return;
        }
    };
    state.phase(&session.id, Phase::SigningIn);
    let result = cli::mfa(cli, &state.files, login, otp, cancel).await;
    let written = state.files.read().ok();
    let region_result = match written.as_ref() {
        Some(written) if result.is_ok() => prepare_region(state, cli, login, written, cancel).await,
        _ => Ok(false),
    };
    let after = if region_result == Ok(false) {
        written
    } else {
        state.files.read().ok()
    };

    let fingerprint = after
        .as_ref()
        .map(|profiles| profiles.fingerprint(&state.files, target));
    let target_changed = fingerprint.as_ref() != Some(&before.fingerprint(&state.files, target));
    let revision = after.as_ref().map(|profiles| profiles.revision());
    let stored = after.as_ref().is_some_and(|profiles| {
        profiles.kind(target) == Kind::IamSession && profiles.target_allowed(target)
    });
    let expiry = match (result, fingerprint, revision) {
        (Ok(Some(at)), Some(fingerprint), Some(revision)) if target_changed && stored => {
            Some(Expiry {
                at,
                fingerprint,
                revision: revision.into(),
                notified: false,
            })
        }
        _ => None,
    };
    if revision != Some(before.revision()) || result.is_ok() {
        let mut data = state.data();
        if let Err(error) = state.update_settings(&mut data, |settings| {
            settings.expiries.remove(&session.id);
            if let Some(after) = after.as_ref().filter(|after| {
                before.changed_only_target(after, target)
                    || (region_result == Ok(true) && before.can_rebind_region_change(after, target))
            }) {
                // An app-owned write may advance the file revision for other managed profiles.
                settings.expiries.retain(|_, expiry| {
                    if expiry.revision == before.revision() {
                        expiry.revision = after.revision().into();
                        true
                    } else {
                        false
                    }
                });
            } else {
                settings.expiries.clear();
            }
            if let Some(expiry) = expiry {
                settings.expiries.insert(session.id.clone(), expiry);
            }
        }) {
            data.activity.insert(session.id.clone(), error);
            return;
        }
    }
    drop(guard);

    if !state.data().closing {
        state.phase(&session.id, Phase::Checking);
        let verification_cancel = if cancel.requested() {
            Cancel::new()
        } else {
            cancel.clone()
        };
        let status = verify(state, cli, session, &verification_cancel).await;
        let message = match result {
            Ok(_) if stored && status == Some(Status::Available) => "IAM MFA 세션을 갱신했습니다.",
            Ok(_) if stored => "IAM MFA 세션을 발급했습니다. STS 신원을 확인하지 못했습니다.",
            Ok(_) => "MFA 발급은 완료됐지만 대상 프로필의 자격 증명을 확인할 수 없습니다.",
            Err(issue) => issue.text(),
        };
        let activity = if region_result.is_err() {
            format!(
                "{message} 대상 프로필에 리전을 기록할 수 없습니다. 도구 연결에서 리전을 확인해 주세요."
            )
        } else {
            message.into()
        };
        state.activity(&session.id, activity);
    }
}

pub(super) async fn renew_sso(state: &AppState, cli: &Cli, sessions: &[Session], cancel: &Cancel) {
    let source = &sessions[0];
    let profile = source.login.profile();
    let preflight = state.files.read().and_then(|profiles| {
        profiles.validate(&source.login)?;
        profiles.ensure_history_disabled(profile)
    });
    if let Err(error) = preflight {
        for session in sessions {
            state.activity(&session.id, error.clone());
        }
        return;
    }
    match cli::sso(cli, &state.files, profile, cancel).await {
        Ok(()) => {
            for session in sessions {
                state.phase(&session.id, Phase::Checking);
            }
            for session in sessions {
                let status = verify(state, cli, session, cancel).await;
                let message = if status == Some(Status::Available) {
                    "Identity Center에 로그인했습니다."
                } else {
                    "Identity Center 로그인은 완료됐습니다. STS 신원을 확인하지 못했습니다."
                };
                state.activity(&session.id, message.into());
            }
        }
        Err(issue) => {
            for session in sessions {
                state.activity(&session.id, issue.text().into());
            }
        }
    }
}
