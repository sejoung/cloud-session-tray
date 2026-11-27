use crate::model::Language;
use std::{collections::BTreeMap, sync::LazyLock};

type Catalog = BTreeMap<String, String>;

static ENGLISH: LazyLock<Catalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../locales/en.json")).expect("Invalid English catalog")
});
static KOREAN: LazyLock<Catalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../locales/ko.json")).expect("Invalid Korean catalog")
});

pub(super) fn text(language: Language, key: &str) -> &'static str {
    let catalog = match language {
        Language::En => &ENGLISH,
        Language::Ko => &KOREAN,
    };
    catalog
        .get(key)
        .or_else(|| ENGLISH.get(key))
        .map_or("Unknown message", String::as_str)
}

pub(super) fn expiry_body(language: Language, label: &str, time: &str) -> String {
    let pattern = text(language, "native.expiryBody");
    let (before, rest) = pattern
        .split_once("{label}")
        .expect("Missing label placeholder");
    let (between, after) = rest.split_once("{time}").expect("Missing time placeholder");
    format!("{before}{label}{between}{time}{after}")
}

pub(super) fn error_key(key: &str) -> &str {
    if (key.starts_with("errors.") || key.starts_with("messages.")) && ENGLISH.contains_key(key) {
        key
    } else {
        "errors.unexpected"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_messages_use_the_selected_catalog() {
        assert_eq!(
            ENGLISH.keys().collect::<Vec<_>>(),
            KOREAN.keys().collect::<Vec<_>>()
        );
        assert_eq!(text(Language::En, "native.quit"), "Quit");
        assert_ne!(
            text(Language::Ko, "native.quit"),
            text(Language::En, "native.quit")
        );
        assert_eq!(
            expiry_body(Language::En, "Example {time}", "12:30"),
            "The Example {time} session expires at 12:30."
        );
    }

    #[test]
    fn diagnostic_codes_never_include_unrecognized_error_text() {
        assert_eq!(error_key("errors.mfaRejected"), "errors.mfaRejected");
        assert_eq!(
            error_key("Errors containing external data"),
            "errors.unexpected"
        );
    }
}
