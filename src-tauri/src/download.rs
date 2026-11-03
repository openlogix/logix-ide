//! Downloading and unpacking release files (the Logix Engine package).

use std::path::Path;
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

/// Unpacks a `.tgz` into `dir` (created if needed). Blocking.
pub fn unpack_tgz(archive: &Path, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let file = std::fs::File::open(archive)?;
    tar::Archive::new(flate2::read::GzDecoder::new(file)).unpack(dir)?;
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
        unpack_tgz(&archive, &tmp.path().join("out")).unwrap();
        assert_eq!(
            std::fs::read(tmp.path().join("out/top/file")).unwrap(),
            b"hello"
        );
    }
}
