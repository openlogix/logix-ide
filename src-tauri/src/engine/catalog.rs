//! The boards the engine supports, from its `get_catalog` tool, and the
//! project's own boards (`custom-boards.json`) as the engine reads them.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::client::{tool_error, Engine};

/// A board the IDE can target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    /// The catalog id, e.g. "alhambra-ii".
    pub id: String,
    /// Human-readable name, e.g. "Alhambra II (Lattice, iCE40)".
    pub description: String,
    /// The FPGA architecture's ID, such as "ice40", as the engine names it.
    pub arch: String,
    /// The architecture's name for people, such as "Lattice iCE40", from
    /// the engine's catalog (`architectures`), for grouping the board list.
    pub arch_label: String,
    /// The architecture's place in the engine's order, for the board list.
    pub arch_order: u32,
    /// The extension of the board's pin files, such as "pcf", from the
    /// engine's catalog, so the UI can tell which pin file is used. `None`
    /// when the engine doesn't say.
    pub pin_extension: Option<String>,
    /// The board's choices for programming it, such as the connection or
    /// the memory, in the order to show them. Empty for most boards.
    pub dimensions: Vec<Dimension>,
    /// The combinations of dimension values that exist; the first is the
    /// board's default. Every one has the board's FPGA and pin file.
    pub configurations: Vec<Configuration>,
    /// One of the project's own boards (`custom-boards.json`), rather than
    /// a catalog board.
    pub custom: bool,
    /// The board's FPGA, for Board Info and the board picker's hints and
    /// search. `None` when the engine doesn't describe it.
    pub fpga: Option<Fpga>,
}

/// A board's FPGA, as Logix Engine's catalog describes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fpga {
    /// The catalog's FPGA ID, such as "ice40hx4k-tq144-8k"; `None` for a
    /// custom board's own FPGA definition. Boards with the same ID have the
    /// same FPGA.
    pub id: Option<String>,
    /// The vendor's part number, such as "ICE40HX4K-TQ144".
    pub part_number: String,
    /// The device size, such as "8k".
    pub size: String,
}

/// A board's variant: dimension values by dimension id, e.g.
/// `{"memory": "ram"}`. Empty means the board's first configuration.
pub type Variant = BTreeMap<String, String>;

/// One kind of choice, e.g. the memory to program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    pub id: String,
    /// The name for people, e.g. "Memory".
    pub label: String,
    pub values: Vec<DimensionValue>,
}

/// One value of a dimension, e.g. RAM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionValue {
    pub id: String,
    /// The name for people, e.g. "RAM".
    pub label: String,
    /// A short explanation, e.g. "Faster to upload; lost when the board is
    /// powered off."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

/// One of the board's variants (the engine's name): one combination of
/// dimension values that exists, and how it was tested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    /// The value of every dimension: the engine's selector, e.g.
    /// `{"connection": "usb", "memory": "ram"}`, sent as `board_variant`.
    pub when: Variant,
    /// Whether programming this way was tried on hardware.
    pub tested: bool,
    /// Warnings for users, e.g. to hold a button while plugging it in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
}

/// The engine's selector of the variant the IDE's choice picks: the first
/// configuration with all the chosen dimension values ({} = the default),
/// with every dimension's value. `None` when the board has no such
/// configuration.
pub fn variant_selector(board: &Board, variant: &Variant) -> Option<Variant> {
    board
        .configurations
        .iter()
        .find(|c| variant.iter().all(|(k, v)| c.when.get(k) == Some(v)))
        .map(|c| c.when.clone())
}

