use super::{
    arguments::args,
    process::{Cancel, run},
};
use crate::model::Cli;
use std::{
    env,
    path::{Path, PathBuf},
    time::Duration,
};

pub async fn inspect(path: &Path) -> Option<Cli> {
    if !path.is_absolute() || !path.is_file() {
        return None;
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    let cancel = Cancel::new();
    let timeout = Duration::from_secs(6);
    let output = run(path, None, &args(&["--version"]), timeout, &cancel, None)
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = [&output.stdout, &output.stderr]
        .iter()
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .find_map(|text| {
            text.split_whitespace()
                .find_map(|word| word.strip_prefix("aws-cli/"))
        })?;
    if !version
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
        || !version.contains('.')
    {
        return None;
    }
    let v2 = version.starts_with("2.");
    let mfa_supported = if v2 {
        // This stops at argument conversion, before any credential loading or network access.
        run(
            path,
            None,
            &args(&[
                "configure",
                "mfa-login",
                "--duration-seconds",
                "cloud-session-tray-probe",
            ]),
            timeout,
            &cancel,
            None,
        )
        .await
        .ok()
        .is_some_and(|probe| {
            !probe.status.success()
                && String::from_utf8_lossy(&probe.stderr).contains("cloud-session-tray-probe")
                && !String::from_utf8_lossy(&probe.stderr).contains("Unknown options")
        })
    } else {
        false
    };
    Some(Cli {
        path: path.to_path_buf(),
        version: version.into(),
        v2,
        mfa_supported,
    })
}

pub async fn discover(preferred: Option<&Path>, home: &Path) -> Option<Cli> {
    if let Some(path) = preferred
        && let Some(cli) = inspect(path).await
    {
        return Some(cli);
    }
    let mut fallback: Option<Cli> = None;
    let mut seen = std::collections::HashSet::new();
    for path in candidates(home) {
        if !seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
            continue;
        }
        if let Some(cli) = inspect(&path).await {
            if cli.mfa_supported {
                return Some(cli);
            }
            if fallback.as_ref().is_none_or(|found| !found.v2 && cli.v2) {
                fallback = Some(cli);
            }
        }
    }
    fallback
}

#[cfg_attr(
    not(any(target_os = "macos", windows)),
    expect(
        unused_mut,
        unused_variables,
        reason = "Only macOS and Windows have additional AWS CLI search paths"
    )
)]
fn candidates(home: &Path) -> Vec<PathBuf> {
    let name = if cfg!(windows) { "aws.exe" } else { "aws" };
    let mut paths: Vec<_> = env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .filter(|path| path.is_absolute())
        .map(|dir| dir.join(name))
        .collect();
    #[cfg(target_os = "macos")]
    {
        paths.extend([
            home.join(".local/bin/aws"),
            home.join(".local/share/aws-cli/aws"),
            home.join("aws-cli/aws"),
            home.join("Library/Amazon/AWSCLIV2/aws"),
        ]);
        paths.extend(
            [
                "/usr/local/bin/aws",
                "/usr/local/aws-cli/aws",
                "/opt/homebrew/bin/aws",
                "/opt/local/bin/aws",
            ]
            .into_iter()
            .map(PathBuf::from),
        );
    }
    #[cfg(windows)]
    {
        let usual = Path::new("Amazon").join("AWSCLIV2").join(name);
        if let Some(local) = env::var_os("LOCALAPPDATA") {
            paths.push(PathBuf::from(local).join("Programs").join(&usual));
        }
        for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(folder) = env::var_os(var) {
                paths.push(PathBuf::from(folder).join(&usual));
            }
        }
        paths.push(
            home.join("scoop")
                .join("apps")
                .join("aws")
                .join("current")
                .join(name),
        );
        paths.push(
            home.join("AppData")
                .join("Local")
                .join("Programs")
                .join(usual),
        );
    }
    paths
}
