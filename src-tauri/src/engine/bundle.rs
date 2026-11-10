//! Downloading and unpacking the engine package from the engine's GitHub
//! releases: `logix-engine-<platform>-<yyyymmdd>.tgz`, with one top-level
//! `logix-engine/` folder holding the executable and its licenses.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::download::{remove_dir_if_exists, unpack_tgz};
use crate::platform::Platform;

/// Written next to the unpacked executable; records which release URL the
/// package came from so a changed pinned release triggers a new download.
const METADATA_FILE: &str = "download-metadata.json";

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    url: String,
}

/// What to download for a given platform and engine release.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageSpec {
    pub url: String,
    pub file_name: String,
}

/// E.g. `.../releases/download/2026-11-03/logix-engine-darwin-arm64-20261103.tgz`.
/// Every platform's package is a `.tgz`.
pub fn package_spec(platform: Platform, repo: &str, tag: &str) -> PackageSpec {
    let yyyymmdd = tag.replace('-', "");
    let file_name = format!(
        "logix-engine-{}-{yyyymmdd}.tgz",
        platform.engine_platform_id()
    );
    PackageSpec {
        url: format!("https://github.com/{repo}/releases/download/{tag}/{file_name}"),
        file_name,
    }
}

/// True if `bin_dir` holds a complete unpack that came from `url`.
pub fn is_unpacked(bin_dir: &Path, exe_name: &str, url: &str) -> bool {
    if !bin_dir.join(exe_name).is_file() {
        return false;
    }
    std::fs::read(bin_dir.join(METADATA_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<Metadata>(&b).ok())
        .is_some_and(|m| m.url == url)
}

/// Unpacks a downloaded package as `bin_dir`, replacing any previous
/// one. Returns the executable. Blocking.
pub fn unpack(
    platform: Platform,
    archive: &Path,
    spec: &PackageSpec,
    tmp_dir: &Path,
    bin_dir: &Path,
) -> Result<PathBuf> {
    let extract_dir = tmp_dir.join("extract");
    remove_dir_if_exists(&extract_dir)?;
    unpack_tgz(archive, &extract_dir).context("the downloaded engine file is damaged")?;

    let unpacked = extract_dir.join("logix-engine");
    let exe_name = platform.engine_exe_name();
    if !unpacked.join(exe_name).is_file() {
        bail!("the downloaded engine file doesn't contain logix-engine/{exe_name}");
    }

    remove_dir_if_exists(bin_dir)?;
    if let Some(parent) = bin_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&unpacked, bin_dir)
        .with_context(|| format!("can't move the engine into {}", bin_dir.display()))?;
    let exe = bin_dir.join(exe_name);
    platform.prepare_unpacked_bundle(bin_dir, &exe)?;

    // Written last, so an interrupted unpack is never treated as complete.
    let metadata = serde_json::to_vec_pretty(&Metadata {
        url: spec.url.clone(),
    })?;
    std::fs::write(bin_dir.join(METADATA_FILE), metadata)?;

    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::fs::remove_file(archive);
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::make_tgz;

    #[test]
    fn builds_release_url() {
        let spec = package_spec(Platform::MacosArm64, "openlogix/logix-engine", "2026-11-03");
        assert_eq!(
            spec.url,
            "https://github.com/openlogix/logix-engine/releases/download/2026-11-03/\
             logix-engine-darwin-arm64-20261103.tgz"
        );
        let linux = package_spec(
            Platform::LinuxX86_64,
            "openlogix/logix-engine",
            "2026-11-03",
        );
        assert_eq!(linux.file_name, "logix-engine-linux-x86-64-20261103.tgz");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unpacks_package_and_detects_it() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = package_spec(Platform::MacosArm64, "openlogix/logix-engine", "2026-11-03");
        let archive = tmp.path().join(&spec.file_name);
        // The layout of the real package.
        make_tgz(
            &archive,
            &[
                ("logix-engine/logix-engine", b"#!/bin/sh\n"),
                ("logix-engine/LICENSE-MIT", b"MIT"),
                ("logix-engine/LICENSE-APACHE", b"Apache"),
            ],
        );
        let bin_dir = tmp.path().join("engine-bin");
        assert!(!is_unpacked(&bin_dir, "logix-engine", &spec.url));

        let exe = unpack(
            Platform::MacosArm64,
            &archive,
            &spec,
            &tmp.path().join("engine-tmp"),
            &bin_dir,
        )
        .unwrap();

        assert_eq!(exe, bin_dir.join("logix-engine"));
        assert!(bin_dir.join("LICENSE-MIT").is_file());
        assert!(bin_dir.join("LICENSE-APACHE").is_file());
        assert!(!archive.exists());
        assert!(is_unpacked(&bin_dir, "logix-engine", &spec.url));
        assert!(!is_unpacked(&bin_dir, "logix-engine", "https://other/url"));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&exe).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0, "not executable");
    }

    #[test]
    fn rejects_archive_without_engine_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = package_spec(Platform::MacosArm64, "openlogix/logix-engine", "2026-11-03");
        let archive = tmp.path().join(&spec.file_name);
        make_tgz(&archive, &[("apio/apio", b"x")]);
        let err = unpack(
            Platform::MacosArm64,
            &archive,
            &spec,
            &tmp.path().join("engine-tmp"),
            &tmp.path().join("engine-bin"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("doesn't contain"), "{err}");
    }
}
