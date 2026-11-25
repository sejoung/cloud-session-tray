use super::core::AppState;
use crate::{
    cli::{self, Cancel},
    model::{Identity, Login, Status},
    profiles::{Kind, Profiles},
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConnection {
    pub profile: String,
    pub cli: PathBuf,
    pub config: PathBuf,
    pub credentials: PathBuf,
    pub region: Option<String>,
    pub region_configured: bool,
    pub kind: Kind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCheck {
    pub status: Status,
    pub identity: Option<Identity>,
    pub message: Option<String>,
}

impl AppState {
    pub fn tool_connection(&self, profile: String) -> Result<ToolConnection, String> {
        let cli = self.current_cli()?;
        let profiles = self.files.read()?;
        let kind = profiles.kind(&profile);
        let saved = kind == Kind::Unconfigured
            && self
                .sessions()
                .iter()
                .any(|session| session.login.profile() == profile);
        if !saved && !matches!(kind, Kind::IamSource | Kind::IamSession | Kind::Sso) {
            return Err("도구에 연결할 수 있는 AWS 프로필을 선택해 주세요.".into());
        }
        let region = profiles.region(&profile);
        Ok(ToolConnection {
            region: region
                .map(str::to_owned)
                .or_else(|| self.inherited_region(&profiles, &profile))
                .or_else(|| {
                    profiles
                        .linked_source(&profile)
                        .and_then(|source| profiles.region(&source).map(str::to_owned))
                }),
            region_configured: region.is_some(),
            profile,
            cli: cli.path,
            config: self.files.config.clone(),
            credentials: self.files.credentials.clone(),
            kind,
        })
    }

    pub async fn test_connection(&self, profile: &str) -> Result<ConnectionCheck, String> {
        let cli = self.current_cli()?;
        let profiles = self.files.read()?;
        profiles.ensure_history_disabled(profile)?;
        match profiles.kind(profile) {
            Kind::IamSession | Kind::IamSource | Kind::Sso => {}
            Kind::Unconfigured => {
                return Ok(ConnectionCheck {
                    status: Status::NeedsRenewal,
                    identity: None,
                    message: Some("이 AWS 프로필에 자격 증명이 없습니다.".into()),
                });
            }
            Kind::Unsupported => return Err("앱에서 지원하는 AWS 프로필을 선택해 주세요.".into()),
        }
        let result = cli::identity(&cli, &self.files, profile, &Cancel::new()).await;
        if profiles.fingerprint(&self.files, profile)
            != self.files.read()?.fingerprint(&self.files, profile)
        {
            return Err("확인 중 AWS 프로필 파일이 변경되었습니다. 다시 확인해 주세요.".into());
        }
        Ok(match result {
            Ok(identity) => ConnectionCheck {
                status: Status::Available,
                identity: Some(identity),
                message: None,
            },
            Err(issue) => ConnectionCheck {
                status: issue.status(),
                identity: None,
                message: Some(issue.text().into()),
            },
        })
    }

    fn inherited_region(&self, profiles: &Profiles, profile: &str) -> Option<String> {
        let state = self.data();
        state
            .settings
            .sessions
            .iter()
            .find_map(|session| match &session.login {
                Login::IamMfa(login) if login.target == profile => login
                    .region
                    .clone()
                    .or_else(|| profiles.region(&login.source).map(str::to_owned)),
                _ => None,
            })
    }
}
