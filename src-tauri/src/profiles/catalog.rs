use super::{
    ini::{Ini, Section},
    paths::Files,
};
use crate::model::{Login, MfaLogin, valid_aws_region, valid_mfa_device};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    IamSource,
    IamSession,
    Sso,
    Unconfigured,
    Unsupported,
}

pub struct Profiles {
    pub(super) config: Ini,
    pub(super) credentials: Ini,
    pub(super) sso_sessions: Ini,
    pub(super) revision: String,
    pub(super) config_digest: String,
}

fn configured(section: Option<&Section>, key: &str) -> bool {
    section
        .and_then(|s| s.get(key))
        .is_some_and(|value| !value.is_empty())
}

fn contains_key(section: Option<&Section>, key: &str) -> bool {
    section.is_some_and(|section| section.contains_key(key))
}

fn invalid_provider(config: Option<&Section>, credentials: Option<&Section>) -> bool {
    [
        "credential_process",
        "credential_source",
        "role_arn",
        "source_profile",
        "web_identity_token_file",
        "login_session",
    ]
    .iter()
    .any(|key| contains_key(config, key) || contains_key(credentials, key))
}

const CREDENTIAL_KEYS: [&str; 4] = [
    "aws_access_key_id",
    "aws_secret_access_key",
    "aws_session_token",
    "aws_security_token",
];

fn has_key_pair(section: Option<&Section>) -> bool {
    configured(section, "aws_access_key_id") && configured(section, "aws_secret_access_key")
}

fn has_token(section: Option<&Section>) -> bool {
    configured(section, "aws_session_token") || configured(section, "aws_security_token")
}

impl Profiles {
    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn same_credentials_revision(&self, other: &Self) -> bool {
        self.revision
            .split_once('/')
            .map(|(_, credentials)| credentials)
            == other
                .revision
                .split_once('/')
                .map(|(_, credentials)| credentials)
    }

    pub fn changed_only_target(&self, other: &Self, target: &str) -> bool {
        self.config_digest == other.config_digest && other_credentials_match(self, other, target)
    }

    pub fn can_rebind_region_change(&self, other: &Self, target: &str) -> bool {
        let others = |name: &&String| name.as_str() != target;
        other_credentials_match(self, other, target)
            && self.sso_sessions == other.sso_sessions
            && self
                .config
                .iter()
                .filter(|(name, _)| others(name))
                .eq(other.config.iter().filter(|(name, _)| others(name)))
    }

    pub fn kind(&self, name: &str) -> Kind {
        let config = self.config.get(name);
        let credentials = self.credentials.get(name);
        if invalid_provider(config, credentials) {
            return Kind::Unsupported;
        }
        let has_sso = [config, credentials]
            .into_iter()
            .flatten()
            .any(|fields| fields.keys().any(|key| key.starts_with("sso_")));
        let has_credentials = CREDENTIAL_KEYS
            .iter()
            .any(|key| contains_key(config, key) || contains_key(credentials, key));
        if has_sso {
            let session_configured = match config.and_then(|fields| fields.get("sso_session")) {
                Some(session) => ["sso_start_url", "sso_region"]
                    .iter()
                    .all(|key| configured(self.sso_sessions.get(session), key)),
                None => ["sso_start_url", "sso_region"]
                    .iter()
                    .all(|key| configured(config, key)),
            };
            return if !has_credentials
                && session_configured
                && configured(config, "sso_account_id")
                && configured(config, "sso_role_name")
            {
                Kind::Sso
            } else {
                Kind::Unsupported
            };
        }
        if contains_key(credentials, "aws_access_key_id") {
            if !has_key_pair(credentials) {
                return Kind::Unsupported;
            }
            return if has_token(credentials) {
                Kind::IamSession
            } else {
                Kind::IamSource
            };
        }
        if has_key_pair(config) {
            return if has_token(config) {
                Kind::IamSession
            } else {
                Kind::IamSource
            };
        }
        if has_credentials {
            Kind::Unsupported
        } else {
            Kind::Unconfigured
        }
    }

    pub fn target_allowed(&self, name: &str) -> bool {
        let config = self.config.get(name);
        let credentials = self.credentials.get(name);
        if CREDENTIAL_KEYS.iter().any(|key| contains_key(config, key))
            || contains_key(credentials, "aws_security_token")
        {
            return false;
        }
        // A CLI rewrite can recover a partial session left by an interrupted write when a token is present.
        self.kind(name) == Kind::Unconfigured
            || (has_token(credentials)
                && !invalid_provider(config, credentials)
                && [config, credentials]
                    .into_iter()
                    .flatten()
                    .all(|fields| !fields.keys().any(|key| key.starts_with("sso_"))))
    }

    pub fn region(&self, name: &str) -> Option<&str> {
        self.setting(name, "region")
            .filter(|region| valid_aws_region(region))
    }

    pub fn has_output_format(&self, name: &str) -> bool {
        self.setting(name, "output")
            .is_some_and(|format| !format.is_empty())
    }

