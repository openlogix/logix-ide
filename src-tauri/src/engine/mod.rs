//! The Logix Engine adapter: the only place that runs the engine or knows
//! about its releases and file layout. The engine is an MCP server that the
//! IDE launches as a child process and talks to over stdin/stdout.
//!
//! Moving from Apio to the engine happens step by step: for now the IDE
//! downloads and starts the engine and installs its packages at startup,
//! while Verify, Upload and the other actions still use Apio.

mod bundle;
mod client;
mod setup;

use std::path::{Path, PathBuf};

pub use client::Engine;
pub use setup::run_setup;

/// The engine release the IDE uses: pinned in package.json (`appConstants`)
/// and passed in by build.rs. Change it there to move to a new release.
/// TODO(revisit): during development this is a daily pre-release. Before an
/// IDE release, pin a stable engine release and check the package's size
/// and SHA-256 (see CLAUDE.md, Open items).
pub const ENGINE_RELEASE_REPO: &str = env!("LOGIX_ENGINE_RELEASE_REPO");
pub const ENGINE_RELEASE_TAG: &str = env!("LOGIX_ENGINE_RELEASE_TAG");

/// Where the engine lives in the IDE's data folder (`~/.logix`).
struct Layout {
    data_dir: PathBuf,
}

impl Layout {
    fn new(data_dir: &Path) -> Self {
        Layout {
            data_dir: data_dir.to_path_buf(),
        }
    }
    /// The unpacked engine package (the executable and its LICENSE), plus
    /// the download record.
    fn bin_dir(&self) -> PathBuf {
        self.data_dir.join("engine-bin")
    }
    /// Scratch space for downloading and unpacking the engine package.
    fn tmp_dir(&self) -> PathBuf {
        self.data_dir.join("engine-tmp")
    }
    /// The engine folder (`--engine-folder`), owned by the engine: its
    /// packages (toolchains), logs and scratch files. The engine requires
    /// its last name to contain "engine".
    fn engine_folder(&self) -> PathBuf {
        self.data_dir.join("engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_release_comes_from_package_json() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../package.json"))
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            json["appConstants"]["engineReleaseRepo"].as_str(),
            Some(ENGINE_RELEASE_REPO)
        );
        assert_eq!(
            json["appConstants"]["engineReleaseTag"].as_str(),
            Some(ENGINE_RELEASE_TAG)
        );
    }

    #[test]
    fn engine_folder_name_contains_engine() {
        let layout = Layout::new(Path::new("/Users/you/.logix"));
        let folder = layout.engine_folder();
        assert_eq!(folder, Path::new("/Users/you/.logix/engine"));
        // The engine's path keyword rule.
        assert!(folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("engine"));
        assert_ne!(layout.bin_dir(), folder);
    }
}
