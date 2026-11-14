//! Downloading and unpacking release files (the Logix Engine package).

use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

/// Downloads `url` to `dest`, reporting `(bytes_received, total_if_known)`.
pub async fn download<F>(url: &str, dest: &Path, mut progress: F) -> Result<()>
where
    F: FnMut(u64, Option<u64>),
{
    let client = reqwest::Client::builder()
        .user_agent(concat!("logix-ide/", env!("LOGIX_IDE_BUILD_VERSION")))
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

/// A package's progress is 60% downloading and 40% unpacking, the same in
/// every Open Logix program (the org's rule, 2026-11-14: website repo's
/// `CLAUDE.md`, Download progress; Logix Engine's `DOWNLOAD_SHARE`).
pub const DOWNLOAD_SHARE: f64 = 60.0;

/// A package's percentage while downloading: 0 to 60, or None when the
/// download's size is unknown.
pub fn download_percent(received: u64, total: Option<u64>) -> Option<u32> {
    let total = total.filter(|&t| t > 0)?;
    Some((DOWNLOAD_SHARE * received.min(total) as f64 / total as f64) as u32)
}

/// A package's percentage while unpacking: 60 to 100, by how much of the
/// downloaded file (`size` bytes) has been read.
pub fn unpack_percent(read: u64, size: u64) -> u32 {
    let done = read.min(size) as f64 / size.max(1) as f64;
    (DOWNLOAD_SHARE + (100.0 - DOWNLOAD_SHARE) * done) as u32
}

/// Counts the bytes read through it, for unpacking's progress.
struct Counting<'a, R> {
    inner: R,
    count: &'a AtomicU64,
}

impl<R: Read> Read for Counting<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

/// Unpacks a `.tgz` into `dir` (created if needed), adding the bytes of the
/// archive it has read to `read` as it goes. Blocking.
pub fn unpack_tgz(archive: &Path, dir: &Path, read: &AtomicU64) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let file = std::fs::File::open(archive)?;
    let counting = Counting {
        inner: std::io::BufReader::new(file),
        count: read,
    };
    tar::Archive::new(flate2::read::GzDecoder::new(counting)).unpack(dir)?;
    Ok(())
}

/// Removes `dir` and its contents if it exists.
pub fn remove_dir_if_exists(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).with_context(|| format!("can't remove {}", dir.display()))?;
    }
    Ok(())
}

/// Writes a .tgz containing the given (path, contents) entries, for tests.
#[cfg(test)]
pub fn make_tgz(path: &Path, entries: &[(&str, &[u8])]) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_tgz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("a.tgz");
        make_tgz(&archive, &[("top/file", b"hello")]);
        let read = AtomicU64::new(0);
        unpack_tgz(&archive, &tmp.path().join("out"), &read).unwrap();
        assert_eq!(
            std::fs::read(tmp.path().join("out/top/file")).unwrap(),
            b"hello"
        );
        // It read the whole archive, so unpacking reports 100%.
        let size = std::fs::metadata(&archive).unwrap().len();
        assert_eq!(read.load(Ordering::Relaxed), size);
        assert_eq!(unpack_percent(read.load(Ordering::Relaxed), size), 100);
    }

    #[test]
    fn downloading_is_60_percent_and_unpacking_40() {
        assert_eq!(download_percent(0, Some(1000)), Some(0));
        assert_eq!(download_percent(500, Some(1000)), Some(30));
        assert_eq!(download_percent(1000, Some(1000)), Some(60));
        assert_eq!(download_percent(500, None), None);
        assert_eq!(unpack_percent(0, 1000), 60);
        assert_eq!(unpack_percent(500, 1000), 80);
        assert_eq!(unpack_percent(1000, 1000), 100);
    }
}
