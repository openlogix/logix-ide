//! Connected USB devices (the engine's `list_devices`) and the boards they
//! may be. The engine does the matching and gives each match a level,
//! strong, possible or weak; the IDE only shows and compares levels, never
//! guessing why a device matched (the engine's criteria are internal and
//! change without notice), for Tools > Connected Devices, the board list's
//! device groups and Upload's choice of device.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::catalog::{variant_values, Board, Variant};
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
    /// With a build folder: how well it matches the build's board in the
    /// build's variant.
    pub build_match: Option<MatchLevel>,
}

/// The engine's match level: what a match means for the user (weak <
/// possible < strong), never its cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchLevel {
    /// It fits, but so do many boards.
    Weak,
    /// It could be the board, but a few other boards look the same.
    Possible,
    /// It's very likely the board.
    Strong,
}

impl MatchLevel {
    /// The engine's `match` value; unknown values (a newer engine) count as
    /// weak.
    fn parse(v: &Value) -> Option<MatchLevel> {
        Some(match v.as_str()? {
            "strong" => MatchLevel::Strong,
            "possible" => MatchLevel::Possible,
            _ => MatchLevel::Weak,
        })
    }
}

/// A board a device may be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBoard {
    pub board: String,
    /// The strongest level of its variants.
    pub level: MatchLevel,
    /// The variants the device fits (the engine's variant IDs), with their
    /// levels.
    #[serde(skip)]
    pub variants: Vec<(String, MatchLevel)>,
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

/// The connected USB devices, with the boards they may be: the catalog's
/// and the project's own, from the custom boards file at `boards_file`.
pub async fn list_devices(
    engine: &Engine,
    boards_file: Option<&std::path::Path>,
) -> Result<DeviceList> {
    let args = match boards_file {
        Some(file) => json!({ "custom_boards_file": file }),
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
            build_match: MatchLevel::parse(&d["build_match"]),
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
    let variants: Vec<(String, MatchLevel)> = b["variants"]
        .as_array()
        .map(|vs| {
            vs.iter()
                .map(|v| {
                    let id = v["id"].as_str().unwrap_or_default().to_string();
                    let level = MatchLevel::parse(&v["match"]).unwrap_or(MatchLevel::Weak);
                    (id, level)
                })
                .collect()
        })
        .unwrap_or_default();
    DeviceBoard {
        board: b["board"].as_str().unwrap_or("").to_string(),
        level: variants
            .iter()
            .map(|(_, l)| *l)
            .max()
            .unwrap_or(MatchLevel::Weak),
        variants,
    }
}

/// A board that a connected device may be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedBoard {
    pub board: String,
    /// The engine's level for this board (its strongest variant's).
    pub level: MatchLevel,
    /// The configurations whose filter matched, e.g. `{"connection":
    /// "ft232h", "memory": "ram"}`.
    pub variants: Vec<Variant>,
}

/// A connected device that fits at least one board, with the boards it may
/// be (one device is one board, so these are alternatives).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedDevice {
    /// The engine's device ID: unique among the connected devices.
    pub id: String,
    /// Its name for people: the product name, else the manufacturer, else
    /// "USB device <vid>:<pid>".
    pub name: String,
    /// Its serial number, empty when it has none.
    pub serial: String,
    /// The boards it may be, strongest first (the engine's order).
    pub boards: Vec<ConnectedBoard>,
}

