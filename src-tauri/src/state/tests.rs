use super::{Action, AppState};
use crate::model::{IamRegistration, MfaLogin};
use crate::{
    cli::Issue,
    model::{Login, SessionDraft, Status},
    profiles::Files,
};
use std::os::unix::fs::PermissionsExt;
use std::{fs, sync::Arc};
use tempfile::TempDir;
use tokio::runtime::Handle;
use zeroize::Zeroizing;

const FAKE_CLI: &str = r#"#!/bin/sh
case "$1 $2" in
  '--version ') printf '%s\n' 'aws-cli/2.40.4 Python/3.14.8'; exit 0 ;;
  'configure mfa-login')
    if [ "$4" = 'cloud-session-tray-probe' ]; then
      printf '%s\n' 'invalid literal cloud-session-tray-probe' >&2
      exit 252
    fi
    for arg in "$@"; do
      case "$arg" in --update-profile=*) target=${arg#*=} ;; esac
    done
    printf '%s' 'MFA token'
    sleep 0.02
    printf '%s' ' code: '
    IFS= read -r code || exit 2
    if IFS= read -r extra; then exit 3; fi
    if [ "$code" != 123456 ]; then
      printf '%s\n' 'MultiFactorAuthentication failed' >&2
      exit 254
    fi
    printf '\n[%s]\naws_access_key_id=fake\naws_secret_access_key=fake\naws_session_token=fake\n' "$target" >> "$AWS_SHARED_CREDENTIALS_FILE"
    printf "Temporary credentials written to profile '%s'\n" "$target"
    printf '%s\n' 'Credentials will expire at 2099-01-01 00:00:00 UTC'
    ;;
  'configure set')
    for arg in "$@"; do
      case "$arg" in --profile=*) profile=${arg#*=} ;; esac
    done
    [ "$3" = region ] || exit 5
    printf '\n[profile %s]\nregion=%s\n' "$profile" "$4" >> "$AWS_CONFIG_FILE"
    ;;
  'configure --no-cli-pager')
    for arg in "$@"; do
      case "$arg" in --profile=*) profile=${arg#*=} ;; esac
    done
    printf 'AWS Access Key ID [None]: '
    IFS= read -r key || exit 4
    printf 'AWS Secret Access Key [None]: '
    IFS= read -r secret || exit 4
    printf 'Default region name [None]: '
    IFS= read -r region || exit 4
    printf 'Default output format [None]: '
    IFS= read -r output || exit 4
    if [ -n "$key" ]; then
      printf '[%s]\naws_access_key_id=%s\naws_secret_access_key=%s\n' "$profile" "$key" "$secret" >> "$AWS_SHARED_CREDENTIALS_FILE" || exit 5
    fi
    if [ -n "$region$output" ]; then
      if [ -f "$AWS_CONFIG_FILE.fail-write" ]; then exit 5; fi
      printf '[profile %s]\nregion=%s\noutput=%s\n' "$profile" "$region" "$output" >> "$AWS_CONFIG_FILE" || exit 5
    fi
    ;;
  'configure list-profiles') printf 'new-source\n' ;;
  'iam list-mfa-devices')
    case " $* " in
      *" --profile=new-source "*) printf '%s\n' '["SERIAL2","SERIAL1","SERIAL1","invalid spaces"]' ;;
      *) exit 5 ;;
    esac
    ;;
  'sso login')
    printf 'login\n' >> "$AWS_CONFIG_FILE.sso-calls"
    if [ -f "$AWS_CONFIG_FILE.slow-sso" ]; then exec sleep 3; fi
    sleep 0.1
    ;;
  'sts get-caller-identity')
    for arg in "$@"; do
      case "$arg" in --profile=*) profile=${arg#*=} ;; esac
    done
    printf '{"Account":"123456789012","Arn":"arn:aws:iam::123456789012:role/%s","UserId":"example"}\n' "$profile"
    ;;
  *) exit 1 ;;
esac
"#;

fn test_state(work: &TempDir) -> AppState {
    let mut state = AppState::new(
        work.path().to_path_buf(),
        work.path().join("app"),
        Handle::current(),
    )
    .unwrap();
    state.files = Files {
        config: work.path().join("aws-config"),
        credentials: work.path().join("aws-credentials"),
    };
    state
}

