fn main() {
    // The app icons are built into the binary (tauri::generate_context!),
    // so rebuild when they change.
    println!("cargo:rerun-if-changed=icons");
    pinned_releases();
    tauri_build::build()
}

/// The Apio and Logix Engine releases Logix uses are pinned in package.json
/// (`appConstants`; the Apio keys are the Apio VS Code extension's). Passes
/// them to the code as LOGIX_APIO_RELEASE_REPO / LOGIX_APIO_RELEASE_TAG (see
/// apio/mod.rs) and LOGIX_ENGINE_RELEASE_REPO / LOGIX_ENGINE_RELEASE_TAG
/// (see engine/mod.rs).
fn pinned_releases() {
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
    // Both end up in a download URL.
    let plain = |s: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    };
    for (repo_key, tag_key, env_prefix) in [
        ("apioCliReleaseRepo", "apioCliReleaseTag", "LOGIX_APIO"),
        ("engineReleaseRepo", "engineReleaseTag", "LOGIX_ENGINE"),
    ] {
        let repo = value(repo_key);
        let tag = value(tag_key);
        assert!(
            plain(&repo) && repo.split('/').count() == 2,
            "appConstants.{repo_key} must look like \"owner/repo\", not {repo:?}"
        );
        assert!(
            plain(&tag) && !tag.contains('/'),
            "bad appConstants.{tag_key} {tag:?}"
        );
        println!("cargo:rustc-env={env_prefix}_RELEASE_REPO={repo}");
        println!("cargo:rustc-env={env_prefix}_RELEASE_TAG={tag}");
    }
}
