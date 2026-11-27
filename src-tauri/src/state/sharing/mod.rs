use super::core::AppState;
use crate::{
    model::{Login, Session, SessionDraft, SharedAuthentication, SharedProfile},
    profiles::{Kind, Profiles},
};

impl AppState {
    pub fn share_profile(&self, name: &str, task: String) -> Result<SharedProfile, String> {
        let profiles = self.files.read()?;
        let saved = self.sessions();
        let session = saved.iter().find(|session| session.login.profile() == name);
        let region = profiles.region(name).map(str::to_owned);
        let (profile, region, authentication) = match profiles.kind(name) {
            Kind::Sso => {
                let settings = profiles.sso_settings(name).ok_or("errors.ssoConfigRead")?;
                (name.into(), region, SharedAuthentication::Sso { settings })
            }
            Kind::IamSource => {
                let target = profiles
                    .linked_target(name)
                    .unwrap_or_else(|| source_target(name));
                let region = profiles.region(&target).map(str::to_owned).or(region);
                (
                    target,
                    region,
                    SharedAuthentication::IamMfa {
                        source_hint: Some(name.into()),
                        duration: None,
                    },
                )
            }
            Kind::IamSession | Kind::Unconfigured => {
                iam_template(&profiles, name, session, region)?
            }
            Kind::Unsupported => {
                return Err("errors.shareableProfileRequired".into());
            }
        };
        let shared = SharedProfile {
            version: 1,
            label: session.map_or_else(
                || name.chars().take(64).collect(),
                |session| session.label.clone(),
            ),
            profile,
            region,
            task,
            authentication,
        };
        shared.validate()?;
        Ok(shared)
    }

    pub async fn import_sso(&self, shared: SharedProfile) -> Result<String, String> {
        let _guard = self.aws_files.lock().await;
        let before = self.files.read()?;
        self.files.import_sso(&shared)?;
        let after = self.files.read()?;
        self.keep_expiries_for_unchanged_profiles(&before, &after)?;
        let id = self
            .sessions()
            .iter()
            .find(|session| session.login.profile() == shared.profile)
            .map(|session| session.id.clone());
        self.save_session(SessionDraft {
            id,
            label: shared.label,
            task: shared.task,
            login: Login::Sso {
                profile: shared.profile,
            },
        })
    }

    fn keep_expiries_for_unchanged_profiles(
        &self,
        before: &Profiles,
        after: &Profiles,
    ) -> Result<(), String> {
        if before.revision() == after.revision() || !before.same_credentials_revision(after) {
            return Ok(());
        }
        let mut data = self.data();
        if data.settings.expiries.is_empty() {
            return Ok(());
        }
        self.update_settings(&mut data, |settings| {
            for session in &settings.sessions {
                if let Some(expiry) = settings.expiries.get_mut(&session.id)
                    && expiry.revision == before.revision()
                    && expiry.fingerprint == after.fingerprint(&self.files, session.login.profile())
                {
                    expiry.revision = after.revision().into();
                }
            }
        })
    }
}

fn iam_template(
    profiles: &Profiles,
    name: &str,
    session: Option<&Session>,
    region: Option<String>,
) -> Result<(String, Option<String>, SharedAuthentication), String> {
    if let Some(Session {
        login: Login::IamMfa(login),
        ..
    }) = session
    {
        let region = region
            .or_else(|| login.region.clone())
            .or_else(|| profiles.region(&login.source).map(str::to_owned));
        return Ok((
            name.into(),
            region,
            SharedAuthentication::IamMfa {
                source_hint: Some(login.source.clone()),
                duration: login.duration,
            },
        ));
    }
    if profiles.kind(name) != Kind::IamSession {
        return Err("errors.authenticatedProfileRequired".into());
    }
    let source_hint = profiles.linked_source(name);
    Ok((
        name.into(),
        region,
        SharedAuthentication::IamMfa {
            source_hint,
            duration: None,
        },
    ))
}

fn source_target(source: &str) -> String {
    let short: String = source.chars().take(56).collect();
    format!("{short}-session")
}

#[cfg(test)]
mod tests;