/// The connected devices that fit catalog boards, each with the boards it
/// may be, strongest first, from the engine's matches and levels.
pub fn connected_devices(devices: &[UsbDevice], boards: &[Board]) -> Vec<ConnectedDevice> {
    let mut found = Vec::new();
    for dev in devices {
        let mut its: Vec<ConnectedBoard> = Vec::new();
        for b in &dev.boards {
            // The engine reports variant IDs; the UI works with their values.
            let info = boards.iter().find(|x| x.id == b.board);
            let mut variants: Vec<Variant> = Vec::new();
            for v in b
                .variants
                .iter()
                .filter_map(|(id, _)| info.and_then(|i| variant_values(i, id)))
            {
                if !variants.contains(&v) {
                    variants.push(v);
                }
            }
            its.push(ConnectedBoard {
                board: b.board.clone(),
                level: b.level,
                variants,
            });
        }
        if its.is_empty() {
            continue;
        }
        // Stable: equal levels keep the engine's order.
        its.sort_by_key(|c| std::cmp::Reverse(c.level));
        let name = [&dev.product, &dev.manufacturer]
            .into_iter()
            .find(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("USB device {}:{}", dev.vid, dev.pid));
        found.push(ConnectedDevice {
            id: dev.id.clone(),
            name,
            serial: dev.serial_number.clone(),
            boards: its,
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device as the engine lists it, matching `boards` (id, level).
    fn device(product: &str, boards: &[(&str, &str)]) -> Value {
        let boards: Vec<Value> = boards
            .iter()
            .map(|(b, l)| json!({"board": b, "variants": [{"id": "usb-flash", "match": l}]}))
            .collect();
        json!({
            "id": format!("usb:1-1:0403:6010:{product}"), "kind": "usb",
            "vid": "0403", "pid": "6010", "product": product,
            "bus": 1, "device": 2, "boards": boards
        })
    }

    /// Catalog boards whose variant usb-flash is USB and flash.
    fn boards() -> Vec<Board> {
        [
            "alhambra-ii",
            "generic-a",
            "generic-b",
            "icestick",
            "a",
            "b",
            "c",
            "d",
        ]
        .iter()
        .map(|id| Board {
            id: id.to_string(),
            description: id.to_string(),
            arch: "ice40".into(),
            arch_label: String::new(),
            arch_order: 0,
            pin_extension: None,
            custom: false,
            dimensions: Vec::new(),
            configurations: vec![super::super::catalog::Configuration {
                id: "usb-flash".into(),
                when: Variant::from([
                    ("connection".into(), "usb".into()),
                    ("memory".into(), "flash".into()),
                ]),
                tested: true,
                user_notes: None,
            }],
        })
        .collect()
    }

    #[test]
    fn each_device_lists_its_boards_strongest_first() {
        let list = parse_devices(&json!({"devices": [
            device("Alhambra II", &[("generic-a", "weak"), ("alhambra-ii", "strong")]),
            device("Something", &[("generic-a", "possible")]),
            device("Unknown", &[]),
        ]}))
        .unwrap();
        let all = connected_devices(&list.devices, &boards());
        assert_eq!(all.len(), 2, "a device that fits no board is left out");
        assert_eq!(all[0].name, "Alhambra II");
        assert_eq!(all[0].id, "usb:1-1:0403:6010:Alhambra II");
        let levels: Vec<_> = all[0]
            .boards
            .iter()
            .map(|b| (b.board.as_str(), b.level))
            .collect();
        assert_eq!(
            levels,
            [
                ("alhambra-ii", MatchLevel::Strong),
                ("generic-a", MatchLevel::Weak)
            ]
        );
        assert_eq!(
            all[0].boards[0].variants,
            [Variant::from([
                ("connection".into(), "usb".into()),
                ("memory".into(), "flash".into())
            ])]
        );
        // The same board on two devices is listed under each.
        assert_eq!(all[1].boards[0].board, "generic-a");
        assert_eq!(all[1].boards[0].level, MatchLevel::Possible);
    }

    #[test]
    fn unknown_levels_count_as_weak() {
        let list = parse_devices(&json!({"devices": [device("X", &[("a", "someday")])]})).unwrap();
        assert_eq!(list.devices[0].boards[0].level, MatchLevel::Weak);
    }

    #[test]
    fn parses_the_engine_device_list() {
        let list = parse_devices(&json!({
            "devices": [{
                "id": "usb:1-1:0403:6010:FTATANKC", "kind": "usb",
                "vid": "0403", "pid": "6010", "manufacturer": "AlhambraBits",
                "product": "Alhambra II v1.0A", "serial_number": "FTATANKC",
                "bus": 1, "device": 7, "build_match": "strong",
                "boards": [{"board": "alhambra-ii", "variants": [{"id": "usb-flash", "match": "strong"}]}]
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
        assert_eq!(d.build_match, Some(MatchLevel::Strong));
        assert_eq!(d.boards[0].board, "alhambra-ii");
        assert_eq!(d.boards[0].level, MatchLevel::Strong);
        assert_eq!(list.unreachable[0].action, "fix_device_permissions");
        assert!(parse_devices(&json!({})).is_err());
    }
}
