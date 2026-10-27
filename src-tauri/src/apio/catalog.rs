//! Apio's board and example catalogs (via `apio api ...` JSON), and
//! fetching an example's files.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::{args, Apio};

/// A board the IDE can target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    /// Apio's board id, e.g. "alhambra-ii".
    pub id: String,
    /// Human-readable name, e.g. "Alhambra II (Lattice, iCE40)".
    pub description: String,
    /// The FPGA architecture, e.g. "ice40", "ecp5", "gowin", "xilinx"
    /// (empty if Apio doesn't say), for grouping the board list.
    pub arch: String,
}

/// The examples Logix can open, and how many others were hidden.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleList {
    pub examples: Vec<Example>,
    /// Examples left out because Logix doesn't support them yet.
    pub hidden: usize,
}

/// One of Apio's examples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub board: String,
    pub name: String,
    pub description: String,
}

impl Apio {
    /// All boards, sorted by description.
    pub async fn boards(&self) -> Result<Vec<Board>> {
        parse_boards(&self.api_json("get-boards", None).await?)
    }

    /// The examples Logix supports (see `example_is_supported`), sorted by
    /// board then name.
    pub async fn examples(&self) -> Result<ExampleList> {
        let all = parse_examples(&self.api_json("get-examples", None).await?)?;
        let total = all.len();
        // Apio's example list has no details about the examples, so look at
        // their folders in Apio's definitions package. If that isn't
        // possible, show them all rather than none.
        let Some(root) = self.examples_dir().await else {
            return Ok(ExampleList {
                examples: all,
                hidden: 0,
            });
        };
        let examples: Vec<Example> = tokio::task::spawn_blocking(move || {
            all.into_iter()
                .filter(|e| example_is_supported(&root.join(&e.board).join(&e.name), &e.board))
                .collect()
        })
        .await?;
        Ok(ExampleList {
            hidden: total - examples.len(),
            examples,
        })
    }

    /// Where Apio keeps its examples: `<packages>/definitions/examples`.
    async fn examples_dir(&self) -> Option<PathBuf> {
        let dir = self.definitions_dir().await?.join("examples");
        dir.is_dir().then_some(dir)
    }

    /// Apio's definitions package: `<packages>/definitions`.
    async fn definitions_dir(&self) -> Option<PathBuf> {
        let system = self.api_json("get-system", None).await.ok()?;
        let packages = system.pointer("/system/apio-packages-dir")?.as_str()?;
        Some(Path::new(packages).join("definitions"))
    }

    /// The boards' USB identities (see `board_match.rs`). Empty if Apio's
    /// definitions can't be read: then no board shows as connected.
    pub async fn usb_rules(&self) -> Vec<super::UsbRule> {
        let Some(dir) = self.definitions_dir().await else {
            return Vec::new();
        };
        let path = dir.join("boards.jsonc");
        let rules = tokio::fs::read_to_string(&path)
            .await
            .map_err(anyhow::Error::from)
            .and_then(|text| super::board_match::parse_usb_rules(&text));
        rules.unwrap_or_else(|e| {
            crate::log!("Can't read board USB ids from {}: {e:#}", path.display());
            Vec::new()
        })
    }

    /// Copies an example's files into a fresh scratch folder and returns it.
    /// The folder is replaced by the next fetch. Apio's output goes to
    /// `on_line`.
    pub async fn fetch_example<F>(&self, board: &str, name: &str, on_line: F) -> Result<PathBuf>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        check_id(board)?;
        check_id(name)?;
        let dst = self.tmp.join("example");
        if dst.exists() {
            tokio::fs::remove_dir_all(&dst)
                .await
                .with_context(|| format!("can't remove {}", dst.display()))?;
        }
        // Apio requires that the destination doesn't exist yet.
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let fetch = args([
            OsString::from("examples"),
            "fetch".into(),
            format!("{board}/{name}").into(),
            "-d".into(),
            dst.clone().into_os_string(),
        ]);
        check_status(self.run(&fetch, None, on_line).await?)?;
        Ok(dst)
    }

    /// Runs `apio api <what> -o <file>` and parses the file. (Apio can mix
    /// status lines into stdout, so the JSON is read from a file.)
    /// `dir` is the project folder for project commands such as
    /// `get-build-report`, or `None`. Info commands run silently (not shown
    /// in the console).
    pub(super) async fn api_json(&self, what: &str, dir: Option<&Path>) -> Result<Value> {
        tokio::fs::create_dir_all(&self.tmp).await?;
        let out = self.tmp.join(format!("api-{what}.json"));
        let api = args([
            OsString::from("api"),
            what.into(),
            "-f".into(),
            "-o".into(),
            out.clone().into_os_string(),
        ]);
        self.run_silent(&api, dir).await?;
        read_json(&out).await
    }
}

/// Fails if Apio did. Its output, already in the console, has the details.
fn check_status(status: std::process::ExitStatus) -> Result<()> {
    if !status.success() {
        bail!("Apio failed ({status}); see the console for details");
    }
    Ok(())
}

