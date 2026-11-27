use super::{
    arguments::args,
    process::{Cancel, run},
};
use crate::model::Cli;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub(super) const BREW_PREFIXES: [&str; 2] = ["/opt/homebrew", "/usr/local"];
pub(super) const BREW_FORMULAS: [&str; 3] = ["awscli", "awscli@1", "awscli@2"];

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum Source {
    Homebrew,
    AwsUser,
    AwsInstaller,
    Unknown,
}

#[derive(Clone, PartialEq, Eq)]
enum Package {
    Brew {
        executable: PathBuf,
        formula: String,
        directory: PathBuf,
    },
    AwsUser {
        directory: PathBuf,
        bin: PathBuf,
    },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    #[serde(flatten)]
    pub cli: Cli,
    source: Source,
    pub can_remove: bool,
    #[serde(skip)]
    executable: PathBuf,
    #[serde(skip)]
    package: Option<Package>,
}

impl Installation {
    pub fn detect(cli: Cli, home: &Path) -> Self {
        let executable = cli.path.canonicalize().unwrap_or_else(|_| cli.path.clone());
        let package = if cfg!(target_os = "macos") {
            BREW_PREFIXES
                .iter()
                .find_map(|prefix| brew_package(Path::new(prefix), &executable))
                .or_else(|| user_package(home, &executable))
        } else {
            None
        };
        let source = match &package {
            Some(Package::Brew { .. }) => Source::Homebrew,
            Some(Package::AwsUser { .. }) => Source::AwsUser,
            None if executable.starts_with("/usr/local/aws-cli")
                || executable.ends_with(Path::new("Amazon").join("AWSCLIV2").join("aws.exe")) =>
            {
                Source::AwsInstaller
            }
            None => Source::Unknown,
        };
        Self {
            cli,
            source,
            can_remove: package.is_some(),
            executable,
            package,
        }
    }

    pub fn same_executable(&self, other: &Self) -> bool {
        self.executable == other.executable
    }

    pub fn can_remove_while_using(&self, active: &Self) -> bool {
        self.package.is_some()
            && active.cli.v2
            && !self.same_executable(active)
            && self.package != active.package
    }

    pub fn contains(&self, path: &Path) -> bool {
        let Some(Package::Brew { directory, .. } | Package::AwsUser { directory, .. }) =
            &self.package
        else {
            return false;
        };
        path.starts_with(directory)
            || path
                .canonicalize()
                .is_ok_and(|resolved| resolved.starts_with(directory))
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.same_executable(other)
            && self.cli.version == other.cli.version
            && self.package == other.package
    }

    pub async fn uninstall(&self, home: &Path) -> Result<(), String> {
        let current = Self::detect(self.cli.clone(), home);
        if !self.matches(&current) {
            return Err("errors.cliInstallationChanged".into());
        }
        match current.package {
            Some(Package::Brew {
                executable,
                formula,
                ..
            }) => {
                let output = run(
                    &executable,
                    None,
                    &args(&["uninstall", "--formula", &formula]),
                    Duration::from_secs(120),
                    &Cancel::new(),
                    None,
                )
                .await
                .map_err(|_| "errors.cliUninstall")?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err("errors.cliBrewUninstall".into())
                }
            }
            Some(Package::AwsUser { directory, bin }) => {
                tokio::task::spawn_blocking(move || remove_user_installation(&directory, &bin))
                    .await
                    .map_err(|_| "errors.cliUninstall")?
                    .map_err(|_| "errors.cliUninstall".into())
            }
            None => Err("errors.cliInstallationChanged".into()),
        }
    }
}

fn brew_package(prefix: &Path, executable: &Path) -> Option<Package> {
    let cellar = prefix.join("Cellar").canonicalize().ok()?;
    let mut relative = executable.strip_prefix(&cellar).ok()?.components();
    let formula = relative.next()?.as_os_str().to_str()?;
    let version = relative.next()?.as_os_str();
    let brew = prefix.join("bin/brew");
    let directory = cellar.join(formula);
    (executable.file_name().is_some_and(|name| name == "aws")
        && BREW_FORMULAS.contains(&formula)
        && brew.is_file()
        && directory
            .join(version)
            .join("INSTALL_RECEIPT.json")
            .is_file())
    .then(|| Package::Brew {
        executable: brew,
        formula: formula.into(),
        directory,
    })
}

fn user_package(home: &Path, executable: &Path) -> Option<Package> {
    let directory = home.canonicalize().ok()?.join(".local/share/aws-cli");
    (executable == directory.join("aws")
        && directory.join("awscli").is_dir()
        && directory.join("base_library.zip").is_file())
    .then(|| Package::AwsUser {
        directory,
        bin: home.join(".local/bin"),
    })
}

fn remove_user_installation(directory: &Path, bin: &Path) -> std::io::Result<()> {
    let links: Vec<_> = ["aws", "aws_completer"]
        .into_iter()
        .filter_map(|name| {
            let link = bin.join(name);
            let target = fs::read_link(&link).ok()?;
            (link.canonicalize().ok()? == directory.join(name)).then_some((link, target))
        })
        .collect();
    let parent = directory.parent().expect("AWS CLI installation parent");
    let stage = tempfile::tempdir_in(parent)?;
    fs::rename(directory, stage.path().join("aws-cli"))?;
    stage.close()?;
    for (link, target) in links {
        if fs::read_link(&link).ok().as_ref() == Some(&target) {
            fs::remove_file(link)?;
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn identifies_a_homebrew_formula_from_its_keg_and_receipt() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path();
        let keg = prefix.join("Cellar/awscli/2.41.0");
        fs::create_dir_all(keg.join("bin")).unwrap();
        fs::create_dir_all(prefix.join("bin")).unwrap();
        fs::write(prefix.join("bin/brew"), "").unwrap();
        fs::write(keg.join("bin/aws"), "").unwrap();
        let executable = keg.join("bin/aws").canonicalize().unwrap();
        assert!(brew_package(prefix, &executable).is_none());
        fs::write(keg.join("INSTALL_RECEIPT.json"), "{}").unwrap();
        assert!(
            matches!(brew_package(prefix, &executable), Some(Package::Brew { formula, .. }) if formula == "awscli")
        );
    }

    #[test]
    fn user_removal_only_removes_its_own_links_and_installation() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().canonicalize().unwrap();
        let directory = home.join(".local/share/aws-cli");
        let bin = home.join(".local/bin");
        fs::create_dir_all(directory.join("awscli")).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir(home.join(".aws")).unwrap();
        fs::write(directory.join("base_library.zip"), "").unwrap();
        fs::write(directory.join("aws"), "").unwrap();
        fs::write(bin.join("aws_completer"), "other installation").unwrap();
        symlink(directory.join("aws"), bin.join("aws")).unwrap();
        assert!(user_package(&home, &directory.join("aws")).is_some());

        remove_user_installation(&directory, &bin).unwrap();

        assert!(!directory.exists());
        assert!(fs::symlink_metadata(bin.join("aws")).is_err());
        assert_eq!(
            fs::read_to_string(bin.join("aws_completer")).unwrap(),
            "other installation"
        );
        assert!(home.join(".aws").is_dir());
    }
}
