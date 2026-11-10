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
    /// The FPGA family's ID, such as "ice40" or "gw1n", as the engine
    /// names it.
    pub family: String,
    /// The family's name for people, such as "Lattice iCE40", from the
    /// engine's catalog (`families`), for grouping the board list.
    pub family_label: String,
    /// The family's place in the engine's order, for the board list.
    pub family_order: u32,
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
    /// The board's FPGA, for the Board Explorer and the board picker's
    /// hints and search. `None` when the engine doesn't describe it.
    pub fpga: Option<Fpga>,
    /// What the board wires to FPGA pins, by port name, as the engine gives
    /// it (each with its `type` and that type's fields), for the Board
    /// Explorer. Empty when none are known.
    pub resources: serde_json::Map<String, Value>,
    /// What every design for the board must do (drives and setups), as the
    /// engine gives it. Empty when nothing.
    pub requires: Vec<Value>,
    /// The board's own `ai_hints`, lines of text for AI agents, and people,
    /// writing designs for it (resources carry their own in `resources`),
    /// for the Board Explorer. Empty when none.
    pub ai_hints: Vec<String>,
}

/// A board's FPGA, as Logix Engine's catalog describes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fpga {
    /// The catalog's FPGA ID, such as "ice40hx4k-tq144-8k": every board's
    /// FPGA is a catalog FPGA, custom boards' too, so boards with the same
    /// ID have the same FPGA.
    pub id: String,
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
    /// `{"connection": "usb", "memory": "ram"}`.
    pub when: Variant,
    /// The engine's label for people, e.g. "USB · Flash".
    pub label: String,
    /// Whether programming this way was tried on hardware.
    pub tested: bool,
    /// Warnings for users, e.g. to hold a button while plugging it in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
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

/// The engine's boards and FPGAs.
#[derive(Debug, Clone)]
pub struct Catalog {
    /// All boards, sorted by description.
    pub boards: Vec<Board>,
    /// Every FPGA the engine supports, by family, in the engine's order.
    pub families: Vec<FpgaFamily>,
}

/// An FPGA family and its FPGAs, for the Board Explorer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FpgaFamily {
    /// The family's ID, such as "gw1n", as boards' `family` names it.
    pub id: String,
    /// Its name for people, such as "Gowin GW1N", from the engine.
    pub label: String,
    /// The extension of its pin files, such as "cst". `None` when the
    /// engine doesn't say.
    pub pin_extension: Option<String>,
    /// Its FPGAs, by part number.
    pub fpgas: Vec<Fpga>,
}

/// Reads the engine's catalog.
pub async fn catalog(engine: &Engine) -> Result<Catalog> {
    let catalog = engine
        .call(
            "get_catalog",
            json!({
                "boards": {"fetch": true},
                "fpgas": {"fetch": true},
                "families": {"fetch": true}
            }),
            None,
        )
        .await?
        .map_err(|e| tool_error("get_catalog", &e))?;
    Ok(Catalog {
        boards: parse_boards(&catalog)?,
        families: parse_families(&catalog),
    })
}

/// The project's custom boards folder as Logix Engine gets it: the folder
/// on disk (`custom_boards_folder`), and how the engine's messages name it
/// and its files (`display_paths`). The user's rules (2026-11-07): files are
/// named relative to the project, except those Logix doesn't show, named by
/// their full path. Board files aren't shown, so a saved project's read as
/// their full path; a project never saved has no folder, and its boards are
/// the example's, so they read `boards/<id>.json`.
#[derive(Debug, Clone)]
pub struct BoardsFolder {
    pub path: std::path::PathBuf,
    pub shown_as: String,
}

impl BoardsFolder {
    /// The folder at `path`, of a saved project or not.
    pub fn new(path: std::path::PathBuf, saved: bool) -> BoardsFolder {
        let shown_as = if saved {
            path.to_string_lossy().into_owned()
        } else {
            crate::project::CUSTOM_BOARDS_FOLDER.into()
        };
        BoardsFolder { path, shown_as }
    }

    /// The engine's `display_paths` for it.
    pub fn display_paths(&self) -> Value {
        json!({"folders": [{"path": self.path, "as": self.shown_as}]})
    }
}

/// `custom_boards_folder` and `display_paths` for a request: the folder and
/// how it reads, or nulls without one.
pub fn boards_fields(file: Option<&BoardsFolder>) -> (Value, Value) {
    match file {
        Some(f) => (json!(f.path), f.display_paths()),
        None => (Value::Null, Value::Null),
    }
}

