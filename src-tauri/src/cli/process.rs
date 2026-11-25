use super::{
    input::Interaction,
    issue::{Issue, classify},
};
use crate::profiles::Files;
use std::{
    env,
    ffi::OsString,
    path::Path,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::watch,
    time,
};
use zeroize::Zeroizing;

const OUTPUT_LIMIT: usize = 1024 * 1024;

pub(super) const AWS_ENV_REMOVE: &[&str] = &[
    "AWS_PROFILE",
    "AWS_DEFAULT_PROFILE",
    "AWS_ACCESS_KEY_ID",
    "AWS_ACCESS_KEY",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SECRET_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_SECURITY_TOKEN",
    "AWS_CREDENTIAL_FILE",
    "AWS_WEB_IDENTITY_TOKEN_FILE",
    "AWS_ROLE_ARN",
    "AWS_ROLE_SESSION_NAME",
    "AWS_CONTAINER_CREDENTIALS_FULL_URI",
    "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
    "AWS_CONTAINER_AUTHORIZATION_TOKEN",
    "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE",
    "AWS_CREDENTIAL_EXPIRATION",
    "AWS_ACCOUNT_ID",
    "AWS_REGION",
    "AWS_DEFAULT_REGION",
    "AWS_DEFAULT_OUTPUT",
    "AWS_ENDPOINT_URL",
    "AWS_DEFAULTS_MODE",
    "AWS_DATA_PATH",
    "AWS_USE_FIPS_ENDPOINT",
    "AWS_USE_DUALSTACK_ENDPOINT",
    "BROWSER",
    "PAGER",
];

pub(super) const AWS_ENV_SET: &[(&str, &str)] = &[
    ("AWS_PAGER", ""),
    ("AWS_CLI_AUTO_PROMPT", "off"),
    ("AWS_CLI_COLOR", "off"),
    ("AWS_EC2_METADATA_DISABLED", "true"),
    ("AWS_IGNORE_CONFIGURED_ENDPOINT_URLS", "true"),
    ("PYTHONUNBUFFERED", "1"),
    ("PYTHONUTF8", "1"),
    ("AWS_CLI_OUTPUT_ENCODING", "UTF-8"),
];

#[derive(Clone)]
pub struct Cancel(watch::Sender<bool>);

impl Cancel {
    pub fn new() -> Self {
        Self(watch::channel(false).0)
    }

    pub fn cancel(&self) {
        self.0.send_replace(true);
    }

    pub fn requested(&self) -> bool {
        *self.0.borrow()
    }

    pub async fn wait(&self) {
        let _ = self.0.subscribe().wait_for(|value| *value).await;
    }
}

pub(super) struct Captured {
    pub(super) status: ExitStatus,
    pub(super) stdout: Zeroizing<Vec<u8>>,
    pub(super) stderr: Zeroizing<Vec<u8>>,
    pub(super) input_complete: bool,
}

pub(super) fn command(path: &Path, files: Option<&Files>) -> Command {
    let mut command = Command::new(path);
    command.kill_on_drop(true);
    for key in AWS_ENV_REMOVE {
        command.env_remove(key);
    }
    for (key, _) in env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("AWS_ENDPOINT_URL")
        {
            command.env_remove(key);
        }
    }
    command.envs(AWS_ENV_SET.iter().copied());
    if let Some(files) = files {
        command.env("AWS_CONFIG_FILE", &files.config);
        command.env("AWS_SHARED_CREDENTIALS_FILE", &files.credentials);
    }
    command
}

fn hidden(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    #[cfg(not(windows))]
    let _ = command;
}

pub(super) async fn run(
    path: &Path,
    files: Option<&Files>,
    args: &[OsString],
    timeout: Duration,
    cancel: &Cancel,
    mut input: Option<Interaction>,
) -> Result<Captured, Issue> {
    if cancel.requested() {
        return Err(Issue::Cancelled);
    }
    let mut command = command(path, files);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hidden(&mut command);
    let mut child = command.spawn().map_err(|_| Issue::Start)?;
    let result = tokio::select! {
        biased;
        _ = cancel.wait() => Err(Issue::Cancelled),
        result = time::timeout(timeout, collect(&mut child, &mut input)) => {
            result.unwrap_or(Err(Issue::Timeout))
        }
    };
    if result.is_err() {
        let _ = child.kill().await;
    }
    result
}

async fn next_bytes(
    reader: &mut (impl AsyncRead + Unpin),
    buffer: &mut Zeroizing<Vec<u8>>,
) -> Result<usize, Issue> {
    let mut bytes = Zeroizing::new([0; 8192]);
    let count = reader.read(&mut *bytes).await.map_err(|_| Issue::Failed)?;
    if buffer.len() + count > OUTPUT_LIMIT {
        return Err(Issue::Failed);
    }
    buffer.extend_from_slice(&bytes[..count]);
    Ok(count)
}

async fn collect(child: &mut Child, input: &mut Option<Interaction>) -> Result<Captured, Issue> {
    let mut stdin = child.stdin.take();
    let mut stdout = child.stdout.take().ok_or(Issue::Start)?;
    let mut stderr = child.stderr.take().ok_or(Issue::Start)?;
    let mut out = Zeroizing::new(Vec::new());
    let mut err = Zeroizing::new(Vec::new());
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut stdout_offset = 0;
    let mut stderr_offset = 0;
    while stdout_open || stderr_open {
        let written = tokio::select! {
            result = next_bytes(&mut stdout, &mut out), if stdout_open => {
                stdout_open = result? != 0;
                stdout_open.then_some((&out, &mut stdout_offset))
            }
            result = next_bytes(&mut stderr, &mut err), if stderr_open => {
                stderr_open = result? != 0;
                stderr_open.then_some((&err, &mut stderr_offset))
            }
        };
        if let (Some(interaction), Some((written, offset))) = (input.as_mut(), written)
            && let Some(reply) = interaction.response(&written[*offset..])?
        {
            *offset = written.len();
            let pipe = stdin.as_mut().ok_or(Issue::Prompt)?;
            pipe.write_all(reply.as_bytes())
                .await
                .map_err(|_| Issue::Prompt)?;
            pipe.write_all(b"\n").await.map_err(|_| Issue::Prompt)?;
            if interaction.complete() {
                pipe.shutdown().await.map_err(|_| Issue::Prompt)?;
                stdin.take();
            }
        }
    }
    Ok(Captured {
        status: child.wait().await.map_err(|_| Issue::Failed)?,
        stdout: out,
        stderr: err,
        input_complete: input.as_ref().is_some_and(Interaction::complete),
    })
}

impl Captured {
    pub(super) fn issue(&self, mfa: bool) -> Issue {
        classify(&self.stderr, mfa, self.input_complete)
    }
}
