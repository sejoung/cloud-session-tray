use super::issue::Issue;
use std::collections::VecDeque;
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Prompt {
    Otp,
    AccessKey,
    SecretKey,
    Region,
    Output,
    Other,
}

pub(super) struct Interaction {
    replies: VecDeque<(Prompt, Zeroizing<String>)>,
    mfa: bool,
}

impl Interaction {
    pub(super) fn otp(code: Zeroizing<String>) -> Self {
        Self {
            replies: [(Prompt::Otp, code)].into(),
            mfa: true,
        }
    }

    pub(super) fn configure(
        access_key_id: Zeroizing<String>,
        secret_access_key: Zeroizing<String>,
        region: String,
        output_format: &str,
    ) -> Self {
        Self {
            replies: [
                (Prompt::AccessKey, access_key_id),
                (Prompt::SecretKey, secret_access_key),
                (Prompt::Region, Zeroizing::new(region)),
                (Prompt::Output, Zeroizing::new(output_format.into())),
            ]
            .into(),
            mfa: false,
        }
    }

    pub(super) fn complete(&self) -> bool {
        self.replies.is_empty()
    }

    pub(super) fn response(&mut self, output: &[u8]) -> Result<Option<Zeroizing<String>>, Issue> {
        let Some((expected, _)) = self.replies.front() else {
            return Ok(None);
        };
        let Some(prompt) = prompt_kind(output) else {
            return Ok(None);
        };
        if *expected != prompt {
            return Err(if self.mfa {
                Issue::CredentialsPrompt
            } else {
                Issue::Prompt
            });
        }
        Ok(self.replies.pop_front().map(|(_, value)| value))
    }
}

fn prompt_kind(output: &[u8]) -> Option<Prompt> {
    // The CLI flushes its non-interactive prompts with no trailing newline.
    let last = output
        .rsplit(|&byte| byte == b'\n' || byte == b'\r')
        .next()?;
    let line = std::str::from_utf8(last).ok()?.trim();
    if line == "MFA token code:" {
        return Some(Prompt::Otp);
    }
    let label = line
        .strip_suffix(':')?
        .split_once(" [")
        .map_or_else(|| line.trim_end_matches(':'), |(name, _)| name);
    match label {
        "AWS Access Key ID" => Some(Prompt::AccessKey),
        "AWS Secret Access Key" => Some(Prompt::SecretKey),
        "Default region name" => Some(Prompt::Region),
        "Default output format" => Some(Prompt::Output),
        "MFA serial number or ARN" | "Profile to update" | "AWS Session Token" => {
            Some(Prompt::Other)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mfa_only_answers_full_otp_prompts() {
        let mut input = Interaction::otp(Zeroizing::new("123456".into()));
        assert!(input.response(b"MFA token").unwrap().is_none());
        assert_eq!(
            input.response(b"AWS Access Key ID [None]: ").unwrap_err(),
            Issue::CredentialsPrompt,
        );
        assert_eq!(
            input
                .response(b"message\r\nMFA token code: ")
                .unwrap()
                .unwrap()
                .as_str(),
            "123456",
        );
        assert!(input.complete());
        assert!(input.response(b"MFA token code: ").unwrap().is_none());
    }
}
