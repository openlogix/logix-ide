//! The Apio adapter: the only place that runs Apio or knows about its
//! releases, file layout and output. Everything else talks to Apio through
//! this module.

mod board_match;
mod bundle;
mod catalog;
mod devices;
mod diagnostics;
mod language_server;
mod process;
mod report;
mod setup;
mod task;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};

use anyhow::{bail, Result};
use tokio_util::sync::CancellationToken;

pub use board_match::{match_boards, ConnectedBoard, UsbRule};
pub use catalog::{starter_example, Board, ExampleList, DEFAULT_BOARD};
pub use devices::Devices;
pub use diagnostics::Diagnostic;
pub use language_server::LanguageServer;
pub use process::{quietly, strip_ansi};
pub use setup::{run_setup, SetupEvent, SetupStage};
pub use task::{Task, TaskOutcome};

/// The Apio release the IDE uses: pinned in package.json (`appConstants`,
/// as in the Apio VS Code extension) and passed in by build.rs. Change it
/// there to move to a new Apio version.
pub const APIO_RELEASE_REPO: &str = env!("LOGIX_APIO_RELEASE_REPO");
pub const APIO_RELEASE_TAG: &str = env!("LOGIX_APIO_RELEASE_TAG");

/// Developer-only override: path to a local `apio` executable to use instead
/// of the downloaded bundle (e.g. to test `apio api` changes). Unset for users.
pub const DEV_APIO_BIN_ENV: &str = "LOGIX_APIO_BIN";

/// A ready-to-use Apio installation.
#[derive(Debug, Clone)]
pub struct Apio {
    exe: PathBuf,
    home: PathBuf,
    /// Scratch space for Apio's JSON output and fetched examples.
    tmp: PathBuf,
}

impl Apio {
    /// Runs a primary Apio command (build, upload, lint, test, sim, format,
    /// examples fetch, ...) with `args` (in `dir`, if given). It's shown in
    /// the console: `on_line` first gets the command line (dimmed), then
    /// each line of Apio's output.
    pub async fn run<F>(
        &self,
        args: &[OsString],
        dir: Option<&Path>,
        on_line: F,
    ) -> Result<ExitStatus>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        self.run_cancellable(args, dir, on_line, &CancellationToken::new())
            .await
    }

    /// Like `run`, but stops Apio (and what it started) when `cancel` is
    /// triggered (the Stop button).
    pub async fn run_cancellable<F>(
        &self,
        args: &[OsString],
        dir: Option<&Path>,
        on_line: F,
        cancel: &CancellationToken,
    ) -> Result<ExitStatus>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        on_line(format!("\x1b[2m$ {}\x1b[0m", command_line(args)));
        let mut cmd = self.command();
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }
        cmd.args(args);
        process::run_streaming(cmd, on_line, cancel).await
    }

    /// Like `run`, but without showing the command line: only Apio's
    /// output goes to `on_line`. For info commands whose output still
    /// matters to the user (e.g. package installation during setup).
    pub async fn run_output_only<F>(
        &self,
        args: &[OsString],
        dir: Option<&Path>,
        on_line: F,
    ) -> Result<ExitStatus>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        let mut cmd = self.command();
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }
        cmd.args(args);
        process::run_streaming(cmd, on_line, &CancellationToken::new()).await
    }

    /// Runs an info command (`apio api ...`) without showing anything in
    /// the console. Fails if Apio does, with its output (colors removed) in
    /// the error, since it isn't in the console.
    pub async fn run_silent(&self, args: &[OsString], dir: Option<&Path>) -> Result<()> {
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = lines.clone();
        let status = self
            .run_output_only(args, dir, move |line| sink.lock().unwrap().push(line))
            .await?;
        if !status.success() {
            let output = lines.lock().unwrap().join("\n");
            anyhow::bail!(
                "{} failed ({status}): {}",
                command_line(args),
                process::strip_ansi(&output).trim()
            );
        }
        Ok(())
    }

    /// A command that runs Apio with our private `APIO_HOME`. Use `run`,
    /// `run_output_only` or `run_silent` instead, which decide what the
    /// console shows.
    fn command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.exe);
        // Don't let the user's own Apio settings (e.g. APIO_PACKAGES,
        // APIO_REMOTE_CONFIG_URL) leak into the IDE's Apio. Developers using
        // the local Apio override keep them.
        if std::env::var_os(DEV_APIO_BIN_ENV).is_none() {
            for key in apio_vars(std::env::vars_os().map(|(k, _)| k)) {
                cmd.env_remove(key);
            }
        }
        // Ask for colored output, as in a terminal (Apio's output library
        // only colors terminal output by default), with a fixed color
        // mode and a line width that doesn't wrap most messages. The
        // console shows the colors; parsers strip them (`strip_ansi`).
        cmd.env_remove("NO_COLOR")
            .env("FORCE_COLOR", "1")
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "120");
        cmd.env("APIO_HOME", &self.home)
            .stdin(Stdio::null())
            .kill_on_drop(true);
        cmd
    }
}