async fn read_json(path: &Path) -> Result<Value> {
    let bytes = tokio::fs::read(path)
        .await
        .with_context(|| format!("can't read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("{} isn't valid JSON", path.display()))
}

/// The board of new projects until the user chooses one: the Alhambra II,
/// FPGAwars' own board (the canonical Apio board).
pub const DEFAULT_BOARD: &str = "alhambra-ii";

/// Examples that make good starting points for a new project, in order of
/// preference: simple, and using the board's clock and LEDs.
const STARTER_EXAMPLES: &[&str] = &["blinky", "ledon"];

/// The example a new project for `board` starts from, if the board has one
/// of the starter examples among `examples`.
pub fn starter_example<'a>(examples: &'a [Example], board: &str) -> Option<&'a Example> {
    STARTER_EXAMPLES.iter().find_map(|name| {
        examples
            .iter()
            .find(|e| e.board == board && e.name == *name)
    })
}

/// Board and example ids are passed to Apio as arguments; keep them plain.
fn check_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && !id.starts_with(['-', '.'])
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok {
        bail!("invalid board or example name: {id:?}");
    }
    Ok(())
}

/// `apio.ini` settings Logix reproduces when it builds. Examples that rely
/// on anything else (custom constraint file names, defines, programmer
/// commands, extra tool options, ...) would build differently, so they're
/// hidden.
const SUPPORTED_INI_KEYS: &[&str] = &["board", "top-module", "default-testbench"];

// TODO(revisit): these rules hide 18 of Apio's 164 examples (Apio
// 2026-10-22). Revisit when Logix supports subfolders, several board setups
// (envs) or more apio.ini settings, so more examples can be shown. See
// "Open items" in CLAUDE.md.
/// Whether Logix can open and build the example in `dir` as is: a top-level
/// `main.v` or `main.sv`, no subfolders, and an `apio.ini` (if any) with a
/// single env for `board`, top module `main` and only supported settings.
pub fn example_is_supported(dir: &Path, board: &str) -> bool {
    if !crate::project::MAIN_FILES
        .iter()
        .any(|m| dir.join(m).is_file())
    {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        if !hidden && entry.path().is_dir() {
            return false;
        }
    }
    match std::fs::read_to_string(dir.join("apio.ini")) {
        Ok(text) => ini_is_supported(&text, board),
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    }
}

