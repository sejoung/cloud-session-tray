use super::{AppState, storage::save_settings};
use crate::model::{Language, Preferences, Settings};
use std::fs;
use tokio::runtime::Handle;

#[tokio::test]
async fn follows_system_language_for_old_preferences_and_persists_an_override() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("settings.json"),
        r#"{"preferences":{"notify":false,"notifyMinutes":10}}"#,
    )
    .unwrap();
    let open = || {
        AppState::new(
            directory.path().to_path_buf(),
            directory.path().to_path_buf(),
            Handle::current(),
        )
        .unwrap()
    };
    let state = open();
    state.set_system_language(Language::Ko);
    assert!(matches!(state.language(), Language::Ko));
    state
        .save_preferences(Preferences {
            language: Some(Language::En),
            ..Preferences::default()
        })
        .unwrap();
    let reopened = open();
    reopened.set_system_language(Language::Ko);
    assert!(matches!(reopened.language(), Language::En));
}

#[tokio::test]
async fn migrates_settings_only_when_the_current_file_is_missing() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("io.github.sejoung.cst");
    let legacy_path = directory
        .path()
        .join("io.github.sejoung.cloud-session-tray/settings.json");
    let legacy = Settings {
        preferences: Preferences {
            notify: true,
            notify_minutes: 25,
            ..Preferences::default()
        },
        ..Settings::default()
    };
    save_settings(&legacy_path, &legacy).unwrap();
    let original = fs::read(&legacy_path).unwrap();

    let app = AppState::new(
        directory.path().to_path_buf(),
        config.clone(),
        Handle::current(),
    )
    .unwrap();
    assert!(app.data().settings.preferences.notify);
    assert_eq!(app.data().settings.preferences.notify_minutes, 25);
    assert!(config.join("settings.json").is_file());
    app.save_preferences(Preferences::default()).unwrap();

    let reopened =
        AppState::new(directory.path().to_path_buf(), config, Handle::current()).unwrap();
    assert!(!reopened.data().settings.preferences.notify);
    assert_eq!(fs::read(legacy_path).unwrap(), original);
}

#[tokio::test]
async fn expiration_notifications_follow_current_profile_evidence_and_send_once() {
    use crate::model::{Expiry, Login, MfaLogin, Session};
    use crate::profiles::Files;
    use chrono::{Duration, Utc};

    let directory = tempfile::tempdir().unwrap();
    let mut app = AppState::new(
        directory.path().to_path_buf(),
        directory.path().join("app"),
        Handle::current(),
    )
    .unwrap();
    app.files = Files {
        config: directory.path().join("config"),
        credentials: directory.path().join("credentials"),
    };
    fs::write(
        &app.files.credentials,
        "[work]\naws_access_key_id=fake\naws_secret_access_key=fake\naws_session_token=fake\n",
    )
    .unwrap();
    let profiles = app.files.read().unwrap();
    let expiry = Expiry {
        at: Utc::now() + Duration::minutes(5),
        fingerprint: profiles.fingerprint(&app.files, "work"),
        revision: profiles.revision().into(),
        notified: false,
    };
    app.update_settings(&mut app.data(), |settings| {
        settings.preferences = Preferences {
            notify: true,
            notify_minutes: 10,
            ..Preferences::default()
        };
        settings.sessions.push(Session {
            id: "work-id".into(),
            label: "Work".into(),
            task: String::new(),
            login: Login::IamMfa(MfaLogin {
                source: "source".into(),
                target: "work".into(),
                device: "SERIAL".into(),
                duration: None,
                region: None,
            }),
        });
        settings.expiries.insert("work-id".into(), expiry.clone());
    })
    .unwrap();

    assert_eq!(app.notifications().len(), 1);
    app.mark_notified("work-id", &expiry);
    assert!(app.notifications().is_empty());

    app.update_settings(&mut app.data(), |settings| {
        settings.expiries.insert("work-id".into(), expiry);
    })
    .unwrap();
    fs::write(&app.files.config, "# Changed externally").unwrap();
    assert!(app.notifications().is_empty());
}