    pub(super) fn setting(&self, name: &str, key: &str) -> Option<&str> {
        self.credentials
            .get(name)
            .and_then(|fields| fields.get(key))
            .or_else(|| self.config.get(name).and_then(|fields| fields.get(key)))
            .map(String::as_str)
    }

    pub fn validate(&self, login: &Login) -> Result<(), String> {
        validate_name(login.profile())?;
        match login {
            Login::Sso { profile } if self.kind(profile) != Kind::Sso => {
                Err("완전한 IAM Identity Center 설정이 있는 프로필을 선택해 주세요.".into())
            }
            Login::Sso { .. } => Ok(()),
            Login::IamMfa(MfaLogin {
                source,
                target,
                device,
                duration,
                region,
            }) => {
                if source == target {
                    return Err("원본과 갱신 대상에 서로 다른 프로필을 지정해 주세요.".into());
                }
                self.validate_iam_source(source)?;
                if !self.target_allowed(target) {
                    return Err("장기 키, SSO, 역할 또는 외부 자격 증명 설정이 없는 별도 프로필을 선택해 주세요.".into());
                }
                if !valid_mfa_device(device) {
                    return Err("MFA 장치 ARN 또는 시리얼을 확인해 주세요.".into());
                }
                if region
                    .as_deref()
                    .is_some_and(|region| !valid_aws_region(region))
                {
                    return Err("대상 프로필의 리전 이름을 확인해 주세요.".into());
                }
                if duration.is_some_and(|seconds| !(900..=129_600).contains(&seconds)) {
                    return Err("세션 기간은 900~129600초 사이로 입력해 주세요.".into());
                }
                Ok(())
            }
        }
    }

    pub fn validate_iam_source(&self, source: &str) -> Result<(), String> {
        validate_name(source)?;
        if self.kind(source) != Kind::IamSource {
            return Err("원본 프로필에 로컬 IAM 장기 키를 설정해 주세요.".into());
        }
        Ok(())
    }

    pub fn validate_registration(&self, name: &str) -> Result<(), String> {
        validate_name(name)?;
        let section_name = if name.contains(' ') {
            shell_words::quote(name)
        } else {
            name.into()
        };
        if !shell_words::split(&section_name).is_ok_and(|words| words == [name]) {
            return Err("AWS 설정 파일에 저장할 수 있는 프로필 이름을 입력해 주세요.".into());
        }
        if self.kind(name) != Kind::Unconfigured {
            return Err("이미 인증 설정이 있는 프로필입니다. 다른 이름을 지정해 주세요.".into());
        }
        self.ensure_history_disabled(name)
    }

    pub fn ensure_history_disabled(&self, profile: &str) -> Result<(), String> {
        if self
            .credentials
            .get(profile)
            .and_then(|fields| fields.get("cli_history"))
            .or_else(|| {
                self.config
                    .get(profile)
                    .and_then(|fields| fields.get("cli_history"))
            })
            .is_some_and(|value| value == "enabled")
        {
            return Err("이 프로필의 AWS CLI 기록이 켜져 있습니다. cli_history를 disabled로 설정한 뒤 다시 시도해 주세요.".into());
        }
        Ok(())
    }

    pub fn sso_group(&self, profile: &str) -> Option<String> {
        if self.kind(profile) != Kind::Sso {
            return None;
        }
        let fields = self.config.get(profile)?;
        if let Some(session) = fields.get("sso_session") {
            Some(format!("session:{session}"))
        } else {
            fields
                .get("sso_start_url")
                .map(|url| format!("legacy:{url}"))
        }
    }

    pub fn fingerprint(&self, files: &Files, profile: &str) -> String {
        let fields = self.config.get(profile);
        let sso = fields
            .and_then(|fields| fields.get("sso_session"))
            .and_then(|session| self.sso_sessions.get(session));
        let mut hash = Sha256::new();
        // Include paths so a check cannot be reused against a different AWS file pair.
        for value in [
            files.config.to_string_lossy().as_ref(),
            files.credentials.to_string_lossy().as_ref(),
            profile,
        ] {
            hash.update(value.len().to_le_bytes());
            hash.update(value);
        }
        for section in [fields, self.credentials.get(profile), sso] {
            hash.update([0]);
            if let Some(fields) = section {
                for (key, value) in fields {
                    hash.update((key.len() as u64).to_le_bytes());
                    hash.update(key);
                    hash.update((value.len() as u64).to_le_bytes());
                    hash.update(value);
                }
            }
        }
        format!("{:x}", hash.finalize())
    }
}

pub(super) fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name == "DEFAULT"
        || name.len() > 256
        || name != name.trim()
        || name.chars().any(|char| char.is_control() || char == ']')
    {
        return Err("AWS 프로필 이름을 확인해 주세요.".into());
    }
    Ok(())
}

fn other_credentials_match(before: &Profiles, after: &Profiles, target: &str) -> bool {
    let others = |name: &&String| name.as_str() != target;
    before
        .credentials
        .iter()
        .filter(|(name, _)| others(name))
        .eq(after.credentials.iter().filter(|(name, _)| others(name)))
}