/// An engine selector, e.g. `{"connection": "usb", "memory": "ram"}`, as
/// dimension values by dimension ID.
pub fn selector(v: &serde_json::Value) -> Variant {
    v.as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, x)| Some((k.clone(), x.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// The engine's boards.
#[derive(Debug, Clone)]
pub struct Catalog {
    /// All boards, sorted by description.
    pub boards: Vec<Board>,
}

/// Reads the engine's catalog.
pub async fn catalog(engine: &Engine) -> Result<Catalog> {
    let catalog = engine
        .call(
            "get_catalog",
            json!({
                "boards": {"fetch": true},
                "fpgas": {"fetch": true},
                "architectures": {"fetch": true}
            }),
            None,
        )
        .await?
        .map_err(|e| tool_error("get_catalog", &e))?;
    Ok(Catalog {
        boards: parse_boards(&catalog)?,
    })
}

/// The project's custom boards file as Logix Engine gets it: the file on
/// disk (`custom_boards_file`), and how the engine's messages name it
/// (`display_paths`). The user's rules (2026-11-07): files are named
/// relative to the project, except those Logix doesn't show, named by their
/// full path. custom-boards.json isn't shown, so a saved project's reads as
/// its full path; a project never saved has no folder, and its file is the
/// example's, so it reads `custom-boards.json`.
#[derive(Debug, Clone)]
pub struct BoardsFile {
    pub path: std::path::PathBuf,
    pub shown_as: String,
}

impl BoardsFile {
    /// The file at `path`, of a saved project or not.
    pub fn new(path: std::path::PathBuf, saved: bool) -> BoardsFile {
        let shown_as = if saved {
            path.to_string_lossy().into_owned()
        } else {
            crate::project::CUSTOM_BOARDS_FILE.into()
        };
        BoardsFile { path, shown_as }
    }

    /// The engine's `display_paths` for it.
    pub fn display_paths(&self) -> Value {
        json!({"files": [{"path": self.path, "as": self.shown_as}]})
    }
}

/// `custom_boards_file` and `display_paths` for a request: the file and
/// how it reads, or nulls without one.
pub fn boards_fields(file: Option<&BoardsFile>) -> (Value, Value) {
    match file {
        Some(f) => (json!(f.path), f.display_paths()),
        None => (Value::Null, Value::Null),
    }
}

/// Why the project's `custom-boards.json` can't be used: Logix Engine's
/// message, and a diagnostic for each problem, at line and column, the
/// file named as the request said (`BoardsFile`), for the console.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardsProblem {
    pub message: String,
    pub diagnostics: Vec<crate::outcome::Diagnostic>,
}

impl BoardsProblem {
    /// The engine's error for a custom boards file.
    pub fn from_error(error: &Value) -> BoardsProblem {
        BoardsProblem {
            diagnostics: super::task::diagnostics(&error["diagnostics"]),
            message: error["message"]
                .as_str()
                .unwrap_or("Logix Engine can't use the custom boards.")
                .to_string(),
        }
    }

    /// The problems as console lines, in red: one per problem, at line and
    /// column (`/Users/you/blinky/custom-boards.json:4:3: error: Trailing
    /// comma.`), like build errors, so the user can find them in their own
    /// editor; the engine's message when it gives no locations.
    pub fn console_lines(&self) -> Vec<String> {
        let red = |text: String| format!("\x1b[31m{text}\x1b[0m");
        if self.diagnostics.is_empty() {
            return vec![red(self.message.clone())];
        }
        self.diagnostics
            .iter()
            .map(|d| {
                let mut at = d.file.clone().unwrap_or_default();
                for n in [d.line, d.column].into_iter().flatten() {
                    at.push_str(&format!(":{n}"));
                }
                let severity = match d.severity {
                    crate::outcome::Severity::Error => "error",
                    crate::outcome::Severity::Warning => "warning",
                };
                red(format!("{at}: {severity}: {}", d.message))
            })
            .collect()
    }
}

/// The project's own boards, as Logix Engine reads them from the custom
/// boards file at `file` (`custom_boards_file`): none without a file, or
/// the problems the engine found in it, at line and column.
pub async fn project_boards(
    engine: &Engine,
    file: Option<&BoardsFile>,
) -> Result<std::result::Result<Vec<Board>, BoardsProblem>> {
    let Some(file) = file else {
        return Ok(Ok(Vec::new()));
    };
    let answer = engine
        .call(
            "get_catalog",
            json!({
                "boards": {"fetch": true},
                "fpgas": {"fetch": true},
                "architectures": {"fetch": true},
                "custom_boards_file": file.path,
                "display_paths": file.display_paths(),
            }),
            None,
        )
        .await?;
    Ok(match answer {
        Ok(catalog) => Ok(custom_only(parse_boards(&catalog)?)),
        Err(e) => Err(BoardsProblem::from_error(&e)),
    })
}

/// Two boards compared by Logix Engine's `compare_boards`: facts only, for
/// the IDE to word (`boardChangeNote` in the UI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardComparison {
    /// Whether a build for one board is valid for the other.
    #[serde(alias = "same_build")]
    pub same_build: bool,
    /// Every aspect, in the engine's order: board, architecture, fpga,
    /// size, constraint_format, pins (later engines may add more).
    pub aspects: Vec<BoardAspect>,
}

