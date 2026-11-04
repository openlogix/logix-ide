//! Connected USB devices (the engine's `list_devices`) and the boards they
//! may be. The engine does the matching and rates each match (confidence 1
//! to 3, higher is surer); the IDE only reads it, for Tools > Connected
//! Devices, the board list's "Connected" group and Upload's choice of
//! device.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::catalog::Variant;
use super::client::{tool_error, Engine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    /// The engine's device ID, for `program_board`. Opaque and short-lived.
    pub id: String,
    /// USB vendor and product ids, as 4 lowercase hex digits.
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
    pub serial_number: String,
    /// The boards it may be, best first (the engine's order).
    pub boards: Vec<DeviceBoard>,
    /// With a build folder: how sure the engine is that it is the build's
    /// board in the build's variant (1 to 3).
    pub build_confidence: Option<u8>,
}

/// A board a device may be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBoard {
    pub board: String,
    /// The best confidence of its variants (1 to 3, higher is surer).
    pub confidence: u8,
    /// The variants the device fits, with their confidence.
    #[serde(skip)]
    pub variants: Vec<(Variant, u8)>,
}

/// A device the engine sees but can't open (a missing driver or
/// permissions), and whose USB ids fit a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreachableDevice {
    pub vid: String,
    pub pid: String,
    /// The engine's suggested fix, e.g. `fix_device_permissions`.
    pub action: String,
    /// The fix in words.
    pub action_description: String,
}

/// What `list_devices` found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceList {
    pub devices: Vec<UsbDevice>,
    pub unreachable: Vec<UnreachableDevice>,
}

/// The connected USB devices, with the boards they may be. With
/// `build_folder`, only the devices that fit that build's board and variant,
/// best first.
pub async fn list_devices(engine: &Engine, build_folder: Option<&str>) -> Result<DeviceList> {
    let args = match build_folder {
        Some(folder) => json!({ "build_folder": folder }),
        None => json!({}),
    };
    let result = engine
        .call("list_devices", args, None)
        .await?
        .map_err(|e| tool_error("list_devices", &e))?;
    parse_devices(&result)
}