async fn fake_state(work: &TempDir) -> Arc<AppState> {
    let state = Arc::new(test_state(work));
    let executable = work.path().join("fake aws");
    fs::write(&executable, FAKE_CLI).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    state.choose_cli(executable).await.unwrap();
    state
}

fn draft(label: &str, login: Login) -> SessionDraft {
    SessionDraft {
        id: None,
        label: label.into(),
        login,
        task: String::new(),
    }
}

fn iam(target: &str) -> Login {
    Login::IamMfa(MfaLogin {
        source: "iam-source".into(),
        target: target.into(),
        device: "FAKEDEVICE".into(),
        duration: Some(900),
        region: None,
    })
}

async fn finished(state: &AppState) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while state.has_jobs() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn iam_renewal_survives_restart_and_invalidates_on_external_edits() {
    let work = tempfile::tempdir().unwrap();
    let state = fake_state(&work).await;
    fs::write(
        &state.files.config,
        "[profile iam-source]\nregion=us-east-1\n[profile other]\nregion=ap-northeast-2\n",
    )
    .unwrap();
    fs::write(
        &state.files.credentials,
        "[iam-source]\naws_access_key_id=fake-long-lived\naws_secret_access_key=fake\n",
    )
    .unwrap();
    let first = state.save_session(draft("Work", iam("work"))).unwrap();
    let second = state.save_session(draft("Other", iam("other"))).unwrap();
    assert!(
        state
            .save_session(draft("Overwrite", iam("iam-source")))
            .is_err()
    );

    state
        .start(&first, Action::Mfa(Zeroizing::new("123456".into())))
        .unwrap();
    state
        .start(&second, Action::Mfa(Zeroizing::new("123456".into())))
        .unwrap();
    finished(&state).await;
    let profiles = state.files.read().unwrap();
    assert_eq!(profiles.region("work"), Some("us-east-1"));
    assert_eq!(profiles.region("other"), Some("ap-northeast-2"));
    for session in state.dashboard().sessions {
        assert!(matches!(session.status, Status::Available));
        assert!(session.expires_at.is_some());
        assert!(
            session
                .identity
                .unwrap()
                .arn
                .ends_with(session.session.login.profile())
        );
    }

    let before = fs::read(&state.files.credentials).unwrap();
    state
        .start(&first, Action::Mfa(Zeroizing::new("654321".into())))
        .unwrap();
    finished(&state).await;
    assert_eq!(fs::read(&state.files.credentials).unwrap(), before);
    assert_eq!(
        state.dashboard().sessions[0].activity.as_deref(),
        Some(Issue::MfaRejected.text())
    );

    let restarted = test_state(&work);
    assert!(
        restarted
            .dashboard()
            .sessions
            .iter()
            .all(|session| session.expires_at.is_some())
    );
    let mut changed = before;
    changed.extend_from_slice(b"\n# external file edit\n");
    fs::write(&restarted.files.credentials, changed).unwrap();
    assert!(
        restarted
            .dashboard()
            .sessions
            .iter()
            .all(|session| session.expires_at.is_none())
    );
}

#[tokio::test]
async fn shared_sso_login_checks_each_profile_once() {
    let work = tempfile::tempdir().unwrap();
    let state = fake_state(&work).await;
    fs::write(&state.files.config, concat!(
            "[sso-session organization]\nsso_start_url=https://example.test/start\nsso_region=us-east-1\n",
            "[profile dev]\nsso_session=organization\nsso_account_id=123456789012\nsso_role_name=Developer\n",
            "[profile audit]\nsso_session=organization\nsso_account_id=123456789012\nsso_role_name=Audit\n",
        )).unwrap();
    let ids: Vec<_> = ["dev", "audit"]
        .into_iter()
        .map(|profile| {
            state
                .save_session(draft(
                    profile,
                    Login::Sso {
                        profile: profile.into(),
                    },
                ))
                .unwrap()
        })
        .collect();
    state.start(&ids[0], Action::Sso).unwrap();
    state.start(&ids[1], Action::Sso).unwrap();
    finished(&state).await;
    let calls = work.path().join("aws-config.sso-calls");
    assert_eq!(fs::read_to_string(calls).unwrap(), "login\n");
    for session in state.dashboard().sessions {
        assert!(matches!(session.status, Status::Available));
        assert!(
            session
                .identity
                .unwrap()
                .arn
                .ends_with(session.session.login.profile())
        );
        assert!(session.expires_at.is_none());
    }
}