fn ini_is_supported(text: &str, board: &str) -> bool {
    let mut envs = 0;
    for line in text.lines() {
        // Indented lines continue a multi-line value.
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let section = section.trim();
            if section.starts_with("env:") {
                envs += 1;
            } else if section != "common" {
                return false; // e.g. [apio] with default-env
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let (key, value) = (key.trim(), value.trim());
        if !SUPPORTED_INI_KEYS.contains(&key)
            || (key == "board" && value != board)
            || (key == "top-module" && value != "main")
        {
            return false;
        }
    }
    envs == 1
}

fn parse_boards(json: &Value) -> Result<Vec<Board>> {
    let map = json
        .get("boards")
        .and_then(Value::as_object)
        .context("unexpected board list from Apio")?;
    let mut boards: Vec<Board> = map
        .iter()
        .map(|(id, info)| Board {
            id: id.clone(),
            description: info
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or(id)
                .to_string(),
            arch: info
                .pointer("/fpga/definition/arch")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
        .collect();
    boards.sort_by_key(|b| b.description.to_lowercase());
    Ok(boards)
}

fn parse_examples(json: &Value) -> Result<Vec<Example>> {
    let map = json
        .get("examples")
        .and_then(Value::as_object)
        .context("unexpected example list from Apio")?;
    let mut examples = Vec::new();
    for (board, list) in map {
        let Some(list) = list.as_object() else {
            continue;
        };
        for (name, info) in list {
            examples.push(Example {
                board: board.clone(),
                name: name.clone(),
                description: info
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    examples.sort_by(|a, b| (&a.board, &a.name).cmp(&(&b.board, &b.name)));
    Ok(examples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_and_sorts_boards() {
        let json = json!({"boards": {
            "zz-board": {"description": "Alpha Board", "fpga": {}},
            "aa-board": {"description": "beta board",
                         "fpga": {"definition": {"arch": "ice40"}}},
            "no-desc": {}
        }});
        let boards = parse_boards(&json).unwrap();
        let ids: Vec<_> = boards.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["zz-board", "aa-board", "no-desc"]);
        assert_eq!(boards[2].description, "no-desc");
        let archs: Vec<_> = boards.iter().map(|b| b.arch.as_str()).collect();
        assert_eq!(archs, ["", "ice40", ""]);
    }

    #[test]
    fn parses_examples() {
        let json = json!({"examples": {
            "b2": {"blinky": {"description": "Blinks an LED"}},
            "b1": {"ledon": {"description": "Turns on an LED"}, "blinky": {}}
        }});
        let examples = parse_examples(&json).unwrap();
        let ids: Vec<_> = examples
            .iter()
            .map(|e| format!("{}/{}", e.board, e.name))
            .collect();
        assert_eq!(ids, ["b1/blinky", "b1/ledon", "b2/blinky"]);
        assert_eq!(examples[2].description, "Blinks an LED");
    }

    #[test]
    fn rejects_bad_json() {
        assert!(parse_boards(&json!({"x": 1})).is_err());
        assert!(parse_examples(&json!([])).is_err());
    }

    fn example_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        for (name, text) in files {
            let path = tmp.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        tmp
    }

    const SIMPLE_INI: &str = "; comment\n[env:default]\nboard = alhambra-ii\ntop-module = main\n";

    #[test]
    fn supports_simple_examples() {
        let dir = example_dir(&[
            ("apio.ini", SIMPLE_INI),
            ("main.v", ""),
            ("pinout.pcf", ""),
            (".vscode/settings.json", ""),
        ]);
        assert!(example_is_supported(dir.path(), "alhambra-ii"));
        let sv = example_dir(&[("apio.ini", SIMPLE_INI), ("main.sv", "")]);
        assert!(example_is_supported(sv.path(), "alhambra-ii"));
    }

    #[test]
    fn rejects_examples_logix_cant_handle() {
        let no_main = example_dir(&[("apio.ini", SIMPLE_INI), ("top.v", "")]);
        assert!(!example_is_supported(no_main.path(), "alhambra-ii"));
        let subfolder = example_dir(&[("apio.ini", SIMPLE_INI), ("main.v", ""), ("bcd/d.v", "")]);
        assert!(!example_is_supported(subfolder.path(), "alhambra-ii"));
        let other_board = example_dir(&[("apio.ini", SIMPLE_INI), ("main.v", "")]);
        assert!(!example_is_supported(other_board.path(), "icezum"));
    }

    #[test]
    fn checks_apio_ini_settings() {
        let ok = |ini: &str| ini_is_supported(ini, "b");
        assert!(ok("[env:default]\nboard = b\ntop-module = main\n"));
        assert!(ok(
            "[common]\ntop-module = main\n[env:x]\nboard = b\ndefault-testbench = main_tb.v\n"
        ));
        assert!(!ok("[env:a]\nboard = b\n[env:c]\nboard = b\n"));
        assert!(!ok("[apio]\ndefault-env = a\n[env:a]\nboard = b\n"));
        assert!(!ok("[env:a]\nboard = b\ntop-module = top\n"));
        assert!(!ok("[env:a]\nboard = b\nconstraint-file = x.pcf\n"));
        assert!(!ok("[env:a]\nboard = b\ndefines =\n    N=4\n"));
        assert!(!ok("board = b\n"));
    }

    /// Lists which of Apio's real examples are hidden. Run with:
    /// `LOGIX_EXAMPLES_DIR=<apio home>/packages/definitions/examples
    /// cargo test survey_real_examples -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn survey_real_examples() {
        let root = PathBuf::from(std::env::var("LOGIX_EXAMPLES_DIR").unwrap());
        let (mut shown, mut hidden) = (0, Vec::new());
        let dirs = |p: &Path| -> Vec<std::fs::DirEntry> {
            std::fs::read_dir(p)
                .unwrap()
                .flatten()
                .filter(|e| e.path().is_dir())
                .collect()
        };
        for board in dirs(&root) {
            for ex in dirs(&board.path()) {
                let b = board.file_name().to_string_lossy().into_owned();
                if example_is_supported(&ex.path(), &b) {
                    shown += 1;
                } else {
                    hidden.push(format!("{b}/{}", ex.file_name().to_string_lossy()));
                }
            }
        }
        hidden.sort();
        println!("shown: {shown}, hidden: {}", hidden.len());
        for h in hidden {
            println!("  hidden: {h}");
        }
    }

    #[test]
    fn picks_starter_example() {
        let ex = |board: &str, name: &str| Example {
            board: board.into(),
            name: name.into(),
            description: String::new(),
        };
        let all = vec![
            ex("a", "ledon"),
            ex("a", "blinky"),
            ex("b", "ledon"),
            ex("c", "pll"),
        ];
        assert_eq!(starter_example(&all, "a").unwrap().name, "blinky");
        assert_eq!(starter_example(&all, "b").unwrap().name, "ledon");
        assert!(starter_example(&all, "c").is_none());
        assert!(starter_example(&all, "z").is_none());
    }

    #[test]
    fn checks_ids() {
        assert!(check_id("alhambra-ii").is_ok());
        assert!(check_id("bcd-counter-sv").is_ok());
        assert!(check_id("").is_err());
        assert!(check_id("-d").is_err());
        assert!(check_id("..").is_err());
        assert!(check_id("a/b").is_err());
        assert!(check_id("a b").is_err());
    }
}
