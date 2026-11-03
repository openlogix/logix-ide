//! The Logix Engine adapter: the only place that runs the engine or knows
//! about its releases and file layout. The engine is an MCP server that the
//! IDE launches as a child process and talks to over stdin/stdout.
//!
//! Moving from Apio to the engine happens step by step: the IDE sets up
//! the engine at startup, and the board list, Verify and Upload use it;
//! Run Tests, Simulate, Show Diagram, Auto Format, Connected Devices and the
//! examples still use Apio.

mod bundle;
mod catalog;
mod client;
mod setup;
mod task;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use catalog::{boards, Board};
pub use client::Engine;
pub use setup::run_setup;
pub use task::{run as run_task, BuildInput};

use client::{Progress, ProgressSink};

/// Follows an operation's phases: `f(message, percent, new_phase)` gets
/// every progress notification with the current phase's message (the
/// engine sends the message only when a phase starts).
fn phase_sink(f: impl Fn(&str, Option<u32>, bool) + Send + Sync + 'static) -> ProgressSink {
    let current = Mutex::new(String::new());
    Arc::new(move |p: Progress| {
        let mut current = current.lock().unwrap();
        let mut new_phase = false;
        if let Some(message) = p.message {
            if *current != message {
                *current = message;
                new_phase = true;
            }
        }
        f(&current, p.percent, new_phase);
    })
}

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