#[tokio::test]
async fn registers_an_iam_source_and_lists_only_valid_devices() {
    let work = tempfile::tempdir().unwrap();
    let state = fake_state(&work).await;
    let registration = || IamRegistration {
        name: "new-source".into(),
        access_key_id: Zeroizing::new("AKIAEXAMPLE000000000".into()),
        secret_access_key: Zeroizing::new("synthetic-secret-for-tests-only".into()),
        region: "ap-northeast-2".into(),
    };

    let profiles = state.register_iam(registration()).await.unwrap();
    let profiles = serde_json::to_value(profiles).unwrap();
    assert_eq!(profiles[0]["name"], "new-source");
    assert_eq!(profiles[0]["kind"], "iamSource");
    assert_eq!(profiles[0]["region"], "ap-northeast-2");
    let config = fs::read_to_string(&state.files.config).unwrap();
    assert!(config.contains("output=json\n"));
    assert_eq!(
        state.mfa_devices("new-source").await.unwrap(),
        ["SERIAL1", "SERIAL2"]
    );

    let credentials = fs::read(&state.files.credentials).unwrap();
    assert!(state.register_iam(registration()).await.is_err());
    assert_eq!(fs::read(&state.files.credentials).unwrap(), credentials);
    let settings = fs::read_to_string(work.path().join("app/settings.json")).unwrap();
    assert!(!settings.contains("synthetic-secret-for-tests-only"));
    assert!(!profiles.to_string().contains("AKIAEXAMPLE"));
}

#[tokio::test]
async fn config_write_failure_leaves_no_keys_and_registration_can_be_retried() {
    let work = tempfile::tempdir().unwrap();
    let state = fake_state(&work).await;
    let write_failure = work.path().join("aws-config.fail-write");
    fs::write(&write_failure, "").unwrap();
    let registration = || IamRegistration {
        name: "new-source".into(),
        access_key_id: Zeroizing::new("AKIAEXAMPLE000000000".into()),
        secret_access_key: Zeroizing::new("synthetic-secret-for-tests-only".into()),
        region: String::new(),
    };

    let result = state.register_iam(registration()).await;
    assert!(result.is_err());
    assert!(!state.files.credentials.exists());
    fs::remove_file(&write_failure).unwrap();
    assert!(state.register_iam(registration()).await.is_ok());
}

#[tokio::test]
async fn cancelled_identity_center_login_can_be_retried() {
    let work = tempfile::tempdir().unwrap();
    let state = fake_state(&work).await;
    fs::write(
        &state.files.config,
        concat!(
            "[profile dev]\nsso_start_url=https://example.test/start\nsso_region=us-east-1\n",
            "sso_account_id=123456789012\nsso_role_name=Developer\n",
        ),
    )
    .unwrap();
    let slow = work.path().join("aws-config.slow-sso");
    let calls = work.path().join("aws-config.sso-calls");
    fs::write(&slow, "").unwrap();
    let id = state
        .save_session(draft(
            "Dev",
            Login::Sso {
                profile: "dev".into(),
            },
        ))
        .unwrap();

    state.start(&id, Action::Sso).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !calls.is_file() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    state.cancel(&id);
    finished(&state).await;
    assert_eq!(
        state.dashboard().sessions[0].activity.as_deref(),
        Some(Issue::Cancelled.text())
    );

    fs::remove_file(&slow).unwrap();
    state.start(&id, Action::Sso).unwrap();
    finished(&state).await;
    assert!(matches!(
        state.dashboard().sessions[0].status,
        Status::Available
    ));
    assert_eq!(fs::read_to_string(calls).unwrap(), "login\nlogin\n");
}
