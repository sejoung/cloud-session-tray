mod sharing;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use sharing::{
    SHARE_FILE_LIMIT, SharedAuthentication, SharedProfile, SsoSession, SsoSettings,
    valid_aws_region, valid_task,
};
use std::{collections::BTreeMap, path::PathBuf};
use zeroize::Zeroizing;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cli {
    pub path: PathBuf,
    pub version: String,
    pub v2: bool,
    pub mfa_supported: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Unknown,
    Available,
    NeedsRenewal,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Queued,
    SigningIn,
    Checking,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Identity {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub status: Status,
    pub checked_at: DateTime<Utc>,
    pub identity: Option<Identity>,
    pub message: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expiry {
    pub at: DateTime<Utc>,
    pub fingerprint: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub notified: bool,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Login {
    IamMfa(MfaLogin),
    Sso { profile: String },
}

impl Login {
    pub fn profile(&self) -> &str {
        match self {
            Self::IamMfa(login) => &login.target,
            Self::Sso { profile } => profile,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MfaLogin {
    pub source: String,
    pub target: String,
    pub device: String,
    pub duration: Option<u32>,
    pub region: Option<String>,
}

pub fn valid_mfa_device(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/:_=+,@.-".contains(c))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IamRegistration {
    pub name: String,
    pub access_key_id: Zeroizing<String>,
    pub secret_access_key: Zeroizing<String>,
    pub region: String,
}

impl IamRegistration {
    pub fn validate(&self) -> Result<(), String> {
        if self.access_key_id.starts_with("ASIA") {
            return Err("IAM 원본 프로필에는 세션 토큰이 없는 장기 액세스 키가 필요합니다.".into());
        }
        if !(16..=128).contains(&self.access_key_id.len())
            || !self
                .access_key_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
            || !(16..=256).contains(&self.secret_access_key.len())
            || !self
                .secret_access_key
                .bytes()
                .all(|byte| byte.is_ascii_graphic())
        {
            return Err("AWS 액세스 키 ID와 비밀 액세스 키를 확인해 주세요.".into());
        }
        if self.region.len() > 64
            || !self
                .region
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err("AWS 리전 이름을 확인해 주세요.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub task: String,
    pub login: Login,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDraft {
    pub id: Option<String>,
    pub label: String,
    pub login: Login,
    #[serde(default)]
    pub task: String,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub notify: bool,
    pub notify_minutes: u16,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            notify: false,
            notify_minutes: 10,
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub cli_path: Option<PathBuf>,
    pub preferences: Preferences,
    pub sessions: Vec<Session>,
    pub checks: BTreeMap<String, Check>,
    pub expiries: BTreeMap<String, Expiry>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    #[serde(flatten)]
    pub session: Session,
    pub status: Status,
    pub phase: Option<Phase>,
    pub identity: Option<Identity>,
    pub checked_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub check_message: Option<String>,
    pub activity: Option<String>,
    pub can_cancel: bool,
}
