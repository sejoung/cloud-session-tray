use super::core::AppState;
use crate::{
    model::{Cli, Expiry, Preferences, SessionView, Status},
    profiles::Files,
};
use chrono::{Duration, Utc};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    cli: Option<Cli>,
    preferred_path: Option<PathBuf>,
    files: Files,
    pub(super) sessions: Vec<SessionView>,
    preferences: Preferences,
    scanning: bool,
    wizard: bool,
    profiles_error: Option<String>,
    settings_error: Option<String>,
}

impl AppState {
    pub fn dashboard(&self) -> Dashboard {
        let aws_idle = self.aws_files.try_lock().ok();
        let profiles = self.files.read();
        let revision = profiles.as_ref().ok().map(|profiles| profiles.revision());
        let mut data = self.data();
        if aws_idle.is_some()
            && data
                .settings
                .expiries
                .values()
                .any(|expiry| Some(expiry.revision.as_str()) != revision)
        {
            let _ = self.update_settings(&mut data, |settings| {
                settings
                    .expiries
                    .retain(|_, expiry| Some(expiry.revision.as_str()) == revision);
            });
        }
        let sessions = data
            .settings
            .sessions
            .iter()
            .map(|session| {
                let fingerprint = profiles
                    .as_ref()
                    .ok()
                    .map(|profiles| profiles.fingerprint(&self.files, session.login.profile()));
                let check = data.settings.checks.get(&session.id);
                let valid_check =
                    check.filter(|check| check.fingerprint == fingerprint && fingerprint.is_some());
                let expiry = data.settings.expiries.get(&session.id).filter(|expiry| {
                    fingerprint.as_ref() == Some(&expiry.fingerprint)
                        && revision == Some(expiry.revision.as_str())
                });
                let status = if expiry.is_some_and(|expiry| expiry.at <= Utc::now()) {
                    Status::NeedsRenewal
                } else {
                    valid_check.map_or(Status::Unknown, |check| check.status)
                };
                let job = data.jobs.get(&session.id);
                SessionView {
                    session: session.clone(),
                    status,
                    phase: job.map(|job| job.phase),
                    identity: valid_check.and_then(|check| check.identity.clone()),
                    checked_at: check.map(|check| check.checked_at),
                    expires_at: expiry.map(|expiry| expiry.at),
                    check_message: valid_check
                        .and_then(|check| check.message.clone())
                        .or_else(|| {
                            check
                                .filter(|check| {
                                    check.fingerprint.is_none() && fingerprint.is_none()
                                })
                                .and_then(|check| check.message.clone())
                        })
                        .or_else(|| {
                            if check.is_some() && valid_check.is_none() {
                                Some("errors.filesChanged".into())
                            } else {
                                None
                            }
                        }),
                    activity: data.activity.get(&session.id).cloned(),
                    can_cancel: job.is_some_and(|job| !job.cancel.requested()),
                }
            })
            .collect();
        Dashboard {
            cli: data.cli.clone(),
            preferred_path: data.settings.cli_path.clone(),
            files: self.files.clone(),
            sessions,
            preferences: data.settings.preferences,
            scanning: data.scanning,
            wizard: data.wizard,
            profiles_error: profiles.err().or_else(|| data.profiles_error.clone()),
            settings_error: data.settings_error.clone(),
        }
    }

    pub fn notifications(&self) -> Vec<(String, String, Expiry)> {
        let Ok(_guard) = self.aws_files.try_lock() else {
            return Vec::new();
        };
        let Ok(profiles) = self.files.read() else {
            return Vec::new();
        };
        let data = self.data();
        if !data.settings.preferences.notify {
            return Vec::new();
        }
        let now = Utc::now();
        let until = now + Duration::minutes(i64::from(data.settings.preferences.notify_minutes));
        data.settings
            .sessions
            .iter()
            .filter_map(|session| {
                let expiry = data.settings.expiries.get(&session.id)?;
                if expiry.notified
                    || expiry.at <= now
                    || expiry.at > until
                    || expiry.revision != profiles.revision()
                    || data.jobs.contains_key(&session.id)
                    || profiles.fingerprint(&self.files, session.login.profile())
                        != expiry.fingerprint
                {
                    return None;
                }
                Some((session.id.clone(), session.label.clone(), expiry.clone()))
            })
            .collect()
    }

    pub fn mark_notified(&self, id: &str, expected: &Expiry) {
        let mut data = self.data();
        let _ = self.update_settings(&mut data, |settings| {
            if let Some(expiry) = settings.expiries.get_mut(id)
                && expiry.fingerprint == expected.fingerprint
                && expiry.at == expected.at
            {
                expiry.notified = true;
            }
        });
    }
}
