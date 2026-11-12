//! Logix IDE's version, as people see it: its release date, `yyyy.mm.dd`
//! (the Open Logix versioning rule, website repo's `CLAUDE.md`). Release tags
//! and pins use the same date as `yyyy-mm-dd`.

/// Logix IDE's version, `yyyy.mm.dd`. Set by build.rs: from the release tag
/// in `LOGIX_IDE_BUILD_VERSION` for releases, otherwise today's UTC date.
pub const APP_VERSION: &str = env!("LOGIX_IDE_BUILD_VERSION");

/// The version people see for a release tag: `yyyy.mm.dd` for `yyyy-mm-dd`.
/// Anything else is returned as is.
pub fn shown(tag: &str) -> String {
    let b = tag.as_bytes();
    let is_tag = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if is_tag {
        tag.replace('-', ".")
    } else {
        tag.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_version_is_a_dotted_date() {
        let b = APP_VERSION.as_bytes();
        assert_eq!(b.len(), 10, "{APP_VERSION}");
        assert!(
            b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 {
                *c == b'.'
            } else {
                c.is_ascii_digit()
            }),
            "{APP_VERSION}"
        );
    }

    #[test]
    fn tags_are_shown_as_dotted_dates() {
        assert_eq!(shown("2026-11-12"), "2026.11.12");
        assert_eq!(shown("2026-11-12-2"), "2026-11-12-2");
    }
}
