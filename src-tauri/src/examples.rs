//! Logix's examples, built into the app from the repo's `examples/` folder:
//! `examples/<board>/<name>/`, each a Logix project (its files and a
//! `logix-ide.toml` with the board, a one-line `summary` and optional `notes`). Plain Rust, no Tauri.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use include_dir::{include_dir, Dir};
use serde::Serialize;

static EXAMPLES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../examples");

/// The board of new projects until the user chooses one: the iCESugar v1.5
/// (a tie-break; the website repo's CLAUDE.md, Canonical board).
pub const DEFAULT_BOARD: &str = "icesugar-1-5";

/// Each catalog board's Bare Minimum, generated (`generate.rs`) like its
/// blinky: `examples/<board>/bare-minimum/`, the board's pins and a design
/// that does nothing yet. Not listed among the examples: the Examples
/// dialog shows it first for every board.
pub const BARE_MINIMUM: &str = "bare-minimum";

/// `main.v` for a board without a Bare Minimum (one the catalog doesn't
/// have): an empty design, which builds.
pub const EMPTY_MAIN: &str = "\
// The top module must be named 'main'.
//
// There's no Bare Minimum for this board to take its pins from. Add ports
// here, and a pin file (File > New File...) that assigns them to the
// FPGA's pins, in the format the board uses (such as a .pcf file for
// iCE40 boards). See the board's documentation for the pins.

module main;

  // Your design goes here.

endmodule
";

/// The examples Logix offers, and how many others were hidden.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleList {
    pub examples: Vec<Example>,
    /// Examples left out because Logix Engine doesn't support their board.
    pub hidden: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub board: String,
    pub name: String,
    /// What it does, in one line (`summary` in its `logix-ide.toml`).
    pub summary: String,
    /// More about it, a line each, such as that it wasn't tried on the
    /// board yet (`notes`; empty when it has none).
    pub notes: Vec<String>,
}

/// Every example, sorted by board then name.
pub fn all() -> Vec<Example> {
    let mut list = Vec::new();
    for board in EXAMPLES.dirs() {
        for example in board.dirs().filter(|d| !d.path().ends_with(BARE_MINIMUM)) {
            let name = |d: &Dir| {
                d.path()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };
            let settings = example
                .get_file(example.path().join(crate::project::SETTINGS_FILE))
                .and_then(|f| f.contents_utf8())
                .and_then(|t| t.parse::<toml_edit::DocumentMut>().ok());
            let summary = settings
                .as_ref()
                .and_then(|d| d.get("summary")?.as_str().map(String::from))
                .unwrap_or_default();
            let notes = settings
                .as_ref()
                .and_then(|d| d.get("notes")?.as_array().cloned())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            list.push(Example {
                board: name(board),
                name: name(example),
                summary,
                notes,
            });
        }
    }
    list.sort_by(|a, b| (&a.board, &a.name).cmp(&(&b.board, &b.name)));
    list
}

/// Whether `board` has a Bare Minimum.
pub fn has_bare_minimum(board: &str) -> bool {
    EXAMPLES
        .get_dir(Path::new(board).join(BARE_MINIMUM))
        .is_some()
}

/// The boards with a Bare Minimum, sorted (for tests).
#[cfg(test)]
pub fn bare_minimum_boards() -> Vec<String> {
    let mut list: Vec<String> = EXAMPLES
        .dirs()
        .filter(|b| b.get_dir(b.path().join(BARE_MINIMUM)).is_some())
        .filter_map(|b| Some(b.path().file_name()?.to_string_lossy().into_owned()))
        .collect();
    list.sort();
    list
}

/// Writes an example's files into `dst` (replaced) and returns it, so it
/// can be loaded as a project and its other files copied on save.
pub fn write(board: &str, name: &str, dst: &Path) -> Result<PathBuf> {
    let Some(example) = EXAMPLES.get_dir(Path::new(board).join(name)) else {
        bail!("there's no example {board}/{name}");
    };
    if dst.exists() {
        std::fs::remove_dir_all(dst).with_context(|| format!("can't remove {}", dst.display()))?;
    }
    std::fs::create_dir_all(dst).with_context(|| format!("can't create {}", dst.display()))?;
    write_dir(example, example.path(), dst)?;
    Ok(dst.to_path_buf())
}

