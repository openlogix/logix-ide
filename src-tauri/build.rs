fn main() {
    // The app icons are built into the binary (tauri::generate_context!),
    // so rebuild when they change.
    println!("cargo:rerun-if-changed=icons");
    app_version();
    tauri_build::build()
}

/// Logix IDE's version, as people see it: its release date, yyyy.mm.dd (the
/// Open Logix versioning rule). Release builds pass the release tag
/// (yyyy-mm-dd) in LOGIX_IDE_BUILD_VERSION; other builds use today's UTC
/// date. Passed to the code as LOGIX_IDE_BUILD_VERSION (see version.rs).
fn app_version() {
    println!("cargo:rerun-if-env-changed=LOGIX_IDE_BUILD_VERSION");
    let version = match std::env::var("LOGIX_IDE_BUILD_VERSION") {
        Ok(tag) if !tag.is_empty() => {
            let b = tag.as_bytes();
            let ok = b.len() == 10
                && b[4] == b'-'
                && b[7] == b'-'
                && b.iter()
                    .enumerate()
                    .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
            assert!(
                ok,
                "LOGIX_IDE_BUILD_VERSION must be a release tag, yyyy-mm-dd, got {tag:?}"
            );
            tag.replace('-', ".")
        }
        _ => today_utc(),
    };
    println!("cargo:rustc-env=LOGIX_IDE_BUILD_VERSION={version}");
}

/// Today's UTC date as yyyy.mm.dd.
fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before 1970")
        .as_secs();
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}.{m:02}.{d:02}")
}