/// Converts command arguments for `Apio::run`.
pub fn args<I, S>(items: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    items.into_iter().map(Into::into).collect()
}

/// How a command is shown in the console, e.g. `apio examples fetch a/b`.
/// Arguments with spaces are quoted.
fn command_line(args: &[OsString]) -> String {
    let mut line = String::from("apio");
    for arg in args {
        let arg = arg.to_string_lossy();
        line.push(' ');
        if arg.is_empty() || arg.contains(char::is_whitespace) {
            line.push_str(&format!("\"{arg}\""));
        } else {
            line.push_str(&arg);
        }
    }
    line
}

/// The `APIO_*` names among the given environment variable names
/// (case-insensitive, as on Windows).
fn apio_vars(keys: impl Iterator<Item = OsString>) -> Vec<OsString> {
    keys.filter(|k| {
        k.to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("APIO_")
    })
    .collect()
}

/// Where the IDE keeps Apio files, under the per-user data dir.
struct Layout {
    root: PathBuf,
}

impl Layout {
    fn new(data_dir: &Path) -> Self {
        Layout {
            root: data_dir.join("apio"),
        }
    }
    /// The unpacked Apio bundle (the `apio` executable and its support files).
    fn bundle_dir(&self) -> PathBuf {
        self.root.join("bundle")
    }
    /// Apio's `APIO_HOME`: its packages (toolchains), caches and settings.
    fn home_dir(&self) -> PathBuf {
        self.root.join("home")
    }
    /// Scratch space for downloads and unpacking.
    fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }
}

/// Apio only accepts an `APIO_HOME` made of printable ASCII without spaces.
fn validate_apio_home(path: &Path) -> Result<()> {
    let s = path.to_string_lossy();
    if let Some(bad) = s.chars().find(|c| !c.is_ascii_graphic()) {
        bail!(
            "the folder {s} contains the character {bad:?}; Apio needs a path \
             of plain ASCII characters without spaces"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn apio_release_comes_from_package_json() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../package.json"))
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            json["appConstants"]["apioCliReleaseRepo"].as_str(),
            Some(APIO_RELEASE_REPO)
        );
        assert_eq!(
            json["appConstants"]["apioCliReleaseTag"].as_str(),
            Some(APIO_RELEASE_TAG)
        );
    }

    use super::*;

    #[test]
    fn shows_command_lines() {
        assert_eq!(command_line(&args(["build"])), "apio build");
        assert_eq!(
            command_line(&args(["sim", "main_tb.v", "--detach"])),
            "apio sim main_tb.v --detach"
        );
        assert_eq!(
            command_line(&args(["examples", "fetch", "a/b", "-d", "/x/my dir"])),
            "apio examples fetch a/b -d \"/x/my dir\""
        );
    }

    #[test]
    fn finds_apio_vars() {
        let keys = [
            "PATH",
            "APIO_HOME",
            "apio_debug",
            "MY_APIO_X",
            "APIO_REMOTE_CONFIG_URL",
        ]
        .map(OsString::from);
        assert_eq!(
            apio_vars(keys.into_iter()),
            vec![
                OsString::from("APIO_HOME"),
                OsString::from("apio_debug"),
                OsString::from("APIO_REMOTE_CONFIG_URL"),
            ]
        );
    }

    #[test]
    fn apio_home_must_be_plain_ascii() {
        assert!(validate_apio_home(Path::new("/Users/ann/.logix/apio/home")).is_ok());
        assert!(validate_apio_home(Path::new("/Users/ann b/.logix")).is_err());
        assert!(validate_apio_home(Path::new("/Users/zoë/.logix")).is_err());
    }
}
