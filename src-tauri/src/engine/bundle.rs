//! Unpacking and checking the engine package, a release file named by
//! Logix IDE's remote config (`remote_config.rs`): such as
//! `logix-engine-darwin-arm64-20261112.tgz`, with one top-level
//! `logix-engine/` folder holding the executable and its licenses. Next to
//! the unpacked executable, its receipt says what it is.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::download::{remove_dir_if_exists, unpack_tgz};
use crate::platform::Platform;
use crate::remote_config::Dependency;

/// Written next to the unpacked executable, last, so an interrupted unpack
/// is never taken for a complete one.
const RECEIPT_FILE: &str = "receipt.json";

/// How Logix IDE lays out the unpacked engine. Raise it only when that
/// changes, so older unpacks are downloaded again.
const LAYOUT: u32 = 1;

/// What was downloaded and unpacked (Remote updates, Receipts, in the
/// website repo's `CLAUDE.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub repo: String,
    pub release: String,
    pub file: String,
    pub url: String,
    pub platform: String,
    pub size: u64,
    pub sha256: String,
    /// The program and its version, for diagnostics only.
    pub downloaded_by: String,
    pub layout: u32,
}

impl Receipt {
    /// Whether it's the dependency the config names, for this platform and
    /// layout. Not the program's version: the SHA-256 proves the bytes.
    pub fn matches(&self, dep: &Dependency, platform: Platform) -> bool {
        self.url == dep.url
            && self.sha256 == dep.sha256
            && self.platform == platform.engine_platform_id()
            && self.layout == LAYOUT
    }
}

/// The receipt in `bin_dir`, if there's a readable one.
pub fn receipt(bin_dir: &Path) -> Option<Receipt> {
    let bytes = std::fs::read(bin_dir.join(RECEIPT_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// True if `bin_dir` holds a complete unpack of `dep`.
pub fn is_unpacked(bin_dir: &Path, platform: Platform, dep: &Dependency) -> bool {
    bin_dir.join(platform.engine_exe_name()).is_file()
        && receipt(bin_dir).is_some_and(|r| r.matches(dep, platform))
}

/// The SHA-256 of a file, lowercase hex. Blocking.
fn sha256_of(file: &Path) -> Result<String> {
    let mut f = std::fs::File::open(file)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Checks a downloaded package against its SHA-256 and unpacks it as
/// `bin_dir`, replacing the release there before. Returns the executable.
/// Blocking.
pub fn unpack(
    platform: Platform,
    archive: &Path,
    dep: &Dependency,
    tmp_dir: &Path,
    bin_dir: &Path,
) -> Result<PathBuf> {
    let sha256 = sha256_of(archive)?;
    if sha256 != dep.sha256 {
        bail!(
            "the downloaded engine file is damaged: its SHA-256 is {sha256}, not {} as the \
             release's SHA256SUMS says",
            dep.sha256
        );
    }
    let size = std::fs::metadata(archive)?.len();
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

    let receipt = Receipt {
        repo: dep.repo.clone(),
        release: dep.release.clone(),
        file: dep.file.clone(),
        url: dep.url.clone(),
        platform: platform.engine_platform_id().to_string(),
        size,
        sha256,
        downloaded_by: format!("logix-ide {}", crate::version::APP_VERSION),
        layout: LAYOUT,
    };
    std::fs::write(
        bin_dir.join(RECEIPT_FILE),
        serde_json::to_vec_pretty(&receipt)?,
    )?;

    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::fs::remove_file(archive);
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::make_tgz;

    /// A package with the real package's layout, and the dependency that
    /// names it.
    fn package(dir: &Path, entries: &[(&str, &[u8])]) -> (PathBuf, Dependency) {
        let file = "logix-engine-darwin-arm64-20261112.tgz";
        let archive = dir.join(file);
        make_tgz(&archive, entries);
        let dep = Dependency {
            repo: "openlogix/logix-engine".into(),
            release: "2026-11-12".into(),
            file: file.into(),
            url: format!(
                "https://github.com/openlogix/logix-engine/releases/download/2026-11-12/{file}"
            ),
            sha256: sha256_of(&archive).unwrap(),
        };
        (archive, dep)
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unpacks_package_and_detects_it() {
        let tmp = tempfile::tempdir().unwrap();
        let (archive, dep) = package(
            tmp.path(),
            &[
                ("logix-engine/logix-engine", b"#!/bin/sh\n"),
                ("logix-engine/LICENSE-MIT", b"MIT"),
                ("logix-engine/LICENSE-APACHE", b"Apache"),
            ],
        );
        let p = Platform::MacosArm64;
        let bin_dir = tmp.path().join("engine-release");
        assert!(!is_unpacked(&bin_dir, p, &dep));

        let exe = unpack(p, &archive, &dep, &tmp.path().join("tmp"), &bin_dir).unwrap();

        assert_eq!(exe, bin_dir.join("logix-engine"));
        assert!(bin_dir.join("LICENSE-MIT").is_file());
        assert!(!archive.exists());
        assert!(is_unpacked(&bin_dir, p, &dep));
        let r = receipt(&bin_dir).unwrap();
        assert_eq!(
            (r.release.as_str(), r.platform.as_str()),
            ("2026-11-12", "darwin-arm64")
        );
        assert!(r.size > 0 && r.downloaded_by.starts_with("logix-ide "));
        // Another release, or the same tag rebuilt, is downloaded again.
        let mut other = dep.clone();
        other.url = other.url.replace("2026-11-12", "2026-11-13");
        assert!(!is_unpacked(&bin_dir, p, &other));
        let mut rebuilt = dep.clone();
        rebuilt.sha256 = "00".repeat(32);
        assert!(!is_unpacked(&bin_dir, p, &rebuilt));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&exe).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0, "not executable");
    }

    #[test]
    fn rejects_a_damaged_download() {
        let tmp = tempfile::tempdir().unwrap();
        let (archive, mut dep) = package(tmp.path(), &[("logix-engine/logix-engine", b"x")]);
        dep.sha256 = "00".repeat(32);
        let bin_dir = tmp.path().join("engine-release");
        let err = unpack(Platform::MacosArm64, &archive, &dep, tmp.path(), &bin_dir).unwrap_err();
        assert!(err.to_string().contains("damaged"), "{err}");
        assert!(!bin_dir.exists());
    }

    #[test]
    fn rejects_archive_without_engine_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let (archive, dep) = package(tmp.path(), &[("apio/apio", b"x")]);
        let err = unpack(
            Platform::MacosArm64,
            &archive,
            &dep,
            &tmp.path().join("tmp"),
            &tmp.path().join("engine-release"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("doesn't contain"), "{err}");
    }
}