/// Why the project's `custom-boards.json` can't be used: Logix Engine's
/// message, and a diagnostic for each problem, at line and column, the
/// file named as the request said (`BoardsFolder`), for the console.
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

/// Every board with the project's own merged in, as Logix Engine reads them
/// from the custom boards file at `file` (`custom_boards_folder`): the
/// catalog's and the file's, one with a catalog board's ID replacing it,
/// the file's marked `custom`. None without a file; or the problems the
/// engine found in it, at line and column.
pub async fn project_boards(
    engine: &Engine,
    file: Option<&BoardsFolder>,
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
                "families": {"fetch": true},
                "custom_boards_folder": file.path,
                "display_paths": file.display_paths(),
            }),
            None,
        )
        .await?;
    Ok(match answer {
        // Merged by the engine: the project's boards with the catalog's,
        // one with a catalog board's ID replacing it.
        Ok(catalog) => Ok(parse_boards(&catalog)?),
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
    /// Every aspect, in the engine's order: board, family, fpga,
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
    file: Option<&BoardsFolder>,
) -> Result<BoardComparison> {
    // No display_paths: the answer names boards, not files.
    let (boards_file, _) = boards_fields(file);
    let answer = engine
        .call(
            "compare_boards",
            json!({
                "from_board": from,
                "to_board": to,
                "custom_boards_folder": boards_file,
            }),
            None,
        )
        .await?
        .map_err(|e| tool_error("compare_boards", &e))?;
    serde_json::from_value(answer).context("Logix Engine's compare_boards answer isn't readable")
}

/// Every FPGA of `get_catalog`'s answer, by family, in the engine's order
/// (families the engine doesn't describe last, by ID).
pub(super) fn parse_families(catalog: &Value) -> Vec<FpgaFamily> {
    let info = &catalog["families"];
    let mut by_family: BTreeMap<String, Vec<Fpga>> = BTreeMap::new();
    if let Some(fpgas) = catalog["fpgas"].as_object() {
        for (id, f) in fpgas {
            let family = f["family"].as_str().unwrap_or("").to_string();
            by_family.entry(family).or_default().push(Fpga {
                id: id.clone(),
                part_number: f["part_number"].as_str().unwrap_or(id).to_string(),
                size: f["size"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    let mut list: Vec<(u64, FpgaFamily)> = by_family
        .into_iter()
        .map(|(id, mut fpgas)| {
            fpgas.sort_by(|a, b| a.part_number.cmp(&b.part_number));
            let i = &info[id.as_str()];
            let label = i["label"].as_str().unwrap_or(&id).to_string();
            let pin_extension = i["constraint_extension"].as_str().map(String::from);
            (
                i["order"].as_u64().unwrap_or(u64::MAX),
                FpgaFamily {
                    id,
                    label,
                    pin_extension,
                    fpgas,
                },
            )
        })
        .collect();
    list.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));
    list.into_iter().map(|(_, f)| f).collect()
}

pub(super) fn parse_boards(catalog: &Value) -> Result<Vec<Board>> {
    let boards = catalog["boards"]
        .as_object()
        .context("unexpected board list from the engine")?;
    let fpgas = &catalog["fpgas"];
    // The engine's facts about each FPGA family (`get_catalog` families).
    let families = &catalog["families"];
    let mut list: Vec<Board> = boards
        .iter()
        .map(|(id, b)| {
            // The FPGA: always a catalog FPGA, by ID.
            let fpga_id = b["fpga_id"].as_str().unwrap_or("");
            let fpga_def = &fpgas[fpga_id];
            let family = fpga_def["family"].as_str().unwrap_or("").to_string();
            let fpga = fpga_def["part_number"].as_str().map(|part| Fpga {
                id: fpga_id.to_string(),
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
                            label: c["label"].as_str().unwrap_or_default().to_string(),
                            tested: c["tested"].as_bool().unwrap_or(true),
                            user_notes: c["user_notes"].as_str().map(String::from),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let info = &families[family.as_str()];
            Board {
                id: id.clone(),
                description: b["name"].as_str().unwrap_or(id).to_string(),
                family_label: info["label"].as_str().unwrap_or(&family).to_string(),
                family_order: info["order"].as_u64().map_or(u32::MAX, |o| o as u32),
                pin_extension: info["constraint_extension"].as_str().map(String::from),
                family,
                dimensions,
                configurations,
                custom: b["custom"].as_bool().unwrap_or(false),
                fpga,
                resources: b["resources"].as_object().cloned().unwrap_or_default(),
                requires: b["requires"].as_array().cloned().unwrap_or_default(),
                ai_hints: b["ai_hints"]
                    .as_array()
                    .map(|h| {
                        h.iter()
                            .filter_map(|l| l.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
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
    fn parses_boards_with_their_family() {
        let catalog = json!({
            "boards": {
                "icezum": {"name": "Icezum Alhambra", "fpga_id": "ice40hx1k-tq144"},
                "alhambra-ii": {"name": "Alhambra II", "fpga_id": "ice40hx4k-tq144-8k",
                    "resources": {"clk": {"type": "clock", "pin": "49", "mhz": 12}},
                    "requires": [{"type": "setup", "text": "Set the jumper."}],
                    "ai_hints": ["A hint."]},
                "odd": {"fpga_id": "unknown"}
            },
            "fpgas": {
                "ice40hx1k-tq144": {"family": "ice40"},
                "ice40hx4k-tq144-8k": {"family": "ice40"}
            }
        });
        let boards = parse_boards(&catalog).unwrap();
        let ids: Vec<_> = boards.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["alhambra-ii", "icezum", "odd"]);
        assert_eq!(boards[0].family, "ice40");
        assert_eq!(boards[0].fpga, None, "no part number in this catalog");
        assert_eq!(boards[2].description, "odd");
        assert_eq!(boards[2].family, "");
        assert!(parse_boards(&json!({})).is_err());
        assert!(boards[0].dimensions.is_empty());
        // Resources and requirements as the engine gives them.
        assert_eq!(boards[0].resources["clk"]["mhz"], 12);
        assert_eq!(boards[0].requires[0]["type"], "setup");
        assert!(boards[1].resources.is_empty() && boards[1].requires.is_empty());
        assert_eq!(boards[0].ai_hints, ["A hint."]);
        assert!(boards[1].ai_hints.is_empty());
    }

    #[test]
    fn parses_variants() {
        let catalog = json!({
            "boards": {
                "sugar": {"name": "Sugar", "fpga_id": "f",
                    "dimensions": [{"id": "memory", "label": "Memory", "values": [
                        {"id": "flash", "label": "Flash"},
                        {"id": "ram", "label": "RAM", "help": "Lost at power-off."}]}],
                    "variants": [
                        {"selector": {"connection": "usb", "memory": "flash"}, "label": "USB · Flash", "programmer": "p", "args": "a", "tested": true},
                        {"selector": {"connection": "usb", "memory": "ram"}, "label": "USB · RAM", "programmer": "p", "args": "a", "tested": false,
                         "user_notes": "Try it."}]}
            },
            "fpgas": {"f": {"family": "ecp5"}}
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
        // The engine's label for people.
        assert_eq!(b.configurations[1].label, "USB · RAM");
        assert_eq!(b.configurations[1].user_notes.as_deref(), Some("Try it."));
    }

    #[test]
    fn the_engines_problems_as_console_lines() {
        // The engine names the file as display_paths said.
        let error = json!({
            "category": "request",
            "message": "The custom boards folder has problems. boards/mine.json:4:3: Trailing comma.",
            "diagnostics": [
                {"file": "boards/mine.json", "line": 4, "column": 3,
                 "severity": "error", "text": "Trailing comma."},
            ]
        });
        let p = BoardsProblem::from_error(&error);
        assert_eq!(
            p.console_lines(),
            ["\x1b[31mboards/mine.json:4:3: error: Trailing comma.\x1b[0m"]
        );
        let no_place = BoardsProblem {
            message: "Can't read it.".into(),
            diagnostics: Vec::new(),
        };
        assert_eq!(no_place.console_lines(), ["\x1b[31mCan't read it.\x1b[0m"]);
    }

    #[test]
    fn a_boards_folder_reads_as_the_user_knows_it() {
        let path = std::path::PathBuf::from("/Users/you/blinky 3/boards");
        // Saved: its full path (Logix doesn't show its files).
        let saved = BoardsFolder::new(path.clone(), true);
        assert_eq!(
            saved.display_paths(),
            json!({"folders": [{"path": path, "as": "/Users/you/blinky 3/boards"}]})
        );
        // Never saved: the example's folder, named relative to the project.
        let new = BoardsFolder::new("/app/examples/x/boards".into(), false);
        assert_eq!(new.shown_as, "boards");
        assert_eq!(boards_fields(None), (Value::Null, Value::Null));
    }

    #[test]
    fn reads_each_boards_fpga() {
        let catalog = json!({
            "boards": {
                "alhambra-ii": {"name": "Alhambra II", "fpga_id": "ice40hx4k-tq144-8k"},
                "mine": {"name": "Mine", "custom": true, "fpga_id": "lfe5u-25f"},
            },
            "fpgas": {
                "ice40hx4k-tq144-8k": {
                    "family": "ice40", "part_number": "ICE40HX4K-TQ144", "size": "8k"},
                "lfe5u-25f": {
                    "family": "ecp5", "part_number": "LFE5U-25F-6BG256C", "size": "25k"},
            },
        });
        let boards = parse_boards(&catalog).unwrap();
        let alhambra = boards.iter().find(|b| b.id == "alhambra-ii").unwrap();
        assert_eq!(
            alhambra.fpga,
            Some(Fpga {
                id: "ice40hx4k-tq144-8k".into(),
                part_number: "ICE40HX4K-TQ144".into(),
                size: "8k".into(),
            })
        );
        // A custom board's FPGA: a catalog FPGA too.
        let mine = boards.iter().find(|b| b.id == "mine").unwrap();
        assert_eq!(mine.family, "ecp5");
        assert_eq!(mine.fpga.as_ref().unwrap().id, "lfe5u-25f");
        assert_eq!(mine.fpga.as_ref().unwrap().part_number, "LFE5U-25F-6BG256C");
    }

    #[test]
    fn custom_boards_are_marked() {
        let catalog = json!({
            "boards": {
                "mine": {"description": "Mine", "fpga_id": "f", "custom": true},
                "alhambra-ii": {"name": "Alhambra II", "fpga_id": "f"},
            },
            "fpgas": {"f": {"family": "ice40"}}
        });
        let all = parse_boards(&catalog).unwrap();
        let mine: Vec<_> = all.iter().filter(|b| b.custom).collect();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].id, "mine");
        assert!(!all.iter().find(|b| b.id == "alhambra-ii").unwrap().custom);
    }

    #[test]
    fn fpgas_are_listed_by_family_in_the_engines_order() {
        let catalog = json!({
            "fpgas": {
                "b": {"family": "gw2a", "part_number": "GW2A-B", "size": "18k"},
                "a": {"family": "gw2a", "part_number": "GW2A-A", "size": "18k"},
                "i": {"family": "ice40", "part_number": "ICE40HX1K", "size": "1k"},
                "x": {"family": "newfamily", "part_number": "X1"},
            },
            "families": {
                "ice40": {"label": "Lattice iCE40", "order": 0},
                "gw2a": {"label": "Gowin GW2A", "order": 3, "constraint_extension": "cst"},
            },
        });
        let families = parse_families(&catalog);
        let summary: Vec<_> = families
            .iter()
            .map(|f| {
                let parts: Vec<_> = f.fpgas.iter().map(|x| x.part_number.as_str()).collect();
                format!("{} ({}): {}", f.label, f.id, parts.join(", "))
            })
            .collect();
        assert_eq!(
            summary,
            [
                "Lattice iCE40 (ice40): ICE40HX1K",
                "Gowin GW2A (gw2a): GW2A-A, GW2A-B",
                "newfamily (newfamily): X1",
            ]
        );
        assert_eq!(families[1].fpgas[0].id, "a");
        assert_eq!(families[1].fpgas[0].size, "18k");
        assert_eq!(families[1].pin_extension.as_deref(), Some("cst"));
        assert_eq!(families[2].pin_extension, None);
    }

    #[test]
    fn families_come_from_the_engine() {
        let catalog = json!({
            "boards": {
                "arty": {"description": "Arty", "fpga_id": "xc7a35t"},
                "odd": {"description": "Odd", "fpga_id": "other"},
            },
            "fpgas": {
                "xc7a35t": {"family": "artix7"},
                "other": {"family": "newfamily"},
            },
            "families": {
                "artix7": {"label": "Xilinx Artix-7", "short_label": "Artix-7", "constraint_extension": "xdc", "order": 5, "toolchain": "openxc7"},
            },
        });
        let boards = parse_boards(&catalog).unwrap();
        let arty = boards.iter().find(|b| b.id == "arty").unwrap();
        assert_eq!(
            (
                arty.family_label.as_str(),
                arty.family_order,
                arty.pin_extension.as_deref()
            ),
            ("Xilinx Artix-7", 5, Some("xdc"))
        );
        // A family the engine doesn't describe: its ID, last, no extension.
        let odd = boards.iter().find(|b| b.id == "odd").unwrap();
        assert_eq!(
            (
                odd.family_label.as_str(),
                odd.family_order,
                odd.pin_extension.as_deref()
            ),
            ("newfamily", u32::MAX, None)
        );
    }
}
