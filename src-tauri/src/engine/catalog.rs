//! The boards the engine supports, from its `get_catalog` tool, and their
//! USB filters (for the board list's "Connected" group).

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::client::{tool_error, Engine};
use super::devices::{usb_rules, UsbRule};

/// A board the IDE can target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    /// The catalog id, e.g. "alhambra-ii" (the same ids as Apio's).
    pub id: String,
    /// Human-readable name, e.g. "Alhambra II (Lattice, iCE40)".
    pub description: String,
    /// The FPGA architecture: "ice40", "ecp5", "gowin" or "xilinx", for
    /// grouping the board list and naming resources in the build report.
    pub arch: String,
}

/// The engine's boards and their USB filters.
#[derive(Debug, Clone)]
pub struct Catalog {
    /// All boards, sorted by description.
    pub boards: Vec<Board>,
    pub usb_rules: Vec<UsbRule>,
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
        usb_rules: usb_rules(&catalog["boards"]),
    })
}

fn parse_boards(catalog: &Value) -> Result<Vec<Board>> {
    let boards = catalog["boards"]
        .as_object()
        .context("unexpected board list from the engine")?;
    let fpgas = &catalog["fpgas"];
    let mut list: Vec<Board> = boards
        .iter()
        .map(|(id, b)| Board {
            id: id.clone(),
            description: b["description"].as_str().unwrap_or(id).to_string(),
            arch: b["fpga_id"]
                .as_str()
                .and_then(|fpga| fpgas[fpga]["architecture"].as_str())
                .unwrap_or("")
                .to_string(),
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
    }
}
