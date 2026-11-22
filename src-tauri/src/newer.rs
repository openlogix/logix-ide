//! The newer-version notice (Milestone 6; the user's decisions,
//! 2026-11-22): when a newer Logix IDE is on this build's channel, a banner
//! under the toolbar says so, with a link to its release page, where its
//! installers and install steps are. Logix IDE never installs itself.
//!
//! - It looks at most once a day: the last answer is kept in `settings.json`
//!   (`newerIdeCheck`) and used until it's a day old, also offline.
//! - Dismissed, the banner stays away for a week (`newerIdeDismissed`),
//!   then comes back if a newer release is still there.
//!
//! Plain Rust, tested; `commands::newer_ide_notice` does the looking.

use serde::{Deserialize, Serialize};

/// How often Logix IDE looks for a newer release, in seconds: a day.
pub const CHECK_EVERY: u64 = 24 * 3600;
/// How long a dismissed banner stays away, in seconds: a week.
pub const QUIET_AFTER_DISMISS: u64 = 7 * 24 * 3600;

/// The last look for a newer release (`newerIdeCheck` in `settings.json`):
/// when, and the newest release on the channel then (its repo and tag),
/// if the config named one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// Unix time, in seconds.
    pub at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// The release's tag, `yyyy-mm-dd`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
}

/// What the banner shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    /// The newer release's version, `yyyy.mm.dd`.
    pub version: String,
    /// This build's version.
    pub current: String,
    /// The release's page, where its installers and install steps are.
    pub url: String,
}

/// Whether to look again: never looked, or the last look is a day old (or
/// in the future, after a clock change).
pub fn due(last: Option<&Check>, now: u64) -> bool {
    last.is_none_or(|c| now < c.at || now - c.at >= CHECK_EVERY)
}

/// The banner to show, if any: the last look found a release newer than
/// `own` (this build's version, `yyyy.mm.dd`), and the banner wasn't
/// dismissed in the last week.
pub fn notice(check: &Check, own: &str, dismissed_at: Option<u64>, now: u64) -> Option<Notice> {
    let (repo, release) = (check.repo.as_ref()?, check.release.as_ref()?);
    if *release <= own.replace('.', "-") {
        return None;
    }
    if dismissed_at.is_some_and(|d| d <= now && now - d < QUIET_AFTER_DISMISS) {
        return None;
    }
    Some(Notice {
        version: crate::version::shown(release),
        current: own.to_string(),
        url: release_page(repo, release),
    })
}

/// A release's page on GitHub.
pub fn release_page(repo: &str, release: &str) -> String {
    format!("https://github.com/{repo}/releases/tag/{release}")
}

/// Now, as Unix time in seconds.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 24 * 3600;

    fn found(release: &str) -> Check {
        Check {
            at: 1000,
            repo: Some("openlogix/logix-ide".into()),
            release: Some(release.into()),
        }
    }

    #[test]
    fn looks_at_most_once_a_day() {
        assert!(due(None, 5));
        let c = found("2026-12-01");
        assert!(!due(Some(&c), 1000));
        assert!(!due(Some(&c), 1000 + DAY - 1));
        assert!(due(Some(&c), 1000 + DAY));
        // A clock set back: look again rather than wait.
        assert!(due(Some(&c), 999));
    }

    #[test]
    fn shows_only_a_newer_release() {
        let n = notice(&found("2026-12-01"), "2026.11.22", None, 2000).unwrap();
        assert_eq!(n.version, "2026.12.01");
        assert_eq!(n.current, "2026.11.22");
        assert_eq!(
            n.url,
            "https://github.com/openlogix/logix-ide/releases/tag/2026-12-01"
        );
        assert_eq!(notice(&found("2026-11-22"), "2026.11.22", None, 2000), None);
        assert_eq!(notice(&found("2026-11-01"), "2026.11.22", None, 2000), None);
        let nothing = Check {
            at: 1000,
            repo: None,
            release: None,
        };
        assert_eq!(notice(&nothing, "2026.11.22", None, 2000), None);
    }

    #[test]
    fn dismissed_stays_away_a_week() {
        let c = found("2026-12-01");
        let dismissed = 10 * DAY;
        assert_eq!(
            notice(&c, "2026.11.22", Some(dismissed), dismissed + 1),
            None
        );
        assert_eq!(
            notice(&c, "2026.11.22", Some(dismissed), dismissed + 7 * DAY - 1),
            None
        );
        assert!(notice(&c, "2026.11.22", Some(dismissed), dismissed + 7 * DAY).is_some());
        // Dismissed "in the future" (a clock change) doesn't hide it forever.
        assert!(notice(&c, "2026.11.22", Some(dismissed), dismissed - 1).is_some());
    }

    #[test]
    fn the_check_is_stored_in_camel_case() {
        let text = serde_json::to_string(&found("2026-12-01")).unwrap();
        assert_eq!(
            text,
            r#"{"at":1000,"repo":"openlogix/logix-ide","release":"2026-12-01"}"#
        );
        let empty: Check = serde_json::from_str(r#"{"at":5}"#).unwrap();
        assert_eq!(empty.release, None);
    }
}
