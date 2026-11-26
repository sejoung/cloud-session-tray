use super::{
    arguments::{args, profile_option, short_network_command},
    input::Interaction,
    issue::Issue,
    process::{Cancel, run},
};
use crate::{
    model::{Cli, IamRegistration, Identity, MfaLogin, valid_mfa_device},
    profiles::Files,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de::DeserializeOwned;
use std::{ffi::OsString, time::Duration};
use zeroize::Zeroizing;

pub async fn list(cli: &Cli, files: &Files) -> Result<Vec<String>, Issue> {
    let output = run(
        &cli.path,
        Some(files),
        &args(&["configure", "list-profiles"]),
        Duration::from_secs(12),
        &Cancel::new(),
        None,
    )
    .await?;
    if !output.status.success() {
        return Err(output.issue(false));
    }
    let stdout = std::str::from_utf8(&output.stdout).map_err(|_| Issue::Config)?;
    let mut names: Vec<_> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

pub async fn identity(
    cli: &Cli,
    files: &Files,
    profile: &str,
    cancel: &Cancel,
) -> Result<Identity, Issue> {
    let arguments = short_network_command(&["sts", "get-caller-identity"], profile);
    run_json(cli, files, arguments, cancel).await
}

pub async fn mfa_devices(cli: &Cli, files: &Files, source: &str) -> Result<Vec<String>, Issue> {
    let arguments = short_network_command(
        &[
            "iam",
            "list-mfa-devices",
            "--query",
            "MFADevices[].SerialNumber",
        ],
        source,
    );
    let mut devices: Vec<String> = run_json(cli, files, arguments, &Cancel::new()).await?;
    devices.retain(|serial| valid_mfa_device(serial));
    devices.sort();
    devices.dedup();
    Ok(devices)
}

async fn run_json<T: DeserializeOwned>(
    cli: &Cli,
    files: &Files,
    mut arguments: Vec<OsString>,
    cancel: &Cancel,
) -> Result<T, Issue> {
    arguments.extend(args(&["--output", "json"]));
    let output = run(
        &cli.path,
        Some(files),
        &arguments,
        Duration::from_secs(25),
        cancel,
        None,
    )
    .await?;
    if !output.status.success() {
        return Err(output.issue(false));
    }
    serde_json::from_slice(&output.stdout).map_err(|_| Issue::Failed)
}

pub async fn mfa(
    cli: &Cli,
    files: &Files,
    login: &MfaLogin,
    otp: Zeroizing<String>,
    cancel: &Cancel,
) -> Result<Option<DateTime<Utc>>, Issue> {
    let mut arguments = short_network_command(&["configure", "mfa-login"], &login.source);
    arguments.extend([
        profile_option("update-profile", &login.target),
        profile_option("serial-number", &login.device),
    ]);
    if let Some(seconds) = login.duration {
        arguments.push(profile_option("duration-seconds", &seconds.to_string()));
    }
    let output = run(
        &cli.path,
        Some(files),
        &arguments,
        Duration::from_secs(45),
        cancel,
        Some(Interaction::otp(otp)),
    )
    .await?;
    if !output.status.success() {
        return Err(output.issue(true));
    }
    if !output.input_complete {
        return Err(Issue::Prompt);
    }
    Ok(expiration(&output.stdout, &login.target))
}

fn expiration(stdout: &[u8], target: &str) -> Option<DateTime<Utc>> {
    let text = std::str::from_utf8(stdout).ok()?;
    let expected = format!("Temporary credentials written to profile '{target}'");
    let mut lines = text.lines();
    lines.find(|line| line.ends_with(&expected))?;
    let value = lines.next()?.strip_prefix("Credentials will expire at ")?;
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S UTC")
        .ok()
        .map(|time| time.and_utc())
        .or_else(|| {
            DateTime::parse_from_rfc3339(value)
                .ok()
                .map(|time| time.to_utc())
        })
}

pub async fn sso(cli: &Cli, files: &Files, profile: &str, cancel: &Cancel) -> Result<(), Issue> {
    let mut arguments = args(&["sso", "login", "--no-cli-pager"]);
    arguments.push(profile_option("profile", profile));
    let output = run(
        &cli.path,
        Some(files),
        &arguments,
        Duration::from_secs(300),
        cancel,
        None,
    )
    .await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(output.issue(false))
    }
}

pub async fn configure_iam(
    cli: &Cli,
    files: &Files,
    profile: IamRegistration,
    output_format: &str,
) -> Result<(), Issue> {
    let mut arguments = args(&["configure", "--no-cli-pager"]);
    arguments.push(profile_option("profile", &profile.name));
    // Write settings first so a failed config write leaves no keys and registration can be retried.
    if !profile.region.is_empty() || !output_format.is_empty() {
        let settings = Interaction::configure(
            Zeroizing::default(),
            Zeroizing::default(),
            profile.region,
            output_format,
        );
        run_configure(cli, files, &arguments, settings).await?;
    }
    let credentials = Interaction::configure(
        profile.access_key_id,
        profile.secret_access_key,
        String::new(),
        "",
    );
    run_configure(cli, files, &arguments, credentials).await
}

async fn run_configure(
    cli: &Cli,
    files: &Files,
    arguments: &[OsString],
    input: Interaction,
) -> Result<(), Issue> {
    let output = run(
        &cli.path,
        Some(files),
        arguments,
        Duration::from_secs(25),
        &Cancel::new(),
        Some(input),
    )
    .await?;
    if !output.status.success() {
        return Err(output.issue(false));
    }
    if !output.input_complete {
        return Err(Issue::Prompt);
    }
    Ok(())
}

pub async fn set_region(
    cli: &Cli,
    files: &Files,
    profile: &str,
    region: &str,
    cancel: &Cancel,
) -> Result<(), Issue> {
    let mut arguments = args(&["configure", "set", "region", region, "--no-cli-pager"]);
    arguments.push(profile_option("profile", profile));
    let output = run(
        &cli.path,
        Some(files),
        &arguments,
        Duration::from_secs(12),
        cancel,
        None,
    )
    .await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(output.issue(false))
    }
}
