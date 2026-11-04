//! The boards the engine supports, from its `get_catalog` tool, and their
//! USB filters (for the board list's "Connected" group).

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
    /// The FPGA architecture: "ice40", "ecp5", "gowin" or "xilinx", for
    /// grouping the board list and naming resources in the build report.
    pub arch: String,
    /// The extension of the board's pin files, e.g. "pcf" (see
    /// `constraint_extension`), so the UI can tell which pin file is used.
    /// `None` for an architecture the engine doesn't list.
    pub pin_extension: Option<String>,
    /// The board's choices for programming it, such as the connection or
    /// the memory, in the order to show them. Empty for most boards.
    pub dimensions: Vec<Dimension>,
    /// The combinations of dimension values that exist; the first is the
    /// board's default. Every one has the board's FPGA and pin file.
    pub configurations: Vec<Configuration>,
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

/// The pin constraint file extension the engine's build accepts for an FPGA
/// architecture (its `docs/tool-api.md`, build tool): each family's tools
/// read only their own format, and the engine refuses another extension.
/// `None` for an architecture it doesn't list.
pub fn constraint_extension(arch: &str) -> Option<&'static str> {
    match arch {
        "ice40" => Some("pcf"),
        "ecp5" => Some("lpf"),
        "gowin" => Some("cst"),
        "xilinx" => Some("xdc"),
        _ => None,
    }
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
            json!({"boards": {"fetch": true}, "fpgas": {"fetch": true}}),
            None,
        )
        .await?
        .map_err(|e| tool_error("get_catalog", &e))?;
    Ok(Catalog {
        boards: parse_boards(&catalog)?,
    })
}

fn parse_boards(catalog: &Value) -> Result<Vec<Board>> {
    let boards = catalog["boards"]
        .as_object()
        .context("unexpected board list from the engine")?;
    let fpgas = &catalog["fpgas"];
    let mut list: Vec<Board> = boards
        .iter()
        .map(|(id, b)| {
            let arch = b["fpga_id"]
                .as_str()
                .and_then(|fpga| fpgas[fpga]["architecture"].as_str())
                .unwrap_or("")
                .to_string();
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
            Board {
                id: id.clone(),
                description: b["description"].as_str().unwrap_or(id).to_string(),
                pin_extension: constraint_extension(&arch).map(String::from),
                arch,
                dimensions,
                configurations,
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
    fn constraint_extensions_follow_the_family() {
        assert_eq!(constraint_extension("ice40"), Some("pcf"));
        assert_eq!(constraint_extension("ecp5"), Some("lpf"));
        assert_eq!(constraint_extension("gowin"), Some("cst"));
        assert_eq!(constraint_extension("xilinx"), Some("xdc"));
        assert_eq!(constraint_extension("other"), None);
    }
}