/// Writes `dir`'s files, and its subfolders' (such as `boards/`), under
/// `dst`, by their paths relative to `root`.
fn write_dir(dir: &Dir<'_>, root: &Path, dst: &Path) -> Result<()> {
    for file in dir.files() {
        let Ok(rel) = file.path().strip_prefix(root) else {
            continue;
        };
        let path = dst.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("can't create {}", parent.display()))?;
        }
        std::fs::write(&path, file.contents())
            .with_context(|| format!("can't write {}", path.display()))?;
    }
    for sub in dir.dirs() {
        write_dir(sub, root, dst)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project;

    #[test]
    fn lists_the_examples_with_summaries_and_notes() {
        let list = all();
        assert!(list.len() > 100, "{}", list.len());
        let blinky = list
            .iter()
            .find(|e| e.board == DEFAULT_BOARD && e.name == "blinky")
            .unwrap();
        assert_eq!(
            blinky.summary,
            "Blinks the RGB LED in green about once a second."
        );
        assert!(blinky.notes.is_empty());
        // Every example has a one-line summary; qualifiers are notes.
        assert!(list
            .iter()
            .all(|e| !e.summary.is_empty() && !e.summary.contains('\n')));
        assert!(list.iter().all(|e| !e.summary.contains("(untested)")));
        let primer = list
            .iter()
            .find(|e| e.board == "sipeed-tang-primer-20k-lite" && e.name == "blinky")
            .unwrap();
        assert_eq!(primer.notes, ["Not yet tried on the board."]);
        // Bare Minimum isn't an example, but every catalog board has one.
        assert!(list.iter().all(|e| e.name != BARE_MINIMUM));
        assert!(has_bare_minimum(DEFAULT_BOARD));
        assert!(!has_bare_minimum("no-such-board"));
        assert!(bare_minimum_boards().len() >= 81);
    }

    /// No "template" examples (Apio's starting projects): Bare Minimum is
    /// the starting point for every board.
    #[test]
    fn no_template_examples() {
        assert!(all().iter().all(|e| e.name != "template"));
    }

    /// Every Bare Minimum opens as a Logix project with its board.
    #[test]
    fn every_bare_minimum_is_a_project() {
        let tmp = tempfile::tempdir().unwrap();
        for board in bare_minimum_boards() {
            let dir = write(&board, BARE_MINIMUM, &tmp.path().join("bare")).unwrap();
            let loaded = project::load(&dir).unwrap_or_else(|err| panic!("{board}: {err:#}"));
            assert_eq!(loaded.board.as_deref(), Some(board.as_str()));
            assert_eq!(loaded.files[0].name, project::MAIN_FILE);
        }
    }

    /// Every example opens as a Logix project with its board.
    #[test]
    fn every_example_is_a_project() {
        let tmp = tempfile::tempdir().unwrap();
        for e in all() {
            let dir = write(&e.board, &e.name, &tmp.path().join("example")).unwrap();
            let loaded =
                project::load(&dir).unwrap_or_else(|err| panic!("{}/{}: {err:#}", e.board, e.name));
            assert_eq!(loaded.board.as_deref(), Some(e.board.as_str()));
            assert!(project::MAIN_FILES.contains(&loaded.files[0].name.as_str()));
        }
        assert!(write("alhambra-ii", "nope", &tmp.path().join("x")).is_err());
    }

    /// Every example's board is one of Logix Engine's (as in the fake
    /// backend's catalog, from the engine's get_catalog).
    #[test]
    fn every_board_is_an_engine_board() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ui/e2e/fixtures/catalog.json"
        ))
        .unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&text).unwrap();
        let boards: Vec<&str> = catalog["boards"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|b| b["id"].as_str())
            .collect();
        for e in all() {
            assert!(boards.contains(&e.board.as_str()), "{}", e.board);
        }
    }
}
