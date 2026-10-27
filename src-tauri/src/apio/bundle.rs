//! Downloading and installing Apio's standalone (PyInstaller) bundle from
//! Apio's GitHub releases, the same way the Apio VS Code extension does.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::platform::Platform;

/// Written next to the installed executable; records which release URL the
/// bundle came from so a changed pinned version triggers a new download.
const METADATA_FILE: &str = "download-metadata.json";

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    url: String,
}

/// What to download for a given platform and Apio release.
#[derive(Debug, PartialEq, Eq)]
pub struct BundleSpec {
    pub url: String,
    pub file_name: String,
    pub ext: &'static str,
}

/// E.g. `.../releases/download/2026-10-22/apio-cli-darwin-arm64-20261022-bundle.tgz`.
pub fn bundle_spec(platform: Platform, repo: &str, tag: &str) -> BundleSpec {
    let ext = platform.apio_bundle_ext();
    let yyyymmdd = tag.replace('-', "");
    let file_name = format!(
        "apio-cli-{}-{yyyymmdd}-bundle.{ext}",
        platform.apio_platform_id()
    );
    BundleSpec {
        url: format!("https://github.com/{repo}/releases/download/{tag}/{file_name}"),
        file_name,
        ext,
    }
}

/// True if `bundle_dir` holds a complete install that came from `url`.
pub fn is_installed(bundle_dir: &Path, exe_name: &str, url: &str) -> bool {
    if !bundle_dir.join(exe_name).is_file() {
        return false;
    }
    std::fs::read(bundle_dir.join(METADATA_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<Metadata>(&b).ok())
        .is_some_and(|m| m.url == url)
}

/// Downloads `url` to `dest`, reporting `(bytes_received, total_if_known)`.
pub async fn download<F>(url: &str, dest: &Path, mut progress: F) -> Result<()>
where
    F: FnMut(u64, Option<u64>),
{
    let client = reqwest::Client::builder()
        .user_agent(concat!("logix-ide/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(60))
        .build()?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("can't connect to the download server (check the internet connection)")?
        .error_for_status()
        .context("the download server returned an error")?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(dest)
        .await
        .with_context(|| format!("can't create {}", dest.display()))?;
    let mut stream = resp.bytes_stream();
    let mut received = 0u64;
    let mut last_report = Instant::now();
    progress(0, total);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("the download was interrupted")?;
        file.write_all(&chunk).await?;
        received += chunk.len() as u64;
        if last_report.elapsed() >= Duration::from_millis(100) {
            progress(received, total);
            last_report = Instant::now();
        }
    }
    file.flush().await?;
    progress(received, total);
    Ok(())
}

/// Unpacks a downloaded bundle archive and installs it as `bundle_dir`,
/// replacing any previous install. Blocking; run off the async runtime.
pub fn install(
    platform: Platform,
    archive: &Path,
    spec: &BundleSpec,
    tmp_dir: &Path,
    bundle_dir: &Path,
) -> Result<PathBuf> {
    let extract_dir = tmp_dir.join("extract");
    remove_dir_if_exists(&extract_dir)?;
    std::fs::create_dir_all(&extract_dir)?;

    match spec.ext {
        "tgz" => {
            let file = std::fs::File::open(archive)?;
            tar::Archive::new(flate2::read::GzDecoder::new(file))
                .unpack(&extract_dir)
                .context("the downloaded Apio file is damaged")?;
        }
        other => bail!("unpacking .{other} files is not supported yet"),
    }

    // The archive contains a single top-level `apio/` directory.
    let unpacked = extract_dir.join("apio");
    let exe_name = platform.apio_exe_name();
    if !unpacked.join(exe_name).is_file() {
        bail!("the downloaded Apio file doesn't contain apio/{exe_name}");
    }

    remove_dir_if_exists(bundle_dir)?;
    if let Some(parent) = bundle_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&unpacked, bundle_dir)
        .with_context(|| format!("can't move Apio into {}", bundle_dir.display()))?;
    let exe = bundle_dir.join(exe_name);
    platform.prepare_unpacked_bundle(bundle_dir, &exe)?;

    // Written last, so an interrupted install is never treated as complete.
    let metadata = serde_json::to_vec_pretty(&Metadata {
        url: spec.url.clone(),
    })?;
    std::fs::write(bundle_dir.join(METADATA_FILE), metadata)?;

    let _ = std::fs::remove_dir_all(&extract_dir);
    let _ = std::fs::remove_file(archive);
    Ok(exe)
}

fn remove_dir_if_exists(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).with_context(|| format!("can't remove {}", dir.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_release_url() {
        let spec = bundle_spec(Platform::MacosArm64, "fpgawars/apio", "2026-10-22");
        assert_eq!(
            spec.url,
            "https://github.com/fpgawars/apio/releases/download/2026-10-22/\
             apio-cli-darwin-arm64-20261022-bundle.tgz"
        );
        assert_eq!(spec.ext, "tgz");
        let win = bundle_spec(Platform::WindowsX86_64, "fpgawars/apio", "2026-10-22");
        assert_eq!(win.file_name, "apio-cli-windows-amd64-20261022-bundle.zip");
    }

    /// Writes a .tgz containing the given (path, contents) entries.
    fn make_tgz(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn installs_bundle_and_detects_it() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = bundle_spec(Platform::MacosArm64, "fpgawars/apio", "2026-10-22");
        let archive = tmp.path().join(&spec.file_name);
        make_tgz(
            &archive,
            &[("apio/apio", b"#!/bin/sh\n"), ("apio/_internal/x", b"x")],
        );
        let bundle_dir = tmp.path().join("bundle");
        assert!(!is_installed(&bundle_dir, "apio", &spec.url));

        let exe = install(
            Platform::MacosArm64,
            &archive,
            &spec,
            &tmp.path().join("tmp"),
            &bundle_dir,
        )
        .unwrap();

        assert_eq!(exe, bundle_dir.join("apio"));
        assert!(bundle_dir.join("_internal/x").is_file());
        assert!(!archive.exists());
        assert!(is_installed(&bundle_dir, "apio", &spec.url));
        assert!(!is_installed(&bundle_dir, "apio", "https://other/url"));
    }

    #[test]
    fn rejects_archive_without_apio_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = bundle_spec(Platform::MacosArm64, "fpgawars/apio", "2026-10-22");
        let archive = tmp.path().join(&spec.file_name);
        make_tgz(&archive, &[("other/file", b"x")]);
        let err = install(
            Platform::MacosArm64,
            &archive,
            &spec,
            &tmp.path().join("tmp"),
            &tmp.path().join("bundle"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("doesn't contain"));
    }
}
