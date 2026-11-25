use super::{Files, Kind};
use std::fs;

#[test]
fn profiles_with_other_credential_providers_are_not_mfa_targets() {
    let work = tempfile::tempdir().unwrap();
    let files = Files {
        config: work.path().join("custom-config"),
        credentials: work.path().join("custom-credentials"),
    };
    fs::write(&files.config, concat!(
            "[profile role]\nrole_arn=arn:aws:iam::123456789012:role/example\nsource_profile=source\n",
            "[profile \"external process\"]\ncredential_process=example-command\n",
            "[profile sso]\nsso_start_url=https://example.test/start\nsso_region=us-east-1\n",
            "sso_account_id=123456789012\nsso_role_name=Developer\n",
            "[profile empty]\nregion=us-east-1\n",
        )).unwrap();
    fs::write(&files.credentials, concat!(
            "[source]\naws_access_key_id=fake\naws_secret_access_key=fake\n",
            "[temporary]\naws_access_key_id=fake\naws_secret_access_key=fake\naws_session_token=fake\n",
            "[legacy]\naws_access_key_id=fake\naws_secret_access_key=fake\naws_security_token=fake\n",
        )).unwrap();

    let profiles = files.read().unwrap();
    for name in ["role", "external process", "sso", "source", "legacy"] {
        assert!(!profiles.target_allowed(name), "{name}");
    }
    for name in ["empty", "temporary", "new-target"] {
        assert!(profiles.target_allowed(name), "{name}");
    }
    assert!(profiles.kind("source") == Kind::IamSource);
    assert!(profiles.kind("sso") == Kind::Sso);
}
