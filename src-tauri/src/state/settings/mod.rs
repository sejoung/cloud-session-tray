pub(super) mod storage;

use super::core::AppState;
use crate::model::{Expiry, Preferences};
use chrono::{Duration, Utc};

impl AppState {
    pub fn save_preferences(&self, preferences: Preferences) -> Result<(), String> {
        if !(1..=1440).contains(&preferences.notify_minutes) {
            return Err("errors.invalidNotificationTime".into());
        }
        let mut data = self.data();
        self.update_settings(&mut data, |settings| settings.preferences = preferences)
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

#[cfg(test)]
mod tests;
