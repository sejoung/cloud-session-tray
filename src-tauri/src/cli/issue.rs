use crate::model::Status;
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Issue {
    Cancelled,
    Timeout,
    Start,
    CredentialsPrompt,
    MfaRejected,
    Prompt,
    Network,
    Auth,
    Denied,
    Write,
    Config,
    Browser,
    Failed,
}

impl Issue {
    pub fn text(self) -> &'static str {
        match self {
            Self::Cancelled => "작업을 취소했습니다.",
            Self::Timeout => "응답 시간이 초과되었습니다. 다시 시도해 주세요.",
            Self::Start => "AWS CLI를 실행할 수 없습니다. 설치 경로를 확인해 주세요.",
            Self::CredentialsPrompt => {
                "AWS CLI가 원본 자격 증명을 요청했습니다. IAM 원본 프로필 설정을 확인해 주세요."
            }
            Self::MfaRejected => "MFA 인증이 거부되었습니다. 장치 설정과 새 OTP를 확인해 주세요.",
            Self::Prompt => {
                "AWS CLI의 OTP 요청을 확인할 수 없습니다. CLI를 업데이트한 뒤 다시 시도해 주세요."
            }
            Self::Network => {
                "AWS에 연결할 수 없습니다. 네트워크, 프록시 또는 인증서를 확인해 주세요."
            }
            Self::Auth => "자격 증명이 없거나 만료되었습니다. 세션을 갱신해 주세요.",
            Self::Denied => "이 작업에 필요한 AWS 권한이 없습니다.",
            Self::Write => "AWS 자격 증명 파일에 기록할 수 없습니다. 경로와 권한을 확인해 주세요.",
            Self::Config => {
                "AWS 프로필이나 설정 파일을 읽을 수 없습니다. 설정과 파일 권한을 확인해 주세요."
            }
            Self::Browser => {
                "기본 브라우저에서 인증을 열 수 없습니다. 브라우저 설정을 확인해 주세요."
            }
            Self::Failed => {
                "AWS CLI 명령이 실패했습니다. 프로필 설정을 확인한 뒤 다시 시도해 주세요."
            }
        }
    }

    pub fn status(self) -> Status {
        match self {
            Self::Auth => Status::NeedsRenewal,
            _ => Status::Unknown,
        }
    }
}

pub(super) fn classify(stderr: &[u8], mfa: bool, sent_otp: bool) -> Issue {
    let stderr = String::from_utf8_lossy(stderr);
    let text = Zeroizing::new(stderr.to_ascii_lowercase());
    let has = |patterns: &[&str]| patterns.iter().any(|pattern| text.contains(pattern));
    if has(&["keyboardinterrupt", "interrupted by user"]) {
        Issue::Cancelled
    } else if mfa
        && has(&[
            "multifactorauthentication",
            "multi-factor authentication",
            "mfa failure",
            "mfa code",
            "mfa token",
        ])
    {
        Issue::MfaRejected
    } else if has(&[
        "expiredtoken",
        "invalidclienttokenid",
        "unrecognizedclientexception",
        "invalid_grant",
        "invalidgrant",
        "token has expired",
        "token is invalid",
        "unable to locate credentials",
        "partial credentials",
        "sso session associated",
        "error loading sso token",
        "unauthorizedexception",
        "invalid sso session",
        "invalid security token",
        "security token included in the request is invalid",
        "invalidaccesskeyid",
        "signaturedoesnotmatch",
    ]) {
        Issue::Auth
    } else if has(&["accessdenied", "not authorized to perform"]) {
        Issue::Denied
    } else if has(&[
        "could not connect",
        "failed to connect",
        "connection reset",
        "connection aborted",
        "endpointconnectionerror",
        "connect timeout",
        "read timeout",
        "timed out",
        "no such host",
        "ssl validation",
        "certificate verify",
        "proxy connection",
        "name or service not known",
        "temporary failure in name resolution",
    ]) {
        Issue::Network
    } else if mfa
        && has(&[
            "permission denied",
            "access is denied",
            "read-only file system",
            "no space left",
            "winerror 5",
            "is a directory",
            "oserror",
        ])
    {
        Issue::Write
    } else if has(&[
        "profile could not be found",
        "config profile",
        "unable to parse config",
        "invalidconfigerror",
        "invalid configuration",
    ]) {
        Issue::Config
    } else if has(&[
        "could not open browser",
        "unable to open browser",
        "failed to open browser",
    ]) {
        Issue::Browser
    } else if mfa && !sent_otp {
        Issue::Prompt
    } else {
        Issue::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_errors_and_connection_errors_have_different_states() {
        assert!(matches!(
            classify(b"ExpiredToken: session expired", false, false).status(),
            Status::NeedsRenewal
        ));
        assert!(matches!(
            classify(b"Could not connect to the endpoint URL", false, false).status(),
            Status::Unknown
        ));
        assert_eq!(
            classify(
                b"AccessDenied: not authorized to perform iam:ListMFADevices",
                false,
                false
            ),
            Issue::Denied
        );
        assert_eq!(
            classify(
                b"AccessDenied: MultiFactorAuthentication failed",
                true,
                true
            ),
            Issue::MfaRejected
        );
    }
}
