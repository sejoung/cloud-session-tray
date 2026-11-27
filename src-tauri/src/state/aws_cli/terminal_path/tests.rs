use super::*;
use std::{os::unix::fs::PermissionsExt, process::Command};

fn executable(path: &Path, version: &str) {
    fs::write(path, format!("#!/bin/sh\nprintf '%s\\n' '{version}'\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn shell_version(shell: &str, mode: &[&str], home: &Path, path: &Path) -> String {
    let result = Command::new(shell)
        .args(mode)
        .args(["-i", "-c", "aws --version"])
        .env_clear()
        .env("HOME", home)
        .env("ZDOTDIR", home)
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}

#[test]
fn new_shells_follow_the_link_and_restore_the_original_path_on_disable() {
    let home = tempfile::tempdir().unwrap();
    let fallback = home.path().join("fallback");
    fs::create_dir(&fallback).unwrap();
    executable(&fallback.join("aws"), "original CLI");
    let first = home.path().join("first cli");
    let second = home.path().join("second 'cli'");
    executable(&first, "first CLI");
    executable(&second, "second CLI");
    let alias = home.path().join("App Support/terminal-bin/aws");
    let block = shell_block(&alias).unwrap();

    for (shell, mode, file) in [
        ("/bin/bash", &[][..], ".bashrc"),
        ("/bin/bash", &["--login"][..], ".bash_profile"),
        ("/bin/zsh", &["-d"][..], ".zshrc"),
        ("/bin/zsh", &["-d", "-l"][..], ".zlogin"),
    ] {
        if !Path::new(shell).is_file() {
            continue;
        }
        let config = home.path().join(file);
        let prefix = format!(
            "export PATH={}\n# Keep this line",
            shell_words::quote(fallback.to_str().unwrap()),
        );
        fs::write(&config, &prefix).unwrap();
        let files = [config.clone()];
        let edits = prepare_edits(&files, Some(&block)).unwrap();
        apply_edits(&edits).unwrap();
        let second_apply = prepare_edits(&files, Some(&block)).unwrap();
        assert_eq!(second_apply[0].before, second_apply[0].after);

        change_link(&alias, Some(&first)).unwrap();
        assert_eq!(
            shell_version(shell, mode, home.path(), &fallback),
            "first CLI"
        );
        change_link(&alias, Some(&second)).unwrap();
        assert_eq!(
            shell_version(shell, mode, home.path(), &fallback),
            "second CLI"
        );

        let edits = prepare_edits(&files, None).unwrap();
        apply_edits(&edits).unwrap();
        change_link(&alias, None).unwrap();
        assert_eq!(fs::read_to_string(&config).unwrap(), prefix);
        assert_eq!(
            shell_version(shell, mode, home.path(), &fallback),
            "original CLI"
        );
    }
}

#[test]
fn removes_an_app_created_startup_file_only_if_it_has_no_user_content() {
    let owned = rewrite_hook(None, Some("export PATH=/example"))
        .unwrap()
        .unwrap();
    assert!(rewrite_hook(Some(&owned), None).unwrap().is_none());
    let extended = format!("{owned}# User settings\n");
    assert_eq!(
        rewrite_hook(Some(&extended), None).unwrap().as_deref(),
        Some("# User settings\n")
    );
    assert!(rewrite_hook(Some(&format!("\n{START}\n")), None).is_err());
}
