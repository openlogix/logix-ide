//! The Logix Engine adapter: the only place that runs the engine or knows
//! about its releases and file layout. The engine is an MCP server that the
//! IDE launches as a child process and talks to over stdin/stdout.
//! Every FPGA operation goes through it: the board list, Verify, Upload,
//! Run Tests, Simulate, Show Diagram, Auto Format, the connected devices
//! and the language server's executable.

mod catalog;
mod client;
mod devices;
mod format;
mod info;
mod setup;
mod task;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use catalog::{
    boards_fields, catalog, compare_boards, project_boards, Board, BoardComparison, BoardsFolder,
    BoardsProblem, Catalog, FpgaFamily, Variant,
};
pub use client::Engine;
pub use devices::{
    connected_devices, install_driver, list_devices, uninstall_driver, ConnectedDevice, DeviceList,
    DriverInstall, UnreachableDevice, UsbDevice,
};
pub use format::format_file;
pub use info::program_path;
pub use setup::{check_for_updates, download_missing, start as start_engine, update_if_due};
pub use task::{boards_problem_outcome, run as run_task, TaskInput};

use client::{Progress, ProgressSink};

use crate::remote_config::ConfigSource;

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

/// Where the engine lives in the IDE's data folder (`~/.logix/ide`). With
/// the remote config override (`LOGIX_IDE_DEV_REMOTE_CONFIG`) everything a
/// config decides (its cache, the engine release and the engine folder with
/// its packages) is kept in `dev/config-override/` instead, so testing a
/// config never changes what the regular Logix IDE uses.
struct Layout {
    data_dir: PathBuf,
    /// The folder for what the remote config decides: the data folder, or
    /// `dev/config-override/` with the override.
    config_dir: PathBuf,
}

impl Layout {
    fn new(data_dir: &Path, source: &ConfigSource) -> Self {
        let config_dir = if source.is_override {
            data_dir.join("dev").join("config-override")
        } else {
            data_dir.to_path_buf()
        };
        Layout {
            data_dir: data_dir.to_path_buf(),
            config_dir,
        }
    }
    /// The engine releases, one folder each (`releases_dir/<release>/`).
    fn releases_dir(&self) -> PathBuf {
        self.config_dir.join("engine-release")
    }
    /// The unpacked engine package of `release` (the executable and its
    /// licenses), plus its receipt. A folder per release, so a new release
    /// never touches the files of the one running: Windows locks a running
    /// program's files until it exits.
    fn bin_dir(&self, release: &str) -> PathBuf {
        self.releases_dir().join(release)
    }
    /// Scratch space for downloading and unpacking the packages.
    fn tmp_dir(&self) -> PathBuf {
        self.config_dir.join("_scratch").join("download")
    }
    /// The unpacked examples package, with its receipt.
    fn examples_dir(&self) -> PathBuf {
        self.config_dir.join("examples")
    }
    /// The engine folder (`--engine-folder`), owned by the engine: its
    /// packages (toolchains), logs and scratch files. The engine requires
    /// its last name to contain "engine".
    fn engine_folder(&self) -> PathBuf {
        self.config_dir.join("engine")
    }
    /// The engine folder of a locally built engine (`LOGIX_DEV_ENGINE_BINARY`),
    /// kept apart so it never touches the released engine's packages.
    fn dev_engine_folder(&self) -> PathBuf {
        self.data_dir.join("dev").join("engine")
    }
}

/// What this run uses, for System Info and About: the engine, the examples
/// and the remote config that named them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InUse {
    pub engine: EngineInUse,
    /// The examples' release (`yyyy-mm-dd`).
    pub examples: String,
    pub config: ConfigInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineInUse {
    /// The release the remote config names (`yyyy-mm-dd`).
    Release(String),
    /// A locally built engine (`LOGIX_DEV_ENGINE_BINARY`).
    Local(PathBuf),
}

/// The remote config in use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInfo {
    pub source: String,
    pub is_override: bool,
    /// The channel and the config file read.
    pub channel: crate::remote_config::Channel,
    /// When it was read (seconds since 1970, UTC).
    pub read_at: u64,
}

static IN_USE: Mutex<Option<InUse>> = Mutex::new(None);

/// The engine this run uses, once setup has chosen it.
pub fn in_use() -> Option<InUse> {
    IN_USE.lock().unwrap().clone()
}

fn set_in_use(value: InUse) {
    *IN_USE.lock().unwrap() = Some(value);
}

/// The environment variable naming a locally built engine binary, for
/// developing the IDE and the engine together (`scripts/run.sh
/// --engine-repo-dir` sets it).
pub const DEV_ENGINE_VARIABLE: &str = "LOGIX_DEV_ENGINE_BINARY";

/// The locally built engine to use instead of the release the remote config names: the
/// value of `LOGIX_DEV_ENGINE_BINARY`, in every build (development and
/// released), for Logix IDE developers and users guided by them, such as to
/// diagnose a GitHub-built IDE. Not for normal use. Unset or empty means
/// none; a value that isn't a file is an error.
pub(crate) fn dev_engine_binary(
    value: Option<std::ffi::OsString>,
) -> anyhow::Result<Option<PathBuf>> {
    match value {
        Some(v) if !v.is_empty() => {
            let path = PathBuf::from(v);
            if !path.is_file() {
                anyhow::bail!(
                    "{DEV_ENGINE_VARIABLE} must be the path to a logix-engine binary, not {}",
                    path.display()
                );
            }
            Ok(Some(path))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_folder_name_contains_engine() {
        let layout = Layout::new(Path::new("/Users/you/.logix/ide"), &ConfigSource::regular());
        let folder = layout.engine_folder();
        assert_eq!(folder, Path::new("/Users/you/.logix/ide/engine"));
        // The engine's path keyword rule.
        assert!(folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("engine"));
        assert!(!layout.bin_dir("2026-11-12").starts_with(&folder));
        let dev = layout.dev_engine_folder();
        assert_eq!(dev, Path::new("/Users/you/.logix/ide/dev/engine"));
        assert!(dev
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("engine"));
    }

    #[test]
    fn a_config_override_keeps_to_its_own_folder() {
        let source = ConfigSource {
            location: "https://example/config.jsonc".into(),
            is_override: true,
        };
        let layout = Layout::new(Path::new("/Users/you/.logix/ide"), &source);
        let dir = Path::new("/Users/you/.logix/ide/dev/config-override");
        assert_eq!(layout.engine_folder(), dir.join("engine"));
        assert_eq!(
            layout.bin_dir("2026-11-12"),
            dir.join("engine-release").join("2026-11-12")
        );
        assert!(layout.tmp_dir().starts_with(dir));
        assert_eq!(layout.config_dir, dir);
    }

    #[test]
    fn a_dev_engine_binary_must_be_a_file() {
        assert!(dev_engine_binary(None).unwrap().is_none());
        assert!(dev_engine_binary(Some("".into())).unwrap().is_none());
        let tmp = tempfile::tempdir().unwrap();
        assert!(
            dev_engine_binary(Some(tmp.path().into())).is_err(),
            "a folder"
        );
        let exe = tmp.path().join("logix-engine");
        std::fs::write(&exe, "").unwrap();
        assert_eq!(
            dev_engine_binary(Some(exe.clone().into())).unwrap(),
            Some(exe)
        );
    }
}
