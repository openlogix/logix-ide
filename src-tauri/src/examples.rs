//! Logix's examples, built into the app from the repo's `examples/` folder:
//! `examples/<board>/<name>/`, each a Logix project (its files and a
//! `logix-ide.toml` with the board, a one-line `summary` and optional `notes`). Plain Rust, no Tauri.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use include_dir::{include_dir, Dir};
use serde::Serialize;

static EXAMPLES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../examples");

/// The board of new projects until the user chooses one: the Alhambra II,
/// FPGAwars' own board.
pub const DEFAULT_BOARD: &str = "alhambra-ii";

/// Examples that make good starting points for a new project, in order of
/// preference: simple, and using the board's clock and LEDs.
const STARTER_EXAMPLES: &[&str] = &["blinky", "ledon"];

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
        for example in board.dirs() {
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

/// The example a new project for `board` starts from, if the board has one
/// of the starter examples among `examples`.
pub fn starter_example<'a>(examples: &'a [Example], board: &str) -> Option<&'a Example> {
    STARTER_EXAMPLES.iter().find_map(|name| {
        examples
            .iter()
            .find(|e| e.board == board && e.name == *name)
    })
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
    for file in example.files() {
        let Some(file_name) = file.path().file_name() else {
            continue;
        };
        let path = dst.join(file_name);
        std::fs::write(&path, file.contents())
            .with_context(|| format!("can't write {}", path.display()))?;
    }
    Ok(dst.to_path_buf())
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
        assert_eq!(blinky.summary, "Blinking led");
        assert!(blinky.notes.is_empty());
        // Every example has a one-line summary; qualifiers are notes.
        assert!(list
            .iter()
            .all(|e| !e.summary.is_empty() && !e.summary.contains('\n')));
        assert!(list.iter().all(|e| !e.summary.contains("(untested)")));
        let primer = list
            .iter()
            .find(|e| e.board == "sipeed-tang-primer-20k" && e.name == "blinky")
            .unwrap();
        assert_eq!(primer.notes, ["Not yet tried on the board."]);
        assert_eq!(
            starter_example(&list, DEFAULT_BOARD).map(|e| e.name.as_str()),
            Some("blinky")
        );
        assert!(starter_example(&list, "no-such-board").is_none());
    }

    /// No "template" examples (Apio's starting projects): Bare Minimum is
    /// the starting point for every board. Kéfir I's became its ledon, so
    /// its Bare Minimum has the board's pins.
    #[test]
    fn no_template_examples() {
        let list = all();
        assert!(list.iter().all(|e| e.name != "template"));
        assert_eq!(
            starter_example(&list, "kefir").map(|e| e.name.as_str()),
            Some("ledon")
        );
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
