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
            state.activity(&session.id, Issue::Cancelled.key().into());
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
            log::warn!("IAM MFA renewal preflight failed: code={error}");
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
            log::warn!("Could not persist the IAM MFA renewal: code={error}");
            data.activity.insert(session.id.clone(), vec![error]);
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
        let messages = mfa_activity(result.map(|_| ()), stored, status, region_result.is_err());
        state.data().activity.insert(session.id.clone(), messages);
    }
}

fn mfa_activity(
    renewal: Result<(), Issue>,
    stored: bool,
    status: Option<Status>,
    region_error: bool,
) -> Vec<String> {
    let message = match renewal {
        Ok(()) if stored && status == Some(Status::Available) => {
            log::info!("IAM MFA session renewed");
            "messages.mfaRenewed"
        }
        Ok(()) if stored => {
            log::warn!("IAM MFA credentials issued without verified STS identity");
            "messages.mfaIdentityUnverified"
        }
        Ok(()) => {
            log::warn!("IAM MFA destination credentials could not be verified");
            "messages.mfaCredentialsUnverified"
        }
        Err(issue) => {
            log::warn!("IAM MFA renewal failed: code={}", issue.key());
            issue.key()
        }
    };
    let mut messages = vec![message.into()];
    if region_error {
        log::warn!("Could not write the destination region after IAM MFA renewal");
        messages.push("errors.targetRegionWrite".into());
    }
    messages
}

pub(super) async fn renew_sso(state: &AppState, cli: &Cli, sessions: &[Session], cancel: &Cancel) {
    let source = &sessions[0];
    let profile = source.login.profile();
    let preflight = state.files.read().and_then(|profiles| {
        profiles.validate(&source.login)?;
        profiles.ensure_history_disabled(profile)
    });
    if let Err(error) = preflight {
        log::warn!("Identity Center sign-in preflight failed: code={error}");
        for session in sessions {
            state.activity(&session.id, error.clone());
        }
        return;
    }
    match cli::sso(cli, &state.files, profile, cancel).await {
        Ok(()) => {
            log::info!("Identity Center sign-in completed");
            for session in sessions {
                state.phase(&session.id, Phase::Checking);
            }
            for session in sessions {
                let status = verify(state, cli, session, cancel).await;
                let message = if status == Some(Status::Available) {
                    "messages.ssoSignedIn"
                } else {
                    "messages.ssoIdentityUnverified"
                };
                state.activity(&session.id, message.into());
            }
        }
        Err(issue) => {
            log::warn!("Identity Center sign-in failed: code={}", issue.key());
            for session in sessions {
                state.activity(&session.id, issue.key().into());
            }
        }
    }
}
