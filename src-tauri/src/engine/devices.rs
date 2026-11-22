//! Connected USB devices (the engine's `list_devices`) and the boards they
//! may be. The engine does the matching and gives each match a level,
//! strong, possible or weak; the IDE only shows and compares levels, never
//! guessing why a device matched (the engine's criteria are internal and
//! change without notice), for Tools > Connected Devices, the board list's
//! device groups and Upload's choice of device.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::catalog::{selector, Variant};
use super::client::{tool_error, Engine, ToolFailure};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    /// The engine's device ID, for `program_board`. Opaque and short-lived.
    pub id: String,
    /// The engine's name for people, e.g. "Alhambra II v1.0A (AlhambraBits,
    /// 0403:6010)".
    pub label: String,
    /// USB vendor and product ids, as 4 lowercase hex digits.
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
    pub serial_number: String,
    /// Its USB release (the engine's `release`, bcdDevice as four hex
    /// digits, such as "0100"); empty when it doesn't report one.
    pub release: String,
    /// The boards it may be, best first (the engine's order).
    pub boards: Vec<DeviceBoard>,
    /// The boards it most likely is, by the engine (none when its matches
    /// are only weak).
    pub likely_boards: Vec<String>,
    /// With a build folder: how well it matches the build's board in the
    /// build's variant.
    pub build_match: Option<MatchLevel>,
    /// The engine's name for it in a sentence, such as "the Alhambra II"
    /// (the selected board when it may be that one), for messages.
    pub who: String,
    /// What it needs for its board's programmer and what it has (the
    /// engine's `driver`): on Windows, for a board whose programmer needs
    /// one, a driver (WinUSB); on Linux, for every board, a udev rule.
    pub driver: Option<Driver>,
    /// Its serial ports (the engine's `serial_ports`), for the Serial
    /// Monitor's Change… list.
    pub serial_ports: Vec<DevicePort>,
    /// The port of its board's UART, the one the Serial Monitor opens (the
    /// engine's `serial_port`); None when the engine can't tell.
    pub serial_port: Option<String>,
}

/// One serial port of a device: its name, its USB interface when known,
/// and what it's for, by the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePort {
    pub port: String,
    pub interface: Option<u8>,
    /// "uart" (the board's serial port), "programmer" (its programmer's
    /// interface), or None when the engine can't tell.
    pub role: Option<String>,
    /// The engine's phrase for people, such as "The Alhambra II's
    /// programming port".
    pub description: String,
}

/// The engine's `driver` for a device (Windows and Linux): what the board's
/// programmer needs (Windows: a driver on a USB interface; Linux: a udev
/// rule), what the device has now, and whether the programmer can use it
/// as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    /// What's needed: "winusb" (Windows) or "udev-rule" (Linux).
    pub needed: String,
    /// On Windows, the USB interface that needs it, e.g. 0 (an FTDI chip's
    /// first channel; its other channels, such as a serial port, keep
    /// theirs). None on Linux: a rule is for the whole device.
    pub interface: Option<u8>,
    /// Windows: the driver it has now, by its service name in lowercase
    /// (e.g. "ftdibus"). Linux: the file of Logix Engine's udev rule for
    /// it, when there's one. None otherwise.
    pub current: Option<String>,
    /// Whether the programmer can use it as it is; false: install_driver.
    pub ready: bool,
    /// The engine's texts for people (the same for every frontend, naming
    /// none): what it needs ("the WinUSB driver", "a udev rule"), its
    /// state, and a short tag while it isn't ready.
    pub label: String,
    pub state: String,
    pub tag: Option<String>,
    /// Whether Install/Add (`install_driver`) and Remove
    /// (`uninstall_driver`) apply, by the engine.
    pub can_add: bool,
    pub can_remove: bool,
    /// The engine's questions before each, with their status and cancel
    /// lines.
    pub add: Option<Consent>,
    pub remove: Option<Consent>,
}

/// One of the engine's questions to ask before a driver or udev rule
/// change (its `add`, `remove`, or the same-USB-IDs refusal's `consent`),
/// shown in Logix IDE's own dialog, with its status and cancel lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Consent {
    pub title: String,
    pub lead: String,
    pub steps: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub confirm: String,
    pub busy: String,
    pub cancelled: String,
}