pub(super) fn parse_devices(result: &Value) -> Result<DeviceList> {
    let list = result["devices"]
        .as_array()
        .context("unexpected device list from the engine")?;
    let text = |d: &Value, key: &str| d[key].as_str().unwrap_or("").trim().to_string();
    let rating = |v: &Value| v.as_u64().map(|c| c.clamp(1, 3) as u8);
    let devices = list
        .iter()
        .filter(|d| d["kind"] == "usb")
        .map(|d| UsbDevice {
            id: text(d, "id"),
            vid: text(d, "vid"),
            pid: text(d, "pid"),
            manufacturer: text(d, "manufacturer"),
            product: text(d, "product"),
            serial_number: text(d, "serial_number"),
            boards: d["boards"]
                .as_array()
                .map(|boards| boards.iter().map(parse_board).collect())
                .unwrap_or_default(),
            build_confidence: rating(&d["build_confidence"]),
        })
        .collect();
    let unreachable = result["unreachable"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|d| UnreachableDevice {
                    vid: text(d, "vid"),
                    pid: text(d, "pid"),
                    action: d["action"]["code"].as_str().unwrap_or("").to_string(),
                    action_description: d["action"]["description"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(DeviceList {
        devices,
        unreachable,
    })
}

fn parse_board(b: &Value) -> DeviceBoard {
    let variants: Vec<(Variant, u8)> = b["variants"]
        .as_array()
        .map(|vs| {
            vs.iter()
                .map(|v| {
                    let when = serde_json::from_value(v["when"].clone()).unwrap_or_default();
                    let c = v["confidence"].as_u64().unwrap_or(1).clamp(1, 3) as u8;
                    (when, c)
                })
                .collect()
        })
        .unwrap_or_default();
    DeviceBoard {
        board: b["board"].as_str().unwrap_or("").to_string(),
        confidence: variants.iter().map(|(_, c)| *c).max().unwrap_or(1),
        variants,
    }
}

/// A board that matches a connected device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedBoard {
    pub board: String,
    /// The device's name matched the board's too (engine confidence 2 or
    /// more), not just the USB chip's ids, which many boards share.
    pub exact: bool,
    /// Worth listing in the board picker's "Connected" group: a name match,
    /// or one of only a few boards with the chip's ids (generic FTDI ids fit
    /// dozens). Boards that aren't suggested still match the device, so the
    /// selected one shows as connected.
    pub suggested: bool,
    /// The configurations whose filter matched, e.g. `{"connection":
    /// "ft232h", "memory": "ram"}`.
    pub variants: Vec<Variant>,
}

/// A device whose chip fits more boards than this (and none by name) is
/// too generic to suggest any of them.
const MAX_GENERIC_MATCHES: usize = 3;

/// A name match: the engine's confidence 2 (a chip's factory default name)
/// or 3 (the board's own name).
const NAMED: u8 = 2;

/// The boards that match the connected devices, from the engine's matches.
/// Each board is listed once, suggested ones first: per device, the boards
/// whose name matched, or else (if only a few) the boards with the same
/// chip ids.
pub fn connected_boards(devices: &[UsbDevice]) -> Vec<ConnectedBoard> {
    // Catalog boards only: the IDE sends no custom boards.
    let mut found: Vec<ConnectedBoard> = Vec::new();
    for dev in devices {
        let exact = dev.boards.iter().any(|b| b.confidence >= NAMED);
        let generic = dev.boards.iter().filter(|b| b.confidence < NAMED).count();
        for b in &dev.boards {
            let is_exact = b.confidence >= NAMED;
            let suggested = if exact {
                is_exact
            } else {
                generic <= MAX_GENERIC_MATCHES
            };
            let variants: Vec<Variant> = b.variants.iter().map(|(v, _)| v.clone()).collect();
            match found.iter_mut().find(|c| c.board == b.board) {
                Some(c) => {
                    c.exact |= is_exact;
                    c.suggested |= suggested;
                    for v in variants {
                        if !c.variants.contains(&v) {
                            c.variants.push(v);
                        }
                    }
                }
                None => found.push(ConnectedBoard {
                    board: b.board.clone(),
                    exact: is_exact,
                    suggested,
                    variants,
                }),
            }
        }
    }
    found.sort_by_key(|c| (!c.suggested, !c.exact));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device as the engine lists it, matching `boards` (id, confidence).
    fn device(product: &str, boards: &[(&str, u8)]) -> Value {
        let boards: Vec<Value> = boards
            .iter()
            .map(|(b, c)| {
                json!({"board": b, "variants": [
                    {"when": {"connection": "usb", "memory": "flash"}, "confidence": c}
                ]})
            })
            .collect();
        json!({
            "id": format!("usb:1-1:0403:6010:{product}"), "kind": "usb",
            "vid": "0403", "pid": "6010", "product": product,
            "bus": 1, "device": 2, "boards": boards
        })
    }

    fn connected(devices: Vec<Value>) -> Vec<(String, bool)> {
        let list = parse_devices(&json!({ "devices": devices })).unwrap();
        connected_boards(&list.devices)
            .into_iter()
            .filter(|c| c.suggested)
            .map(|c| (c.board, c.exact))
            .collect()
    }

    #[test]
    fn a_name_match_wins() {
        assert_eq!(
            connected(vec![device(
                "Alhambra II v1.0",
                &[("alhambra-ii", 3), ("generic-a", 1), ("generic-b", 1)]
            )]),
            [("alhambra-ii".to_string(), true)]
        );
    }

    #[test]
    fn a_factory_default_name_counts_as_a_name_match() {
        assert_eq!(
            connected(vec![device(
                "Dual RS232-HS",
                &[("icestick", 2), ("generic-a", 1)]
            )]),
            [("icestick".to_string(), true)]
        );
    }

    #[test]
    fn falls_back_to_the_chip_when_few_boards_share_it() {
        assert_eq!(
            connected(vec![device(
                "Something",
                &[("generic-a", 1), ("generic-b", 1)]
            )]),
            [
                ("generic-a".to_string(), false),
                ("generic-b".to_string(), false)
            ]
        );
        let many: Vec<(&str, u8)> = ["a", "b", "c", "d"].iter().map(|b| (*b, 1)).collect();
        assert!(connected(vec![device("Something", &many)]).is_empty());
    }

    #[test]
    fn every_matching_board_is_listed_suggested_or_not() {
        let list = parse_devices(&json!({"devices": [
            device("Alhambra II", &[("alhambra-ii", 3), ("generic-a", 1)])
        ]}))
        .unwrap();
        let all = connected_boards(&list.devices);
        assert_eq!(all.len(), 2);
        assert!(!all[1].suggested);
        assert_eq!(
            all[0].variants,
            [Variant::from([
                ("connection".into(), "usb".into()),
                ("memory".into(), "flash".into())
            ])]
        );
    }

    #[test]
    fn parses_the_engine_device_list() {
        let list = parse_devices(&json!({
            "devices": [{
                "id": "usb:1-1:0403:6010:FTATANKC", "kind": "usb",
                "vid": "0403", "pid": "6010", "manufacturer": "AlhambraBits",
                "product": "Alhambra II v1.0A", "serial_number": "FTATANKC",
                "bus": 1, "device": 7, "build_confidence": 3,
                "boards": [{"board": "alhambra-ii", "variants": [
                    {"when": {"connection": "usb", "memory": "flash"}, "confidence": 3}
                ]}]
            }],
            "unreachable": [{
                "kind": "usb", "vid": "0403", "pid": "6014", "bus": 1, "device": 9,
                "boards": [],
                "action": {"code": "fix_device_permissions", "description": "Allow it."}
            }]
        }))
        .unwrap();
        let d = &list.devices[0];
        assert_eq!(d.id, "usb:1-1:0403:6010:FTATANKC");
        assert_eq!(d.serial_number, "FTATANKC");
        assert_eq!(d.build_confidence, Some(3));
        assert_eq!(d.boards[0].board, "alhambra-ii");
        assert_eq!(d.boards[0].confidence, 3);
        assert_eq!(list.unreachable[0].action, "fix_device_permissions");
        assert!(parse_devices(&json!({})).is_err());
    }
}
