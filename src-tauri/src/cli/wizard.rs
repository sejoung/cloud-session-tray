#[cfg(windows)]
use super::process::command;
#[cfg(target_os = "macos")]
use super::process::{AWS_ENV_REMOVE, AWS_ENV_SET};
use crate::{model::Cli, profiles::Files};
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(any(target_os = "macos", windows))]
use std::{process::Stdio, time::Duration};
#[cfg(target_os = "macos")]
use tokio::process::Command;
#[cfg(any(target_os = "macos", windows))]
use tokio::time;

#[cfg(windows)]
pub async fn configure_sso(cli: &Cli, files: &Files) -> Result<(), String> {
    let mut process = command(&cli.path, Some(files));
    process.creation_flags(0x00000010); // CREATE_NEW_CONSOLE for the interactive wizard
    process
        .args(["configure", "sso"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut child = process
        .spawn()
        .map_err(|_| "SSO 마법사 콘솔을 열 수 없습니다.")?;
    match time::timeout(Duration::from_secs(600), child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(_) => Err("SSO 마법사가 중단되었습니다. 설정을 확인하고 다시 시도해 주세요.".into()),
        Err(_) => {
            let _ = child.kill().await;
            Err("SSO 마법사 응답 시간이 초과되었습니다.".into())
        }
    }
}

#[cfg(target_os = "macos")]
pub async fn configure_sso(cli: &Cli, files: &Files) -> Result<(), String> {
    use std::{fmt::Write as _, os::unix::fs::PermissionsExt};

    let failure = "터미널에서 SSO 마법사를 열 수 없습니다.";
    let work = tempfile::tempdir().map_err(|_| failure)?;
    let script_path = work.path().join("Configure AWS SSO.command");
    let result_path = work.path().join("result");
    let quote = |path: &Path| -> Result<String, String> {
        path.to_str()
            .map(shell_words::quote)
            .map(|word| word.into_owned())
            .ok_or_else(|| "SSO 마법사의 파일 경로를 읽을 수 없습니다.".into())
    };
    let mut script = String::from("#!/bin/sh\n");
    let _ = writeln!(script, "result={}", quote(&result_path)?);
    script.push_str(
        "trap 'printf \"%s\\n\" \"$?\" > \"$result\"' EXIT\ntrap 'exit 130' HUP INT TERM\n",
    );
    let _ = writeln!(script, "unset {}", AWS_ENV_REMOVE.join(" "));
    for (key, value) in AWS_ENV_SET {
        let _ = writeln!(script, "export {key}={}", shell_words::quote(value));
    }
    let _ = writeln!(script, "export AWS_CONFIG_FILE={}", quote(&files.config)?);
    let _ = writeln!(
        script,
        "export AWS_SHARED_CREDENTIALS_FILE={}",
        quote(&files.credentials)?
    );
    let _ = writeln!(script, "{} configure sso", quote(&cli.path)?);
    tokio::fs::write(&script_path, script)
        .await
        .map_err(|_| failure)?;
    tokio::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))
        .await
        .map_err(|_| failure)?;
    let opened = time::timeout(
        Duration::from_secs(12),
        Command::new("/usr/bin/open")
            .args(["-a", "Terminal"])
            .arg(&script_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status(),
    )
    .await
    .map_err(|_| failure)?
    .map_err(|_| failure)?;
    if !opened.success() {
        return Err(failure.into());
    }
    time::timeout(Duration::from_secs(600), async {
        loop {
            match tokio::fs::read_to_string(&result_path).await {
                Ok(result) if result.trim() == "0" => return Ok(()),
                Ok(result) if !result.is_empty() => {
                    return Err(
                        "SSO 마법사가 중단되었습니다. 설정을 확인하고 다시 시도해 주세요.".into(),
                    );
                }
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("SSO 마법사 결과를 읽을 수 없습니다.".into()),
            }
            time::sleep(Duration::from_millis(400)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        Err("SSO 마법사 응답 시간이 초과되었습니다. 터미널 상태를 확인해 주세요.".into())
    })
}

#[cfg(not(any(target_os = "macos", windows)))]
pub async fn configure_sso(_cli: &Cli, _files: &Files) -> Result<(), String> {
    Err("SSO 마법사는 macOS와 Windows에서 지원됩니다.".into())
}
