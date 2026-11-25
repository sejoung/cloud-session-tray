use serde::{Deserialize, Serialize};
use url::Url;

pub const SHARE_FILE_LIMIT: usize = 64 * 1024;
pub const TASK_LIMIT: usize = 4000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SharedProfile {
    pub version: u8,
    pub profile: String,
    pub label: String,
    pub task: String,
    pub region: Option<String>,
    pub authentication: SharedAuthentication,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SharedAuthentication {
    IamMfa {
        source_hint: Option<String>,
        duration: Option<u32>,
    },
    Sso {
        settings: SsoSettings,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SsoSettings {
    pub start_url: String,
    pub sso_region: String,
    pub account_id: String,
    pub role_name: String,
    pub session: Option<SsoSession>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SsoSession {
    pub name: String,
    pub scopes: Option<String>,
}

pub fn valid_aws_region(region: &str) -> bool {
    !region.is_empty()
        && region.len() <= 64
        && region
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub fn valid_task(task: &str) -> bool {
    task.chars().count() <= TASK_LIMIT
        && !task
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

pub fn valid_shared_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name != "DEFAULT"
        && name == name.trim()
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '[' | ']'))
}

impl SharedProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("지원하지 않는 CST 공유 파일 버전입니다.".into());
        }
        if !valid_shared_name(&self.profile)
            || self.label.trim().is_empty()
            || self.label.chars().count() > 64
            || self.label.chars().any(char::is_control)
            || !valid_task(&self.task)
            || self.region.as_deref().is_some_and(|r| !valid_aws_region(r))
        {
            return Err("공유 파일의 이름, 작업 메모 또는 리전 값을 확인해 주세요.".into());
        }
        match &self.authentication {
            SharedAuthentication::IamMfa {
                source_hint,
                duration,
            } => {
                if source_hint
                    .as_deref()
                    .is_some_and(|name| !valid_shared_name(name))
                    || duration.is_some_and(|seconds| !(900..=129_600).contains(&seconds))
                {
                    return Err("공유 파일의 IAM 프로필 설정을 확인해 주세요.".into());
                }
                Ok(())
            }
            SharedAuthentication::Sso { settings } => settings.validate(),
        }
    }
}

impl SsoSettings {
    fn validate(&self) -> Result<(), String> {
        let valid_url = self.start_url == self.start_url.trim()
            && !self.start_url.chars().any(char::is_control)
            && Url::parse(&self.start_url).is_ok_and(|url| {
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            });
        let valid_role = !self.role_name.is_empty()
            && self.role_name.len() <= 128
            && self
                .role_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_+=,.@-".contains(c));
        if !valid_url
            || !valid_aws_region(&self.sso_region)
            || !valid_role
            || self.account_id.len() != 12
            || !self.account_id.bytes().all(|b| b.is_ascii_digit())
            || self
                .session
                .as_ref()
                .is_some_and(|session| !session.valid())
        {
            return Err(
                "공유 파일의 Identity Center URL, 계정 또는 역할 설정을 확인해 주세요.".into(),
            );
        }
        Ok(())
    }
}

impl SsoSession {
    fn valid(&self) -> bool {
        valid_shared_name(&self.name)
            && self.scopes.as_deref().is_none_or(|scopes| {
                scopes.len() <= 512
                    && scopes
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || ":,_-. ".contains(c))
            })
    }
}
