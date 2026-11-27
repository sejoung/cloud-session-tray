use super::AppState;
use crate::{
    model::{SharedAuthentication, SharedProfile, SsoSession, SsoSettings},
    profiles::{Files, Kind},
};
use std::fs;
use tempfile::TempDir;
use tokio::runtime::Handle;

fn local_app(work: &TempDir) -> AppState {
    let mut app = AppState::new(
        work.path().to_path_buf(),
        work.path().join("app"),
        Handle::current(),
    )
    .unwrap();
    app.files = Files {
        config: work.path().join("config"),
        credentials: work.path().join("credentials"),
    };
    app
}

#[tokio::test]
async fn shares_existing_target_and_uses_default_when_source_keys_match() {
    let work = tempfile::tempdir().unwrap();
    let app = local_app(&work);
    fs::write(&app.files.config, "[profile tech]\nregion=ap-northeast-2\n").unwrap();
    fs::write(&app.files.credentials, concat!(
        "[default]\naws_access_key_id=SOURCEID\naws_secret_access_key=SYNTHETICSECRET\n",
        "[tech-keys]\naws_access_key_id=SOURCEID\naws_secret_access_key=SYNTHETICSECRET\n",
        "[tech]\naws_access_key_id=TEMPID\naws_secret_access_key=TEMPSECRET\naws_session_token=SECRET_SESSION_TOKEN\n",
    )).unwrap();

    let profiles = app.files.read().unwrap();
    assert_eq!(profiles.linked_target("default").as_deref(), Some("tech"));
    assert_eq!(profiles.linked_source("tech").as_deref(), Some("default"));
    for name in ["tech", "default"] {
        let shared = app
            .share_profile(name, "Check the dev deployment".into())
            .unwrap();
        assert_eq!(shared.profile, "tech");
        assert_eq!(shared.task, "Check the dev deployment");
        assert!(
            matches!(shared.authentication, SharedAuthentication::IamMfa { ref source_hint, .. }
            if source_hint.as_deref() == Some("default"))
        );
        let json = serde_json::to_string(&shared).unwrap();
        for sensitive in [
            "SOURCEID",
            "SYNTHETICSECRET",
            "TEMPID",
            "TEMPSECRET",
            "SECRET_SESSION_TOKEN",
        ] {
            assert!(!json.contains(sensitive));
        }
    }
}

#[tokio::test]
async fn imports_sso_with_a_colliding_session_name_without_modifying_credentials() {
    let work = tempfile::tempdir().unwrap();
    let app = local_app(&work);
    let existing_config = concat!(
        "# local configuration\n",
        "[sso-session company]\nsso_region=us-east-1\nsso_start_url=https://existing.example.com/start\n",
    );
    let existing_keys =
        b"[default]\naws_access_key_id=TESTID\naws_secret_access_key=SYNTHETICSECRET\n";
    fs::write(&app.files.config, existing_config).unwrap();
    fs::write(&app.files.credentials, existing_keys).unwrap();
    let profile = SharedProfile {
        version: 1,
        profile: "team-dev".into(),
        label: "Team dev".into(),
        task: "Inspect pods".into(),
        region: Some("ap-northeast-2".into()),
        authentication: SharedAuthentication::Sso {
            settings: SsoSettings {
                start_url: "https://team.example.com/start".into(),
                sso_region: "ap-northeast-2".into(),
                account_id: "123456789012".into(),
                role_name: "Developer".into(),
                session: Some(SsoSession {
                    name: "company".into(),
                    scopes: Some("sso:account:access".into()),
                }),
            },
        },
    };

    let id = app.import_sso(profile.clone()).await.unwrap();
    let written = fs::read_to_string(&app.files.config).unwrap();
    assert!(written.starts_with(existing_config));
    assert!(written.contains("sso_session = company-2"));
    assert!(app.files.read().unwrap().kind("team-dev") == Kind::Sso);
    assert_eq!(fs::read(&app.files.credentials).unwrap(), existing_keys);
    assert_eq!(app.sessions()[0].task, "Inspect pods");

    assert_eq!(app.import_sso(profile.clone()).await.unwrap(), id);
    assert_eq!(fs::read_to_string(&app.files.config).unwrap(), written);
    assert!(
        app.import_sso(SharedProfile {
            profile: "default".into(),
            ..profile
        })
        .await
        .is_err()
    );
    assert_eq!(fs::read(&app.files.credentials).unwrap(), existing_keys);
}

#[test]
fn disallows_executable_or_credential_fields_in_share_files() {
    let json = serde_json::json!({
        "version": 1, "profile": "team", "label": "Team", "task": "", "region": null,
        "authentication": {"kind":"iamMfa", "sourceHint":"default", "duration":43200,
            "credential_process":"unexpected"}
    });
    assert!(serde_json::from_value::<SharedProfile>(json).is_err());
}
