use super::{
    arguments::args,
    installations::Installation,
    process::{Cancel, run},
};
use crate::model::Cli;
use std::{
    cmp::Reverse,
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
    // A freshly installed executable can take longer to start on its first invocation.
    let output = run(
        path,
        None,
        &args(&["--version"]),
        Duration::from_secs(15),
        &cancel,
        None,
    )
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
            Duration::from_secs(6),
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

pub async fn discover(
    preferred: Option<&Path>,
    home: &Path,
    terminal_alias: &Path,
) -> Vec<Installation> {
    let paths = candidates(home)
        .into_iter()
        .filter(|path| path != terminal_alias)
        .collect();
    scan_cli(preferred.filter(|path| *path != terminal_alias), paths)
        .await
        .into_iter()
        .map(|cli| Installation::detect(cli, home))
        .collect()
}

async fn scan_cli(preferred: Option<&Path>, paths: Vec<PathBuf>) -> Vec<Cli> {
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for path in preferred.map(Path::to_path_buf).into_iter().chain(paths) {
        if !seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
            continue;
        }
        if let Some(cli) = inspect(&path).await {
            found.push(cli);
        }
    }
    found.sort_by_key(|cli| Reverse((Some(cli.path.as_path()) == preferred, discovery_rank(cli))));
    found
}

fn discovery_rank(cli: &Cli) -> (bool, bool, [u32; 3]) {
    let mut parts = cli.version.split('.');
    let version = std::array::from_fn(|_| {
        parts
            .next()
            .and_then(|part| part.parse().ok())
            .unwrap_or_default()
    });
    (cli.v2, cli.mfa_supported, version)
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
        use super::installations::{BREW_FORMULAS, BREW_PREFIXES};

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
        for prefix in BREW_PREFIXES {
            paths.extend(
                BREW_FORMULAS
                    .map(|formula| Path::new(prefix).join("opt").join(formula).join("bin/aws")),
            );
        }
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    fn executable(dir: &Path, name: &str, version: &str, mfa: bool) -> PathBuf {
        let path = dir.join(name);
        let probe = if mfa {
            "echo cloud-session-tray-probe >&2"
        } else {
            ":"
        };
        fs::write(
            &path,
            format!(
                "#!/bin/sh\ncase \"$1\" in\n  --version) echo aws-cli/{version} ;;\n  *) {probe}; exit 2 ;;\nesac\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[tokio::test]
    async fn picks_the_newest_compatible_cli_across_installation_paths() {
        let dir = tempfile::tempdir().unwrap();
        let older = executable(dir.path(), "user-cli", "2.9.0", true);
        let updated = executable(dir.path(), "system-cli", "2.42.4", true);
        let same_version = executable(dir.path(), "alternate-cli", "2.42.4", true);
        let no_mfa = executable(dir.path(), "limited-cli", "2.50.0", false);
        let paths = vec![older, updated.clone(), same_version, no_mfa];

        let found = scan_cli(None, paths).await;
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].path, updated);
    }

    #[tokio::test]
    async fn respects_an_explicit_executable_selection() {
        let dir = tempfile::tempdir().unwrap();
        let pinned = executable(dir.path(), "pinned-cli", "2.40.4", true);
        let updated = executable(dir.path(), "system-cli", "2.42.4", true);
        let found = scan_cli(Some(&pinned), vec![updated]).await;
        let selected = &found[0];

        assert_eq!(selected.path, pinned);
        assert_eq!(selected.version, "2.40.4");
    }
}