/// An engine question, if `v` is one.
fn parse_consent(v: &Value) -> Option<Consent> {
    serde_json::from_value(v.clone()).ok()
}

/// The engine's `driver` field, if any.
fn parse_driver(v: &Value) -> Option<Driver> {
    let d = v.as_object()?;
    Some(Driver {
        needed: d.get("needed")?.as_str()?.to_string(),
        interface: d
            .get("interface")
            .and_then(Value::as_u64)
            .and_then(|i| i.try_into().ok()),
        current: d.get("current").and_then(Value::as_str).map(String::from),
        ready: d.get("ready")?.as_bool()?,
        label: v["label"].as_str().unwrap_or("").to_string(),
        state: v["state"].as_str().unwrap_or("").to_string(),
        tag: v["tag"].as_str().map(String::from),
        can_add: v["can_add"].as_bool().unwrap_or(false),
        can_remove: v["can_remove"].as_bool().unwrap_or(false),
        add: parse_consent(&v["add"]),
        remove: parse_consent(&v["remove"]),
    })
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
    pub variants: Vec<(Variant, MatchLevel)>,
}

/// A device the engine sees but can't open (a missing driver or
/// permissions), and whose USB ids fit a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreachableDevice {
    /// The engine's device ID, for `install_driver` (not for
    /// `program_board`, which refuses it).
    pub id: String,
    /// The engine's name for people, e.g. "USB device (0403:6014)".
    pub label: String,
    pub vid: String,
    pub pid: String,
    /// The engine's suggested fix, e.g. `fix_device_permissions`.
    pub action: String,
    /// The engine's name for it in a sentence, as for `UsbDevice`.
    pub who: String,
    /// What to do, for people, by the engine.
    pub fix: String,
    /// As for `UsbDevice`: on Windows, the driver it needs and has.
    pub driver: Option<Driver>,
    /// The boards whose USB IDs it has (all weak: its names can't be read
    /// until it has its driver).
    pub boards: Vec<DeviceBoard>,
}

/// What `list_devices` found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceList {
    pub devices: Vec<UsbDevice>,
    pub unreachable: Vec<UnreachableDevice>,
}