/// One aspect of two boards compared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardAspect {
    pub aspect: String,
    /// "yes", "no" or "unknown" (not in the engine's catalog yet).
    pub same: String,
    /// The two values, when known: any JSON type, by aspect (text today).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Value>,
    /// The comparison for people, such as "FPGA: ICE40HX4K-TQ144 →
    /// ICE40UP5K-SG48.".
    pub text: String,
}

/// What differs between boards `from` and `to` (each in its default
/// variant), the project's own boards included.
pub async fn compare_boards(
    engine: &Engine,
    from: &str,
    to: &str,
    file: Option<&BoardsFile>,
) -> Result<BoardComparison> {
    // No display_paths: the answer names boards, not files.
    let (boards_file, _) = boards_fields(file);
    let answer = engine
        .call(
            "compare_boards",
            json!({
                "from_board": from,
                "to_board": to,
                "custom_boards_file": boards_file,
            }),
            None,
        )
        .await?
        .map_err(|e| tool_error("compare_boards", &e))?;
    serde_json::from_value(answer).context("Logix Engine's compare_boards answer isn't readable")
}

/// The custom boards of a board list.
fn custom_only(boards: Vec<Board>) -> Vec<Board> {
    boards.into_iter().filter(|b| b.custom).collect()
}

pub(super) fn parse_boards(catalog: &Value) -> Result<Vec<Board>> {
    let boards = catalog["boards"]
        .as_object()
        .context("unexpected board list from the engine")?;
    let fpgas = &catalog["fpgas"];
    // The engine's facts about each FPGA family; an engine without them
    // (older) leaves the label as the ID and the order last.
    let archs = &catalog["architectures"];
    let mut list: Vec<Board> = boards
        .iter()
        .map(|(id, b)| {
            // The FPGA: a catalog ID, or a custom board's own definition.
            let fpga_id = b["fpga_id"].as_str();
            let fpga_def = match fpga_id {
                Some(f) => &fpgas[f],
                None => &b["fpga"],
            };
            let arch = fpga_def["architecture"].as_str().unwrap_or("").to_string();
            let fpga = fpga_def["part_number"].as_str().map(|part| Fpga {
                id: fpga_id.map(String::from),
                part_number: part.to_string(),
                size: fpga_def["size"].as_str().unwrap_or("").to_string(),
            });
            let dimensions = serde_json::from_value(b["dimensions"].clone()).unwrap_or_default();
            let configurations = b["variants"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|c| Configuration {
                            when: selector(&c["selector"]),
                            tested: c["tested"].as_bool().unwrap_or(true),
                            user_notes: c["user_notes"].as_str().map(String::from),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let info = &archs[arch.as_str()];
            Board {
                id: id.clone(),
                description: b["description"].as_str().unwrap_or(id).to_string(),
                arch_label: info["label"].as_str().map(String::from).unwrap_or_else(|| {
                    if arch.is_empty() {
                        "Other".into()
                    } else {
                        arch.to_uppercase()
                    }
                }),
                arch_order: info["order"].as_u64().map_or(u32::MAX, |o| o as u32),
                pin_extension: info["constraint_extension"].as_str().map(String::from),
                arch,
                dimensions,
                configurations,
                custom: b["custom"].as_bool().unwrap_or(false),
                fpga,
            }
        })
        .collect();
    list.sort_by_key(|b| b.description.to_lowercase());
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_boards_with_their_architecture() {
        let catalog = json!({
            "boards": {
                "icezum": {"description": "Icezum Alhambra", "fpga_id": "ice40hx1k-tq144"},
                "alhambra-ii": {"description": "Alhambra II", "fpga_id": "ice40hx4k-tq144-8k"},
                "odd": {"fpga_id": "unknown"}
            },
            "fpgas": {
                "ice40hx1k-tq144": {"architecture": "ice40"},
                "ice40hx4k-tq144-8k": {"architecture": "ice40"}
            }
        });
        let boards = parse_boards(&catalog).unwrap();
        let ids: Vec<_> = boards.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["alhambra-ii", "icezum", "odd"]);
        assert_eq!(boards[0].arch, "ice40");
        assert_eq!(boards[0].fpga, None, "no part number in this catalog");
        assert_eq!(boards[2].description, "odd");
        assert_eq!(boards[2].arch, "");
        assert!(parse_boards(&json!({})).is_err());
        assert!(boards[0].dimensions.is_empty());
    }

    #[test]
    fn parses_variants() {
        let catalog = json!({
            "boards": {
                "sugar": {"description": "Sugar", "fpga_id": "f",
                    "dimensions": [{"id": "memory", "label": "Memory", "values": [
                        {"id": "flash", "label": "Flash"},
                        {"id": "ram", "label": "RAM", "help": "Lost at power-off."}]}],
                    "variants": [
                        {"selector": {"connection": "usb", "memory": "flash"}, "programmer": "p", "args": "a", "tested": true},
                        {"selector": {"connection": "usb", "memory": "ram"}, "programmer": "p", "args": "a", "tested": false,
                         "user_notes": "Try it."}]}
            },
            "fpgas": {"f": {"architecture": "ecp5"}}
        });
        let b = &parse_boards(&catalog).unwrap()[0];
        assert_eq!(
            b.dimensions[0].values[1].help.as_deref(),
            Some("Lost at power-off.")
        );
        assert_eq!(b.configurations.len(), 2);
        assert_eq!(b.configurations[1].when["memory"], "ram");
        assert!(!b.configurations[1].tested);
        assert_eq!(b.configurations[1].when["connection"], "usb");
        // The IDE's choice, possibly partial, picks the engine's full selector.
        let pick = |v: serde_json::Value| {
            variant_selector(b, &serde_json::from_value(v).unwrap())
                .map(|s| serde_json::to_value(s).unwrap())
        };
        assert_eq!(
            pick(json!({})),
            Some(json!({"connection": "usb", "memory": "flash"}))
        );
        assert_eq!(
            pick(json!({"memory": "ram"})),
            Some(json!({"connection": "usb", "memory": "ram"}))
        );
        assert_eq!(pick(json!({"memory": "eeprom"})), None);
        assert_eq!(b.configurations[1].user_notes.as_deref(), Some("Try it."));
    }

    #[test]
    fn the_engines_problems_as_console_lines() {
        // The engine names the file as display_paths said.
        let error = json!({
            "category": "request",
            "message": "The custom boards file has problems. custom-boards.json:4:3: Trailing comma.",
            "diagnostics": [
                {"file": "custom-boards.json", "line": 4, "column": 3,
                 "severity": "error", "text": "Trailing comma."},
            ]
        });
        let p = BoardsProblem::from_error(&error);
        assert_eq!(
            p.console_lines(),
            ["\x1b[31mcustom-boards.json:4:3: error: Trailing comma.\x1b[0m"]
        );
        let no_place = BoardsProblem {
            message: "Can't read it.".into(),
            diagnostics: Vec::new(),
        };
        assert_eq!(no_place.console_lines(), ["\x1b[31mCan't read it.\x1b[0m"]);
    }

    #[test]
    fn a_boards_file_reads_as_the_user_knows_it() {
        let path = std::path::PathBuf::from("/Users/you/blinky 3/custom-boards.json");
        // Saved: its full path (Logix doesn't show it).
        let saved = BoardsFile::new(path.clone(), true);
        assert_eq!(
            saved.display_paths(),
            json!({"files": [{"path": path, "as": "/Users/you/blinky 3/custom-boards.json"}]})
        );
        // Never saved: the example's file, named relative to the project.
        let new = BoardsFile::new("/app/examples/x/custom-boards.json".into(), false);
        assert_eq!(new.shown_as, "custom-boards.json");
        assert_eq!(boards_fields(None), (Value::Null, Value::Null));
    }

    #[test]
    fn reads_each_boards_fpga() {
        let catalog = json!({
            "boards": {
                "alhambra-ii": {"description": "Alhambra II", "fpga_id": "ice40hx4k-tq144-8k"},
                "mine": {"description": "Mine", "custom": true, "fpga": {
                    "architecture": "ecp5", "part_number": "LFE5U-25F-6BG256C", "size": "25k"}},
            },
            "fpgas": {"ice40hx4k-tq144-8k": {
                "architecture": "ice40", "part_number": "ICE40HX4K-TQ144", "size": "8k"}},
        });
        let boards = parse_boards(&catalog).unwrap();
        let alhambra = boards.iter().find(|b| b.id == "alhambra-ii").unwrap();
        assert_eq!(
            alhambra.fpga,
            Some(Fpga {
                id: Some("ice40hx4k-tq144-8k".into()),
                part_number: "ICE40HX4K-TQ144".into(),
                size: "8k".into(),
            })
        );
        // A custom board's own FPGA: its family too.
        let mine = boards.iter().find(|b| b.id == "mine").unwrap();
        assert_eq!(mine.arch, "ecp5");
        assert_eq!(mine.fpga.as_ref().unwrap().id, None);
        assert_eq!(mine.fpga.as_ref().unwrap().part_number, "LFE5U-25F-6BG256C");
    }

    #[test]
    fn custom_boards_are_marked() {
        let catalog = json!({
            "boards": {
                "mine": {"description": "Mine", "fpga_id": "f", "custom": true},
                "alhambra-ii": {"description": "Alhambra II", "fpga_id": "f"},
            },
            "fpgas": {"f": {"architecture": "ice40"}}
        });
        let mine = custom_only(parse_boards(&catalog).unwrap());
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].id, "mine");
        assert!(mine[0].custom);
    }

    #[test]
    fn architectures_come_from_the_engine() {
        let catalog = json!({
            "boards": {
                "arty": {"description": "Arty", "fpga_id": "xc7a35t"},
                "odd": {"description": "Odd", "fpga_id": "other"},
            },
            "fpgas": {
                "xc7a35t": {"architecture": "xilinx"},
                "other": {"architecture": "newfamily"},
            },
            "architectures": {
                "xilinx": {"label": "Xilinx", "short_label": "Xilinx", "constraint_extension": "xdc", "order": 3},
            },
        });
        let boards = parse_boards(&catalog).unwrap();
        let arty = boards.iter().find(|b| b.id == "arty").unwrap();
        assert_eq!(
            (
                arty.arch_label.as_str(),
                arty.arch_order,
                arty.pin_extension.as_deref()
            ),
            ("Xilinx", 3, Some("xdc"))
        );
        // A family the engine doesn't describe: its ID, last, no extension.
        let odd = boards.iter().find(|b| b.id == "odd").unwrap();
        assert_eq!(
            (
                odd.arch_label.as_str(),
                odd.arch_order,
                odd.pin_extension.as_deref()
            ),
            ("NEWFAMILY", u32::MAX, None)
        );
    }
}
