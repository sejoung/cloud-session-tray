use super::{
    catalog::{Kind, Profiles, validate_name},
    ini::Section,
    paths::Files,
};
use crate::model::{SharedAuthentication, SharedProfile, SsoSession, SsoSettings};
use std::fmt::Write as _;

impl Profiles {
    pub fn sso_settings(&self, name: &str) -> Option<SsoSettings> {
        if self.kind(name) != Kind::Sso {
            return None;
        }
        let profile = self.config.get(name)?;
        let session = profile.get("sso_session");
        let settings = session
            .and_then(|name| self.sso_sessions.get(name))
            .unwrap_or(profile);
        Some(SsoSettings {
            start_url: settings.get("sso_start_url")?.clone(),
            sso_region: settings.get("sso_region")?.clone(),
            account_id: profile.get("sso_account_id")?.clone(),
            role_name: profile.get("sso_role_name")?.clone(),
            session: session.map(|name| SsoSession {
                name: name.clone(),
                scopes: settings
                    .get("sso_registration_scopes")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            }),
        })
    }

    pub fn name_exists(&self, profile: &str) -> bool {
        self.config.contains_key(profile) || self.credentials.contains_key(profile)
    }
}

impl Files {
    pub fn import_sso(&self, profile: &SharedProfile) -> Result<(), String> {
        profile.validate()?;
        validate_name(&profile.profile)?;
        let SharedAuthentication::Sso { settings } = &profile.authentication else {
            return Err("errors.ssoShareRequired".into());
        };
        let local = self.read()?;
        if local.name_exists(&profile.profile) {
            if local.matches_sso(profile, settings) {
                return Ok(());
            }
            return Err("errors.sharedProfileNameConflict".into());
        }
        let entry = sso_entry(&local, profile, settings);
        self.append_config(&local, &entry)
    }
}

impl Profiles {
    fn matches_sso(&self, profile: &SharedProfile, settings: &SsoSettings) -> bool {
        let Some(local) = self.sso_settings(&profile.profile) else {
            return false;
        };
        local.start_url == settings.start_url
            && local.sso_region == settings.sso_region
            && local.account_id == settings.account_id
            && local.role_name == settings.role_name
            && session_scopes(local.session.as_ref()) == session_scopes(settings.session.as_ref())
            && local.session.is_some() == settings.session.is_some()
            && self.region(&profile.profile) == profile.region.as_deref()
    }
}

fn session_scopes(session: Option<&SsoSession>) -> Option<&str> {
    session.and_then(|session| session.scopes.as_deref())
}

fn sso_entry(local: &Profiles, profile: &SharedProfile, settings: &SsoSettings) -> String {
    let mut entry = String::new();
    if profile.profile == "default" {
        entry.push_str("[default]\n");
    } else {
        let _ = writeln!(entry, "[profile {}]", shell_words::quote(&profile.profile));
    }
    let _ = writeln!(entry, "sso_account_id = {}", settings.account_id);
    let _ = writeln!(entry, "sso_role_name = {}", settings.role_name);
    if let Some(region) = &profile.region {
        let _ = writeln!(entry, "region = {region}");
    }
    if let Some(session) = &settings.session {
        let (name, existing) = choose_session(local, settings, session);
        let _ = writeln!(entry, "sso_session = {name}");
        if !existing {
            let _ = writeln!(entry, "\n[sso-session {}]", shell_words::quote(&name));
            append_sso_settings(&mut entry, settings);
            if let Some(scopes) = &session.scopes {
                let _ = writeln!(entry, "sso_registration_scopes = {scopes}");
            }
        }
    } else {
        append_sso_settings(&mut entry, settings);
    }
    entry
}

fn append_sso_settings(entry: &mut String, settings: &SsoSettings) {
    let _ = writeln!(entry, "sso_start_url = {}", settings.start_url);
    let _ = writeln!(entry, "sso_region = {}", settings.sso_region);
}

fn choose_session(
    local: &Profiles,
    settings: &SsoSettings,
    session: &SsoSession,
) -> (String, bool) {
    // A reused name must resolve to the same portal, region, and scopes because the CLI caches by name.
    let mut name = session.name.clone();
    let prefix: String = session.name.chars().take(56).collect();
    for suffix in 2.. {
        match local.sso_sessions.get(&name) {
            None => return (name, false),
            Some(existing) if same_session(existing, settings, session) => return (name, true),
            Some(_) => name = format!("{prefix}-{suffix}"),
        }
    }
    unreachable!()
}

fn same_session(local: &Section, settings: &SsoSettings, session: &SsoSession) -> bool {
    local.get("sso_start_url") == Some(&settings.start_url)
        && local.get("sso_region") == Some(&settings.sso_region)
        && local
            .get("sso_registration_scopes")
            .filter(|value| !value.is_empty())
            == session.scopes.as_ref()
}
