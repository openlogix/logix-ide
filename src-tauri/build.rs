fn main() {
    // The app icons are built into the binary (tauri::generate_context!),
    // so rebuild when they change.
    println!("cargo:rerun-if-changed=icons");
    apio_release();
    tauri_build::build()
}

/// The Apio release Logix uses is pinned in package.json (`appConstants`,
/// as in the Apio VS Code extension). Passes it to the code as
/// LOGIX_APIO_RELEASE_REPO / LOGIX_APIO_RELEASE_TAG (see apio/mod.rs).
fn apio_release() {
    const PACKAGE: &str = "../package.json";
    println!("cargo:rerun-if-changed={PACKAGE}");
    let text = std::fs::read_to_string(PACKAGE).expect("can't read package.json");
    let json: serde_json::Value =
        serde_json::from_str(&text).expect("package.json isn't valid JSON");
    let value = |key: &str| -> String {
        json.pointer(&format!("/appConstants/{key}"))
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| panic!("package.json has no appConstants.{key}"))
            .to_string()
    };
    let repo = value("apioCliReleaseRepo");
    let tag = value("apioCliReleaseTag");
    // Both end up in a download URL.
    let plain = |s: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    };
    assert!(
        plain(&repo) && repo.split('/').count() == 2,
        "appConstants.apioCliReleaseRepo must look like \"owner/repo\", not {repo:?}"
    );
    assert!(
        plain(&tag) && !tag.contains('/'),
        "bad appConstants.apioCliReleaseTag {tag:?}"
    );
    println!("cargo:rustc-env=LOGIX_APIO_RELEASE_REPO={repo}");
    println!("cargo:rustc-env=LOGIX_APIO_RELEASE_TAG={tag}");
}
