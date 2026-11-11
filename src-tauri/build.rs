fn main() {
    // The app icons are built into the binary (tauri::generate_context!),
    // so rebuild when they change.
    println!("cargo:rerun-if-changed=icons");
    // So are the examples (include_dir! in examples.rs).
    println!("cargo:rerun-if-changed=../examples");
    pinned_releases();
    tauri_build::build()
}

/// The Logix Engine release Logix IDE uses is pinned in package.json
/// (`appConstants`). Passes it to the code as LOGIX_IDE_BUILD_ENGINE_REPO /
/// LOGIX_IDE_BUILD_ENGINE_TAG (see engine/mod.rs).
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
    // They end up in a download URL.
    let plain = |s: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    };
    let repo = value("engineReleaseRepo");
    let tag = value("engineReleaseTag");
    assert!(
        plain(&repo) && repo.split('/').count() == 2,
        "appConstants.engineReleaseRepo must look like \"owner/repo\", not {repo:?}"
    );
    assert!(
        plain(&tag) && !tag.contains('/'),
        "bad appConstants.engineReleaseTag {tag:?}"
    );
    println!("cargo:rustc-env=LOGIX_IDE_BUILD_ENGINE_REPO={repo}");
    println!("cargo:rustc-env=LOGIX_IDE_BUILD_ENGINE_TAG={tag}");
}
