//! Проверка GitHub Releases на доступность новой версии клиента.

use std::cmp::Ordering;

use serde::Deserialize;

use super::handle::AvailableUpdate;

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/loject/cheenhub/releases/latest";
const USER_AGENT: &str = concat!("CheenHub/", env!("CARGO_PKG_VERSION"));

/// Результат проверки последнего GitHub Release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UpdateCheckOutcome {
    /// Текущая версия не ниже последнего опубликованного релиза.
    Current,
    /// На GitHub опубликована новая версия.
    Available(AvailableUpdate),
}

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
}

/// Проверяет последний стабильный релиз CheenHub на GitHub.
pub(crate) async fn check_latest_release() -> Result<UpdateCheckOutcome, String> {
    let response = reqwest::Client::new()
        .get(GITHUB_LATEST_RELEASE_URL)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("Не удалось связаться с GitHub: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub вернул ответ {} при проверке обновлений.",
            response.status()
        ));
    }

    let release = response
        .json::<GithubRelease>()
        .await
        .map_err(|error| format!("Не удалось прочитать ответ GitHub: {error}"))?;

    if release.draft || release.prerelease {
        return Ok(UpdateCheckOutcome::Current);
    }

    let latest_version = normalize_release_version(&release.tag_name);
    let current_version = env!("CARGO_PKG_VERSION");
    if compare_versions(&latest_version, current_version) != Ordering::Greater {
        return Ok(UpdateCheckOutcome::Current);
    }

    Ok(UpdateCheckOutcome::Available(AvailableUpdate {
        version: latest_version,
        tag: release.tag_name,
        title: release.name.filter(|name| !name.trim().is_empty()),
        release_url: release.html_url,
    }))
}

fn normalize_release_version(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_owned()
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = version_parts(left);
    let right_parts = version_parts(right);
    let max_len = left_parts.len().max(right_parts.len());

    for index in 0..max_len {
        let left_part = *left_parts.get(index).unwrap_or(&0);
        let right_part = *right_parts.get(index).unwrap_or(&0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    Ordering::Equal
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .split(['.', '-', '+'])
        .map(|part| {
            part.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
        })
        .take_while(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{compare_versions, normalize_release_version};

    #[test]
    fn normalizes_github_release_tags() {
        assert_eq!(normalize_release_version("v0.8.1"), "0.8.1");
        assert_eq!(normalize_release_version("V1.2.3"), "1.2.3");
    }

    #[test]
    fn compares_semver_like_versions() {
        assert_eq!(compare_versions("0.8.1", "0.8.0"), Ordering::Greater);
        assert_eq!(compare_versions("0.8.0", "0.8.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.7.9", "0.8.0"), Ordering::Less);
    }
}
