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
    pub fn key(self) -> &'static str {
        match self {
            Self::Cancelled => "messages.cancelled",
            Self::Timeout => "errors.cliTimeout",
            Self::Start => "errors.cliStart",
            Self::CredentialsPrompt => "errors.credentialsPrompt",
            Self::MfaRejected => "errors.mfaRejected",
            Self::Prompt => "errors.otpPrompt",
            Self::Network => "errors.awsNetwork",
            Self::Auth => "errors.awsAuth",
            Self::Denied => "errors.awsDenied",
            Self::Write => "errors.credentialsWrite",
            Self::Config => "errors.awsConfig",
            Self::Browser => "errors.authBrowser",
            Self::Failed => "errors.cliFailed",
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
