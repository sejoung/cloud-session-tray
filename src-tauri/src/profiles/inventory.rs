use super::catalog::{Kind, Profiles};
use crate::model::valid_mfa_device;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    name: String,
    preferred: bool,
    kind: Kind,
    region: Option<String>,
    mfa_device: Option<String>,
    sso_session: Option<String>,
    target_allowed: bool,
    in_config: bool,
    in_credentials: bool,
    linked_target: Option<String>,
    linked_source: Option<String>,
}

impl Profiles {
    pub fn public(&self, name: String, preferred: &str) -> ProfileView {
        let get = |key| self.setting(&name, key).map(str::to_owned);
        ProfileView {
            preferred: name == preferred,
            kind: self.kind(&name),
            region: get("region"),
            mfa_device: get("mfa_serial").filter(|serial| valid_mfa_device(serial)),
            sso_session: get("sso_session"),
            target_allowed: self.target_allowed(&name),
            in_config: self.config.contains_key(&name),
            in_credentials: self.credentials.contains_key(&name),
            linked_target: self.linked_target(&name),
            linked_source: self.linked_source(&name),
            name,
        }
    }
}

impl Profiles {
    pub fn linked_target(&self, source: &str) -> Option<String> {
        if self.kind(source) != Kind::IamSource {
            return None;
        }
        if source != "default" {
            return self.target_from_key_name(source).map(str::to_owned);
        }
        let mut targets = self
            .credentials
            .keys()
            .chain(self.config.keys())
            .filter(|alias| alias.ends_with("-keys") && self.same_iam_keys(source, alias))
            .filter_map(|alias| self.target_from_key_name(alias))
            .collect::<std::collections::BTreeSet<_>>();
        if targets.len() == 1 {
            targets.pop_first().map(str::to_owned)
        } else {
            None
        }
    }

    pub fn linked_source(&self, target: &str) -> Option<String> {
        let candidate = format!("{target}-keys");
        if self.kind(&candidate) != Kind::IamSource {
            return None;
        }
        Some(if self.same_iam_keys("default", &candidate) {
            "default".into()
        } else {
            candidate
        })
    }

    fn target_from_key_name<'a>(&self, source: &'a str) -> Option<&'a str> {
        let target = source.strip_suffix("-keys")?;
        (!target.is_empty() && self.name_exists(target) && self.target_allowed(target))
            .then_some(target)
    }

    fn same_iam_keys(&self, left: &str, right: &str) -> bool {
        self.kind(left) == Kind::IamSource
            && self.kind(right) == Kind::IamSource
            && ["aws_access_key_id", "aws_secret_access_key"]
                .iter()
                .all(|key| self.setting(left, key) == self.setting(right, key))
    }
}
