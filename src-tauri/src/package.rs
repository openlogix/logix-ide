//! Logix IDE's dependency packages, the release files its remote config
//! names (`remote_config.rs`): Logix Engine (`logix-engine-<platform>-<yyyymmdd>.tgz`,
//! one top-level `logix-engine/` folder with the executable and its
//! licenses) and the examples (`logix-ide-examples-<yyyymmdd>.tgz`, one
//! top-level `logix-ide-examples/` folder). Each is checked against its
//! SHA-256 and unpacked as a folder of its own, with its receipt, which
//! says what it is. Plain Rust, no Tauri.

use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::download::{remove_dir_if_exists, unpack_tgz};
use crate::platform::Platform;
use crate::remote_config::Dependency;

/// Written in the unpacked folder, last, so an interrupted unpack is never
/// taken for a complete one.
const RECEIPT_FILE: &str = "receipt.json";

/// How Logix IDE lays out its unpacked packages. Raise it only when that
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

/// The receipt in `dir`, if there's a readable one.
pub fn receipt(dir: &Path) -> Option<Receipt> {
    let bytes = std::fs::read(dir.join(RECEIPT_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// True if `dir` holds a complete unpack of `dep`, with `needed` (a path
/// inside it, such as the engine's executable) there.
pub fn is_unpacked(dir: &Path, platform: Platform, dep: &Dependency, needed: &str) -> bool {
    dir.join(needed).exists() && receipt(dir).is_some_and(|r| r.matches(dep, platform))
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

/// Checks a downloaded package against its SHA-256 and unpacks its one
/// top-level folder `top` as `dest`, replacing the release there before.
/// `needed` must be in it; `prepare` then readies the folder (such as
/// making the engine executable). Blocking.
#[allow(clippy::too_many_arguments)]
pub fn unpack(
    platform: Platform,
    archive: &Path,
    dep: &Dependency,
    top: &str,
    needed: &str,
    tmp_dir: &Path,
    dest: &Path,
    prepare: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let sha256 = sha256_of(archive)?;
    if sha256 != dep.sha256 {
        bail!(
            "the downloaded file {} is damaged: its SHA-256 is {sha256}, not {} as the \
             release's SHA256SUMS says",
            dep.file,
            dep.sha256
        );
    }
    let size = std::fs::metadata(archive)?.len();
    let extract_dir = tmp_dir.join("extract");
    remove_dir_if_exists(&extract_dir)?;
    unpack_tgz(archive, &extract_dir)
        .with_context(|| format!("the downloaded file {} is damaged", dep.file))?;

    let unpacked = extract_dir.join(top);
    if !unpacked.join(needed).exists() {
        bail!(
            "the downloaded file {} doesn't contain {top}/{needed}",
            dep.file
        );
    }

    remove_dir_if_exists(dest)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&unpacked, dest)
        .with_context(|| format!("can't move {top} into {}", dest.display()))?;
    prepare(dest)?;

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
        dest.join(RECEIPT_FILE),
        serde_json::to_vec_pretty(&receipt)?,
    )?;

    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::fs::remove_file(archive);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::make_tgz;

    /// A package with the real package's layout, and the dependency that
    /// names it.
    fn package(dir: &Path, entries: &[(&str, &[u8])]) -> (std::path::PathBuf, Dependency) {
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
        let exe = "logix-engine";
        assert!(!is_unpacked(&bin_dir, p, &dep, exe));

        unpack(
            p,
            &archive,
            &dep,
            "logix-engine",
            exe,
            &tmp.path().join("tmp"),
            &bin_dir,
            |d| p.prepare_unpacked_bundle(d, &d.join(exe)),
        )
        .unwrap();

        let exe_path = bin_dir.join(exe);
        assert!(bin_dir.join("LICENSE-MIT").is_file());
        assert!(!archive.exists());
        assert!(is_unpacked(&bin_dir, p, &dep, exe));
        let r = receipt(&bin_dir).unwrap();
        assert_eq!(
            (r.release.as_str(), r.platform.as_str()),
            ("2026-11-12", "darwin-arm64")
        );
        assert!(r.size > 0 && r.downloaded_by.starts_with("logix-ide "));
        // Another release, or the same tag rebuilt, is downloaded again.
        let mut other = dep.clone();
        other.url = other.url.replace("2026-11-12", "2026-11-13");
        assert!(!is_unpacked(&bin_dir, p, &other, exe));
        let mut rebuilt = dep.clone();
        rebuilt.sha256 = "00".repeat(32);
        assert!(!is_unpacked(&bin_dir, p, &rebuilt, exe));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&exe_path).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0, "not executable");
    }

    #[test]
    fn rejects_a_damaged_download() {
        let tmp = tempfile::tempdir().unwrap();
        let (archive, mut dep) = package(tmp.path(), &[("logix-engine/logix-engine", b"x")]);
        dep.sha256 = "00".repeat(32);
        let bin_dir = tmp.path().join("engine-release");
        let err = unpack(
            Platform::MacosArm64,
            &archive,
            &dep,
            "logix-engine",
            "logix-engine",
            tmp.path(),
            &bin_dir,
            |_| Ok(()),
        )
        .unwrap_err();
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
            "logix-engine",
            "logix-engine",
            &tmp.path().join("tmp"),
            &tmp.path().join("engine-release"),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(err.to_string().contains("doesn't contain"), "{err}");
    }
}
