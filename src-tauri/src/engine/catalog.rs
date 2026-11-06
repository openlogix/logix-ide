//! The boards the engine supports, from its `get_catalog` tool, and the
//! project's own boards (`custom-boards.json`) as the engine reads them.

use std::collections::BTreeMap;
use std::path::Path;

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
    /// The engine's variant ID, e.g. "usb-ram", sent as `board_variant`.
    /// The UI works with `when` only.
    #[serde(skip)]
    pub id: String,
    /// The value of every dimension.
    pub when: Variant,
    /// Whether programming this way was tried on hardware.
    pub tested: bool,
    /// Warnings for users, e.g. to hold a button while plugging it in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
}

/// The engine's ID of the variant the IDE's choice picks: the first
/// configuration with all the chosen dimension values ({} = the default).
/// `None` when the board has no such configuration.
pub fn variant_id(board: &Board, variant: &Variant) -> Option<String> {
    board
        .configurations
        .iter()
        .find(|c| variant.iter().all(|(k, v)| c.when.get(k) == Some(v)))
        .map(|c| c.id.clone())
}

/// The dimension values of a board's variant, by the engine's variant ID.
pub fn variant_values(board: &Board, id: &str) -> Option<Variant> {
    board
        .configurations
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.when.clone())
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

/// Why the project's `custom-boards.json` can't be used: Logix Engine's
/// message, and a diagnostic for each problem, at line and column in
/// `custom-boards.json`, for the editor's markers.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardsProblem {
    pub message: String,
    pub diagnostics: Vec<crate::outcome::Diagnostic>,
}

impl BoardsProblem {
    /// The engine's error for a custom boards file at `file`, its
    /// diagnostics named as the project's file.
    pub fn from_error(error: &Value, file: &Path) -> BoardsProblem {
        let given = file.to_string_lossy();
        let mut diagnostics = super::task::diagnostics(&error["diagnostics"]);
        for d in &mut diagnostics {
            if d.file.as_deref() == Some(given.as_ref()) {
                d.file = Some(crate::project::CUSTOM_BOARDS_FILE.into());
            }
        }
        BoardsProblem {
            // The scratch copy's path reads as the project's file.
            message: error["message"]
                .as_str()
                .unwrap_or("Logix Engine can't use the custom boards.")
                .replace(given.as_ref(), crate::project::CUSTOM_BOARDS_FILE),
            diagnostics,
        }
    }
}

/// The project's own boards, as Logix Engine reads them from the custom
/// boards file at `file` (`custom_boards_file`): none without a file, or
/// the problems the engine found in it, at line and column.
pub async fn project_boards(
    engine: &Engine,
    file: Option<&Path>,
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
                "custom_boards_file": file,
            }),
            None,
        )
        .await?;
    Ok(match answer {
        Ok(catalog) => Ok(custom_only(parse_boards(&catalog)?)),
        Err(e) => Err(BoardsProblem::from_error(&e, file)),
    })
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
                            id: c["id"].as_str().unwrap_or_default().to_string(),
                            when: ["connection", "memory"]
                                .iter()
                                .filter_map(|d| {
                                    c[*d].as_str().map(|v| (d.to_string(), v.to_string()))
                                })
                                .collect(),
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
                        {"id": "usb-flash", "connection": "usb", "memory": "flash", "programmer": "p", "args": "a", "usb": {}, "tested": true},
                        {"id": "usb-ram", "connection": "usb", "memory": "ram", "programmer": "p", "args": "a", "usb": {}, "tested": false,
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
        assert_eq!(b.configurations[1].id, "usb-ram");
        // The IDE's choice, possibly partial, picks the engine's variant ID.
        let pick = |v: serde_json::Value| variant_id(b, &serde_json::from_value(v).unwrap());
        assert_eq!(pick(json!({})).as_deref(), Some("usb-flash"));
        assert_eq!(pick(json!({"memory": "ram"})).as_deref(), Some("usb-ram"));
        assert_eq!(pick(json!({"memory": "eeprom"})), None);
        assert_eq!(variant_values(b, "usb-ram").unwrap()["memory"], "ram");
        assert_eq!(b.configurations[1].user_notes.as_deref(), Some("Try it."));
    }

    #[test]
    fn the_engines_problems_name_the_projects_file() {
        let file = Path::new("/Users/you/.logix/boards/1a2b/custom-boards.json");
        let error = json!({
            "category": "request",
            "message": "The custom boards file has problems. /Users/you/.logix/boards/1a2b/custom-boards.json:4:3: Trailing comma.",
            "diagnostics": [
                {"file": "/Users/you/.logix/boards/1a2b/custom-boards.json", "line": 4, "column": 3,
                 "severity": "error", "text": "Trailing comma."},
            ]
        });
        let p = BoardsProblem::from_error(&error, file);
        assert_eq!(
            p.message,
            "The custom boards file has problems. custom-boards.json:4:3: Trailing comma."
        );
        assert_eq!(p.diagnostics.len(), 1);
        let d = &p.diagnostics[0];
        assert_eq!(
            (d.file.as_deref(), d.line, d.column, d.message.as_str()),
            (
                Some("custom-boards.json"),
                Some(4),
                Some(3),
                "Trailing comma."
            )
        );
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
