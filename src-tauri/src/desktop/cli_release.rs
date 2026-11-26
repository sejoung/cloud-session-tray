use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

const VERSION_URL: &str = "https://awscli.amazonaws.com/v2/version.txt";
const VERSION_LIMIT: usize = 64;

#[derive(Default)]
pub(super) struct CliReleaseCache(Mutex<Option<ReleaseCheck>>);

#[derive(Deserialize, Serialize)]
struct ReleaseCheck {
    checked_at: DateTime<Utc>,
    version: Option<String>,
}

impl ReleaseCheck {
    fn is_recent(&self, now: DateTime<Utc>) -> bool {
        (TimeDelta::zero()..TimeDelta::days(1)).contains(&(now - self.checked_at))
    }
}

#[tauri::command]
pub(super) async fn latest_cli_version(
    app: AppHandle,
    cache: State<'_, CliReleaseCache>,
    force: bool,
) -> Result<Option<String>, String> {
    let mut cached = cache.0.lock().await;
    let path = app
        .path()
        .app_cache_dir()
        .ok()
        .map(|directory| directory.join("aws-cli-release.json"));
    if cached.is_none()
        && let Some(path) = &path
        && let Ok(bytes) = tokio::fs::read(path).await
    {
        *cached = serde_json::from_slice(&bytes).ok();
    }
    if !force && let Some(result) = cached.as_ref().filter(|entry| entry.is_recent(Utc::now())) {
        return Ok(result.version.clone());
    }

    let version = fetch_version().await;
    let result = ReleaseCheck {
        checked_at: Utc::now(),
        version: version.clone(),
    };
    if let Some(path) = path
        && let Some(directory) = path.parent()
        && let Ok(bytes) = serde_json::to_vec(&result)
        && tokio::fs::create_dir_all(directory).await.is_ok()
    {
        let _ = tokio::fs::write(&path, bytes).await;
    }
    *cached = Some(result);
    Ok(version)
}

async fn fetch_version() -> Option<String> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(6))
        .build()
        .ok()?;
    let mut response = client.get(VERSION_URL).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if chunk.len() > VERSION_LIMIT - bytes.len() {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    parse_version(&bytes)
}

fn parse_version(bytes: &[u8]) -> Option<String> {
    let version = std::str::from_utf8(bytes).ok()?.trim();
    let (minor, patch) = version.strip_prefix("2.")?.split_once('.')?;
    [minor, patch]
        .into_iter()
        .all(|part| part.bytes().all(|byte| byte.is_ascii_digit()) && part.parse::<u32>().is_ok())
        .then(|| version.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_published_v2_version_numbers() {
        assert_eq!(parse_version(b"2.42.4\r\n").as_deref(), Some("2.42.4"));
        for invalid in ["1.42.4", "2.42", "2.42.4.5", "2.+42.4", "<html>"] {
            assert_eq!(parse_version(invalid.as_bytes()), None);
        }
    }

    #[test]
    fn caches_successes_and_unavailable_results_for_one_day() {
        let now = Utc::now();
        for version in [None, Some("2.42.4".into())] {
            let check = ReleaseCheck {
                checked_at: now,
                version,
            };
            assert!(check.is_recent(now + TimeDelta::hours(23)));
            assert!(!check.is_recent(now + TimeDelta::hours(24)));
            assert!(!check.is_recent(now - TimeDelta::seconds(1)));
        }
    }
}