/// The connected USB devices, with the boards they may be: the catalog's
/// and the project's own, from its custom boards file.
pub async fn list_devices(
    engine: &Engine,
    boards_file: Option<&super::BoardsFolder>,
    board: Option<&str>,
) -> Result<DeviceList> {
    let mut args = match boards_file {
        Some(file) => json!({
            "custom_boards_folder": file.path,
            "display_paths": file.display_paths(),
        }),
        None => json!({}),
    };
    // The selected board: the engine names devices by it and lists its
    // devices first.
    if let Some(board) = board {
        args["board"] = json!(board);
    }
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
            label: text(d, "label"),
            vid: text(d, "vid"),
            pid: text(d, "pid"),
            manufacturer: text(d, "manufacturer"),
            product: text(d, "product"),
            serial_number: text(d, "serial_number"),
            release: text(d, "release"),
            boards: d["boards"]
                .as_array()
                .map(|boards| boards.iter().map(parse_board).collect())
                .unwrap_or_default(),
            likely_boards: d["likely_boards"]
                .as_array()
                .map(|l| {
                    l.iter()
                        .filter_map(|b| b.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            build_match: MatchLevel::parse(&d["build_match"]),
            who: text(d, "who"),
            driver: parse_driver(&d["driver"]),
            serial_ports: d["serial_ports"]
                .as_array()
                .map(|ports| {
                    ports
                        .iter()
                        .filter_map(|p| {
                            Some(DevicePort {
                                role: p["role"].as_str().map(String::from),
                                description: p["description"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                port: p["port"].as_str()?.to_string(),
                                interface: p["interface"]
                                    .as_u64()
                                    .and_then(|i| u8::try_from(i).ok()),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            serial_port: d["serial_port"].as_str().map(String::from),
        })
        .collect();
    let unreachable = result["unreachable"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|d| UnreachableDevice {
                    id: text(d, "id"),
                    label: text(d, "label"),
                    vid: text(d, "vid"),
                    pid: text(d, "pid"),
                    action: d["action"]["code"].as_str().unwrap_or("").to_string(),
                    who: text(d, "who"),
                    fix: text(d, "fix"),
                    driver: parse_driver(&d["driver"]),
                    boards: d["boards"]
                        .as_array()
                        .map(|boards| boards.iter().map(parse_board).collect())
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(DeviceList {
        devices,
        unreachable,
    })
}

/// The engine's action code for a device that needs a driver first.
pub const INSTALL_DRIVERS: &str = "install_drivers";

/// The engine's action code when `install_driver` refuses because other
/// connected devices have the same USB IDs (they'd get WinUSB too).
pub const CONFIRM_SAME_IDS: &str = "confirm_same_ids";

/// What `install_driver` did.
#[derive(Debug, Clone, PartialEq)]
pub enum DriverInstall {
    /// Installed (or ready already): the device's ID now, and what was done
    /// for people (the engine's `summary`, a sentence each).
    Done {
        device: String,
        installed: bool,
        summary: Vec<String>,
    },
    /// Refused: other connected devices have the same USB IDs. The engine's
    /// question names them; retry with `allow_same_ids` only if the user
    /// agrees they get WinUSB too.
    SameIds(Consent),
}

/// Installs WinUSB on a device's interface (Windows) or adds its udev rule
/// (Linux): the engine's `install_driver`. The user must have agreed;
/// Windows then asks for permission naming Logix Engine, Linux for the
/// user's password. `build_folder` picks the build's board.
pub async fn install_driver(
    engine: &Engine,
    device: &str,
    boards_file: Option<&super::BoardsFolder>,
    build_folder: Option<&std::path::Path>,
    allow_same_ids: bool,
    board: Option<&str>,
    options: super::client::CallOptions,
) -> Result<DriverInstall> {
    let mut args = json!({ "device": device, "allow_same_ids": allow_same_ids });
    if let Some(board) = board {
        args["board"] = json!(board);
    }
    if let Some(file) = boards_file {
        args["custom_boards_folder"] = json!(file.path);
        args["display_paths"] = file.display_paths();
    }
    if let Some(folder) = build_folder {
        args["build_folder"] = json!(folder);
    }
    match engine.call_with("install_driver", args, options).await? {
        Ok(r) => Ok(install_result(&r, device)),
        Err(e) => match same_ids(&e) {
            Some(consent) => Ok(DriverInstall::SameIds(consent)),
            None => Err(ToolFailure::from_value(&e).into()),
        },
    }
}

/// `install_driver`'s result: the device's ID now (it can change once the
/// driver lets the engine read its serial number).
pub(super) fn install_result(r: &Value, device: &str) -> DriverInstall {
    DriverInstall::Done {
        device: r["device"].as_str().unwrap_or(device).to_string(),
        installed: r["installed"].as_bool().unwrap_or(true),
        summary: strings(&r["summary"]),
    }
}

/// A list of texts, such as the engine's `summary`.
fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|l| {
            l.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// The engine's question when its error is the same-USB-IDs refusal.
pub(super) fn same_ids(error: &Value) -> Option<Consent> {
    if error["action"]["code"] != CONFIRM_SAME_IDS {
        return None;
    }
    parse_consent(&error["data"]["consent"])
}

/// Removes the WinUSB driver libwdi installed for a device's interface
/// (Windows), so Windows restores its original driver, or Logix Engine's
/// udev rule (Linux), for every device with the same USB IDs: the engine's
/// `uninstall_driver`. Returns what was done, for people (the engine's
/// `summary`, a sentence each).
pub async fn uninstall_driver(
    engine: &Engine,
    device: &str,
    boards_file: Option<&super::BoardsFolder>,
    board: Option<&str>,
    options: super::client::CallOptions,
) -> Result<Vec<String>> {
    let mut args = json!({ "device": device });
    if let Some(board) = board {
        args["board"] = json!(board);
    }
    if let Some(file) = boards_file {
        args["custom_boards_folder"] = json!(file.path);
        args["display_paths"] = file.display_paths();
    }
    let r = engine
        .call_with("uninstall_driver", args, options)
        .await?
        .map_err(|e| anyhow::Error::from(ToolFailure::from_value(&e)))?;
    Ok(strings(&r["summary"]))
}

fn parse_board(b: &Value) -> DeviceBoard {
    let variants: Vec<(Variant, MatchLevel)> = b["variants"]
        .as_array()
        .map(|vs| {
            vs.iter()
                .map(|v| {
                    let level = MatchLevel::parse(&v["match"]).unwrap_or(MatchLevel::Weak);
                    (selector(&v["selector"]), level)
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
    /// Its name for people, the engine's `label`.
    pub name: String,
    /// Its serial number, empty when it has none.
    pub serial: String,
    /// The boards it may be, strongest first (the engine's order).
    pub boards: Vec<ConnectedBoard>,
}

/// The connected devices that fit catalog boards, each with the boards it
/// may be, strongest first, from the engine's matches and levels.
pub fn connected_devices(devices: &[UsbDevice]) -> Vec<ConnectedDevice> {
    let mut found = Vec::new();
    for dev in devices {
        let mut its: Vec<ConnectedBoard> = Vec::new();
        for b in &dev.boards {
            // The engine reports each variant by its selector: its values.
            let mut variants: Vec<Variant> = Vec::new();
            for (v, _) in &b.variants {
                if !variants.contains(v) {
                    variants.push(v.clone());
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
        // The engine lists them strongest first.
        found.push(ConnectedDevice {
            id: dev.id.clone(),
            name: dev.label.clone(),
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
            .map(|(b, l)| json!({"board": b, "variants": [{"selector": {"connection": "usb", "memory": "flash"}, "match": l}]}))
            .collect();
        json!({
            "id": format!("usb:1-1:0403:6010:{product}"), "kind": "usb",
            "label": format!("{product} (0403:6010)"),
            "vid": "0403", "pid": "6010", "product": product,
            "bus": 1, "device": 2, "boards": boards
        })
    }

    #[test]
    fn each_device_lists_its_boards_in_the_engines_order() {
        // The engine lists them strongest first.
        let list = parse_devices(&json!({"devices": [
            device("Alhambra II", &[("alhambra-ii", "strong"), ("generic-a", "weak")]),
            device("Something", &[("generic-a", "possible")]),
            device("Unknown", &[]),
        ]}))
        .unwrap();
        let all = connected_devices(&list.devices);
        assert_eq!(all.len(), 2, "a device that fits no board is left out");
        assert_eq!(all[0].name, "Alhambra II (0403:6010)");
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
                "label": "Alhambra II v1.0A (AlhambraBits, 0403:6010)",
                "likely_boards": ["alhambra-ii"],
                "vid": "0403", "pid": "6010", "manufacturer": "AlhambraBits",
                "product": "Alhambra II v1.0A", "serial_number": "FTATANKC",
                "bus": 1, "device": 7, "build_match": "strong",
                "boards": [{"board": "alhambra-ii", "variants": [{"selector": {"connection": "usb", "memory": "flash"}, "match": "strong"}]}]
            }],
            "unreachable": [{
                "kind": "usb", "label": "USB device (0403:6014)",
                "vid": "0403", "pid": "6014", "bus": 1, "device": 9,
                "boards": [{"board": "icestick", "variants": [{"selector": {}, "match": "weak"}]}],
                "action": {"code": "fix_device_permissions", "description": "Allow it."}
            }]
        }))
        .unwrap();
        let d = &list.devices[0];
        assert_eq!(d.id, "usb:1-1:0403:6010:FTATANKC");
        assert_eq!(d.serial_number, "FTATANKC");
        assert_eq!(d.label, "Alhambra II v1.0A (AlhambraBits, 0403:6010)");
        assert_eq!(d.likely_boards, ["alhambra-ii"]);
        assert_eq!(list.unreachable[0].label, "USB device (0403:6014)");
        assert_eq!(d.build_match, Some(MatchLevel::Strong));
        assert_eq!(d.boards[0].board, "alhambra-ii");
        assert_eq!(d.boards[0].level, MatchLevel::Strong);
        assert_eq!(list.unreachable[0].action, "fix_device_permissions");
        // Its boards, for Connected Devices' note on what it may be.
        assert_eq!(list.unreachable[0].boards[0].board, "icestick");
        assert_eq!(list.unreachable[0].boards[0].level, MatchLevel::Weak);
        assert!(parse_devices(&json!({})).is_err());
    }
}
