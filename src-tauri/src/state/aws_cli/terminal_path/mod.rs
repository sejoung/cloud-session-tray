use crate::state::core::{AppState, Data};
use std::{
    collections::HashSet,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

const START: &str = "# >>> Cloud Session Tray AWS CLI PATH >>>";
const END: &str = "# <<< Cloud Session Tray AWS CLI PATH <<<";
const CREATED: &str = "# Cloud Session Tray created this file";
const CONFIG_ERROR: &str = "errors.terminalPathConfig";
const LINK_ERROR: &str = "errors.terminalPathLink";

#[derive(Clone, Copy)]
enum Shell {
    Zsh,
    Bash,
}

fn current_shell() -> Option<Shell> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
    match Path::new(&shell).file_name()?.to_str()? {
        "zsh" => Some(Shell::Zsh),
        "bash" => Some(Shell::Bash),
        _ => None,
    }
}

impl AppState {
    pub(in crate::state) fn terminal_path_supported(&self) -> bool {
        current_shell().is_some()
    }

    pub(super) fn terminal_alias(&self) -> PathBuf {
        self.settings_path
            .with_file_name("terminal-bin")
            .join("aws")
    }

    pub(super) fn sync_terminal_path(&self, data: &mut Data) {
        if data.settings.terminal_path_files.is_empty() {
            return;
        }
        let result = cli_target(data)
            .and_then(|target| change_link(&self.terminal_alias(), target.as_deref()));
        data.terminal_path_error = result.err().map(str::to_owned);
        if let Some(error) = &data.terminal_path_error {
            log::warn!("AWS CLI terminal PATH synchronization failed: code={error}");
        }
    }

    pub async fn set_terminal_path(&self, enabled: bool) -> Result<(), String> {
        let _guard = self.discovery.lock().await;
        let mut data = self.data();
        self.configure_terminal_path(&mut data, enabled)
            .map_err(str::to_owned)
    }

    fn configure_terminal_path(&self, data: &mut Data, enabled: bool) -> Result<(), &'static str> {
        let currently_enabled = !data.settings.terminal_path_files.is_empty();
        if currently_enabled == enabled {
            return Ok(());
        }
        let (files, target) = if enabled {
            let shell = current_shell().ok_or("errors.terminalPathUnsupported")?;
            (
                startup_files(&self.home, shell),
                Some(cli_target(data)?.ok_or("errors.cliRequired")?),
            )
        } else {
            (data.settings.terminal_path_files.clone(), None)
        };
        let alias = self.terminal_alias();
        let previous = link_target(&alias)?;
        let block = enabled.then(|| shell_block(&alias)).transpose()?;
        let edits = prepare_edits(&files, block.as_deref())?;
        apply_edits(&edits)?;
        let result = change_link(&alias, target.as_deref()).and_then(|()| {
            let files = if enabled {
                edits.iter().map(|edit| edit.path.clone()).collect()
            } else {
                Vec::new()
            };
            self.update_settings(data, |settings| settings.terminal_path_files = files)
                .map_err(|_| "errors.settingsSave")
        });
        if let Err(error) = result {
            rollback(&edits);
            if change_link(&alias, previous.as_deref()).is_err() {
                log::warn!("Could not restore the AWS CLI terminal link");
            }
            return Err(error);
        }
        data.terminal_path_error = None;
        log::info!("AWS CLI terminal PATH integration: enabled={enabled}");
        Ok(())
    }
}

fn cli_target(data: &Data) -> Result<Option<PathBuf>, &'static str> {
    data.cli
        .as_ref()
        .filter(|cli| cli.v2)
        .map(|cli| cli.path.canonicalize().map_err(|_| LINK_ERROR))
        .transpose()
}

fn startup_files(home: &Path, shell: Shell) -> Vec<PathBuf> {
    match shell {
        Shell::Zsh => {
            let directory = env::var_os("ZDOTDIR")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| home.to_path_buf());
            let mut files = vec![directory.join(".zshrc")];
            let login = directory.join(".zlogin");
            if login.exists() {
                files.push(login);
            }
            files
        }
        Shell::Bash => {
            let login = [".bash_profile", ".bash_login", ".profile"]
                .iter()
                .map(|name| home.join(name))
                .find(|path| path.exists())
                .unwrap_or_else(|| home.join(".bash_profile"));
            vec![home.join(".bashrc"), login]
        }
    }
}

fn shell_block(alias: &Path) -> Result<String, &'static str> {
    let executable = alias.to_str().ok_or(CONFIG_ERROR)?;
    let directory = alias.parent().and_then(Path::to_str).ok_or(CONFIG_ERROR)?;
    if directory.contains([':', '\n', '\r']) {
        return Err(CONFIG_ERROR);
    }
    let directory = shell_words::quote(directory);
    let executable = shell_words::quote(executable);
    Ok(format!(
        "if [ -x {executable} ]; then\n  case \"$PATH\" in\n    {directory}|{directory}:*) ;;\n    *) export PATH={directory}\"${{PATH:+:$PATH}}\" ;;\n  esac\nfi\n"
    ))
}

