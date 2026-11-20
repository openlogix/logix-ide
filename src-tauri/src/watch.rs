//! Watches the open project's folder for changes other programs make (an AI
//! agent, another editor), so Logix IDE shows them without asking (the
//! user's decision, 2026-11-20; `spec/design/ai-side-by-side.md`). The
//! operating system's file notifications (`notify`: FSEvents, inotify,
//! ReadDirectoryChangesW), not polling. Only the folder's top level, where a
//! project's files are; only project files and the project file
//! (`logix-ide.toml`): generated folders (`_build`), hidden files and other
//! files are ignored. The UI reads the files and decides what changed: its
//! own saves come back too, and are no change there.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::project;

/// How long changes are gathered before they're reported: one save is
/// often several events (write, rename into place, metadata).
const GATHER: Duration = Duration::from_millis(100);

/// A running watch; dropping it stops it.
pub struct ProjectWatch {
    _watcher: RecommendedWatcher,
}

/// The names worth reporting among changed `paths`: project files (tabs and
/// GTKWave layouts) and the project file, directly in `dir`; sorted, each
/// once.
pub fn changed_names(dir: &Path, paths: &[PathBuf]) -> Vec<String> {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let mut names: Vec<String> = paths
        .iter()
        .filter(|p| {
            p.parent().is_some_and(|parent| {
                parent
                    .canonicalize()
                    .unwrap_or_else(|_| parent.to_path_buf())
                    == dir
            })
        })
        .filter_map(|p| p.file_name()?.to_str().map(String::from))
        .filter(|name| {
            name == project::SETTINGS_FILE
                || (project::is_project_file(name) && project::check_plain_file_name(name).is_ok())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Starts watching `dir`; `on_change` gets the names of the files that
/// changed (`changed_names`), on a thread of its own.
pub fn start(
    dir: &Path,
    on_change: impl Fn(Vec<String>) + Send + 'static,
) -> notify::Result<ProjectWatch> {
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    let dir = dir.to_path_buf();
    std::thread::Builder::new()
        .name("project watch".into())
        .spawn(move || {
            // Ends when the watcher is dropped (its sender goes).
            while let Ok(first) = rx.recv() {
                let mut paths = Vec::new();
                let mut take = |event: notify::Result<notify::Event>| {
                    if let Ok(e) = event {
                        paths.extend(e.paths);
                    }
                };
                take(first);
                let deadline = Instant::now() + GATHER;
                loop {
                    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(e) => take(e),
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                let names = changed_names(&dir, &paths);
                if !names.is_empty() {
                    on_change(names);
                }
            }
        })
        .map_err(notify::Error::io)?;
    Ok(ProjectWatch { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_only_project_files_at_the_top_of_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::create_dir(dir.join("_build")).unwrap();
        let paths: Vec<PathBuf> = [
            "adder.v",
            "adder_tb.gtkw",
            "logix-ide.toml",
            "notes.txt",
            ".hidden.v",
            "_build/main.json",
            "adder.v",
        ]
        .iter()
        .map(|n| dir.join(n))
        .collect();
        assert_eq!(
            changed_names(dir, &paths),
            ["adder.v", "adder_tb.gtkw", "logix-ide.toml"]
        );
    }

    #[test]
    fn reports_a_file_written_into_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let (tx, rx) = mpsc::channel();
        let _watch = start(&dir, move |names| {
            let _ = tx.send(names);
        })
        .unwrap();
        // Some systems start watching a moment after the call.
        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(dir.join("adder.v"), "module adder; endmodule\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a project file").unwrap();
        let names = rx.recv_timeout(Duration::from_secs(10)).expect("a change");
        assert_eq!(names, ["adder.v"]);
    }
}