struct Edit {
    path: PathBuf,
    before: Option<String>,
    after: Option<String>,
}

fn prepare_edits(files: &[PathBuf], block: Option<&str>) -> Result<Vec<Edit>, &'static str> {
    let mut seen = HashSet::new();
    let mut edits = Vec::new();
    for file in files {
        let path = match file.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound && !file.is_symlink() => {
                file.clone()
            }
            Err(_) => return Err(CONFIG_ERROR),
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        let before = read_file(&path).map_err(|_| CONFIG_ERROR)?;
        let after = rewrite_hook(before.as_deref(), block)?;
        edits.push(Edit {
            path,
            before,
            after,
        });
    }
    Ok(edits)
}

fn rewrite_hook(before: Option<&str>, block: Option<&str>) -> Result<Option<String>, &'static str> {
    let original = before.unwrap_or_default();
    let start = format!("\n{START}\n");
    let end = format!("\n{END}\n");
    let (remaining, created) = if let Some((before, managed)) = original.split_once(&start) {
        let (managed, after) = managed.split_once(&end).ok_or(CONFIG_ERROR)?;
        if after.contains(&start) {
            return Err(CONFIG_ERROR);
        }
        (
            format!("{before}{after}"),
            managed.lines().next() == Some(CREATED),
        )
    } else if original.lines().any(|line| line == START || line == END) {
        return Err(CONFIG_ERROR);
    } else {
        (original.to_owned(), before.is_none())
    };
    match block {
        Some(code) => {
            let marker = if created {
                format!("{CREATED}\n")
            } else {
                String::new()
            };
            Ok(Some(format!(
                "{remaining}{start}{marker}{}{end}",
                code.trim_end_matches('\n')
            )))
        }
        None if remaining.is_empty() && created => Ok(None),
        None => Ok(Some(remaining)),
    }
}

fn read_file(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == io::ErrorKind::NotFound && !path.is_symlink() => Ok(None),
        Err(error) => Err(error),
    }
}

fn replace_file(path: &Path, before: Option<&str>, after: Option<&str>) -> io::Result<()> {
    if read_file(path)?.as_deref() != before {
        return Err(io::Error::other("shell startup file changed"));
    }
    if before == after {
        return Ok(());
    }
    let Some(content) = after else {
        return fs::remove_file(path);
    };
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("missing shell configuration directory"))?;
    let mut pending = tempfile::NamedTempFile::new_in(parent)?;
    if before.is_some() {
        pending
            .as_file()
            .set_permissions(fs::metadata(path)?.permissions())?;
    }
    pending.write_all(content.as_bytes())?;
    pending.as_file().sync_all()?;
    pending.persist(path)?;
    Ok(())
}

fn apply_edits(edits: &[Edit]) -> Result<(), &'static str> {
    for (index, edit) in edits.iter().enumerate() {
        if replace_file(&edit.path, edit.before.as_deref(), edit.after.as_deref()).is_err() {
            rollback(&edits[..index]);
            return Err(CONFIG_ERROR);
        }
    }
    Ok(())
}

fn rollback(edits: &[Edit]) {
    for edit in edits.iter().rev() {
        if replace_file(&edit.path, edit.after.as_deref(), edit.before.as_deref()).is_err() {
            log::warn!("Could not restore a shell startup file");
        }
    }
}

fn link_target(alias: &Path) -> Result<Option<PathBuf>, &'static str> {
    match fs::read_link(alias) {
        Ok(target) => Ok(Some(target)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(LINK_ERROR),
    }
}

fn change_link(alias: &Path, target: Option<&Path>) -> Result<(), &'static str> {
    if link_target(alias)?.as_deref() == target {
        return Ok(());
    }
    let Some(target) = target else {
        return fs::remove_file(alias).map_err(|_| LINK_ERROR);
    };
    #[cfg(unix)]
    {
        let directory = alias.parent().ok_or(LINK_ERROR)?;
        if target == alias || target.starts_with(directory) {
            return Err(LINK_ERROR);
        }
        fs::create_dir_all(directory).map_err(|_| LINK_ERROR)?;
        let staging = tempfile::tempdir_in(directory).map_err(|_| LINK_ERROR)?;
        let staged = staging.path().join("aws");
        std::os::unix::fs::symlink(target, &staged).map_err(|_| LINK_ERROR)?;
        fs::rename(staged, alias).map_err(|_| LINK_ERROR)
    }
    #[cfg(not(unix))]
    {
        let _ = target;
        Err("errors.terminalPathUnsupported")
    }
}

#[cfg(all(test, unix))]
mod tests;
