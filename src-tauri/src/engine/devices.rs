//! Connected USB devices (the engine's `list_devices`), and which boards
//! they are, for the board list's "Connected" group: the USB filters of
//! every configuration of every board (from `get_catalog`) matched against
//! the devices. (Which device Upload uses is the engine's decision.)

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};

use super::catalog::Variant;
use super::client::{tool_error, Engine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    /// USB vendor and product ids, as 4 lowercase hex digits.
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
    pub serial_number: String,
}

/// The connected USB devices.
pub async fn list_devices(engine: &Engine) -> Result<Vec<UsbDevice>> {
    let result = engine
        .call("list_devices", json!({}), None)
        .await?
        .map_err(|e| tool_error("list_devices", &e))?;
    parse_devices(&result)
}

fn parse_devices(result: &Value) -> Result<Vec<UsbDevice>> {
    let list = result["devices"]
        .as_array()
        .context("unexpected device list from the engine")?;
    let text = |d: &Value, key: &str| d[key].as_str().unwrap_or("").trim().to_string();
    Ok(list
        .iter()
        .filter(|d| d["kind"] == "usb")
        .map(|d| UsbDevice {
            vid: text(d, "vid"),
            pid: text(d, "pid"),
            manufacturer: text(d, "manufacturer"),
            product: text(d, "product"),
            serial_number: text(d, "serial_number"),
        })
        .collect())
}

/// The USB filter of one configuration of a board.
#[derive(Debug, Clone)]
pub struct UsbRule {
    pub board: String,
    /// The configuration's dimension values (empty for most boards).
    pub variant: Variant,
    vid: String,
    pid: String,
    product: Option<Regex>,
}

/// A board that matches a connected device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedBoard {
    pub board: String,
    /// The board's product name matched too, not just the USB chip's ids
    /// (which many boards share).
    pub exact: bool,
    /// Worth listing in the board picker's "Connected" group: a product
    /// name match, or one of only a few boards with the chip's ids (generic
    /// FTDI ids fit dozens). Boards that aren't suggested still match the
    /// device, so the selected one shows as connected.
    pub suggested: bool,
    /// The configurations whose filter matched, e.g. `{"connection":
    /// "ft232h"}` for a board with several adapters (one empty variant for
    /// most boards).
    pub variants: Vec<Variant>,
}

/// A device whose chip fits more boards than this (and none by product
/// name) is too generic to suggest any of them (generic FTDI ids fit
/// dozens).
const MAX_GENERIC_MATCHES: usize = 3;

/// The USB filters of the catalog's boards (`get_catalog`'s `boards`).
/// Boards without one, or with a pattern Rust can't read, are skipped.
pub fn usb_rules(boards: &Value) -> Vec<UsbRule> {
    let Some(boards) = boards.as_object() else {
        return Vec::new();
    };
    let mut rules = Vec::new();
    for (board, def) in boards {
        let Some(configurations) = def["variants"]["configurations"].as_array() else {
            continue;
        };
        for c in configurations {
            let usb = &c["usb"];
            let text = |key: &str| usb[key].as_str().map(str::to_lowercase);
            let (Some(vid), Some(pid)) = (text("vid"), text("pid")) else {
                continue;
            };
            let product = match usb["product_regex"].as_str() {
                Some(pattern) => match Regex::new(pattern) {
                    Ok(re) => Some(re),
                    Err(_) => continue,
                },
                None => None,
            };
            rules.push(UsbRule {
                board: board.clone(),
                variant: serde_json::from_value(c["when"].clone()).unwrap_or_default(),
                vid,
                pid,
                product,
            });
        }
    }
    rules
}

/// The boards that match the connected USB devices, as the engine matches
/// them (equal ids, and the product name contains a match for the pattern,
/// if the board has one). Each board is listed once, suggested ones first:
/// per device, the boards whose product name matches, or else (if only a
/// few) the boards with the same chip ids and no product pattern.
pub fn match_boards(rules: &[UsbRule], devices: &[UsbDevice]) -> Vec<ConnectedBoard> {
    let mut found: Vec<ConnectedBoard> = Vec::new();
    for dev in devices {
        let (vid, pid) = (dev.vid.to_lowercase(), dev.pid.to_lowercase());
        let matching: Vec<&UsbRule> = rules
            .iter()
            .filter(|r| r.vid == vid && r.pid == pid)
            .filter(|r| {
                r.product
                    .as_ref()
                    .is_none_or(|re| re.is_match(&dev.product))
            })
            .collect();
        let exact = matching.iter().any(|r| r.product.is_some());
        let mut generic_boards: Vec<&str> = matching
            .iter()
            .filter(|r| r.product.is_none())
            .map(|r| r.board.as_str())
            .collect();
        generic_boards.dedup();
        let suggest = |r: &UsbRule| {
            if exact {
                r.product.is_some()
            } else {
                generic_boards.len() <= MAX_GENERIC_MATCHES
            }
        };
        for rule in matching {
            let is_exact = rule.product.is_some();
            let suggested = suggest(rule);
            match found.iter_mut().find(|c| c.board == rule.board) {
                Some(c) => {
                    c.exact |= is_exact;
                    c.suggested |= suggested;
                    if !c.variants.contains(&rule.variant) {
                        c.variants.push(rule.variant.clone());
                    }
                }
                None => found.push(ConnectedBoard {
                    board: rule.board.clone(),
                    exact: is_exact,
                    suggested,
                    variants: vec![rule.variant.clone()],
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

    fn boards() -> Value {
        let board = |vid: &str, pid: &str, regex: Option<&str>| {
            let mut usb = json!({"vid": vid, "pid": pid});
            if let Some(r) = regex {
                usb["product_regex"] = json!(r);
            }
            json!({"variants": {"configurations": [{"usb": usb}]}})
        };
        json!({
            "alhambra-ii": board("0403", "6010", Some("^Alhambra II.*")),
            "icebreaker": board("0403", "6010", Some("^(Dual RS232-HS)|(iCEBreaker.*)")),
            "generic-a": board("0403", "6010", None),
            "generic-b": board("0403", "6010", None),
            "tinyfpga": board("1D50", "6130", None),
            "no-usb": {"variants": {"configurations": [{}]}},
            "stick": {"variants": {"configurations": [
                {"when": {"connection": "dfu"}, "usb": {"vid": "1209", "pid": "5af1"}},
                {"when": {"connection": "ft232h"}, "usb": {"vid": "0403", "pid": "6014"}}
            ]}},
            "bad-regex": board("1234", "5678", Some("(")),
        })
    }

    fn device(vid: &str, pid: &str, product: &str) -> UsbDevice {
        UsbDevice {
            vid: vid.into(),
            pid: pid.into(),
            manufacturer: String::new(),
            product: product.into(),
            serial_number: String::new(),
        }
    }

    /// The boards suggested in the board picker's Connected group.
    fn connected(devices: Vec<UsbDevice>) -> Vec<(String, bool)> {
        match_boards(&usb_rules(&boards()), &devices)
            .into_iter()
            .filter(|c| c.suggested)
            .map(|c| (c.board, c.exact))
            .collect()
    }

    #[test]
    fn reads_the_rules_from_the_catalog() {
        let rules = usb_rules(&boards());
        let mut names: Vec<_> = rules.iter().map(|r| r.board.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            [
                "alhambra-ii",
                "generic-a",
                "generic-b",
                "icebreaker",
                "stick",
                "stick",
                "tinyfpga"
            ]
        );
        assert!(usb_rules(&Value::Null).is_empty());
    }

    #[test]
    fn a_product_name_match_wins() {
        assert_eq!(
            connected(vec![device("0403", "6010", "Alhambra II v1.0")]),
            [("alhambra-ii".to_string(), true)]
        );
    }

    #[test]
    fn falls_back_to_the_chip_when_few_boards_share_it() {
        assert_eq!(
            connected(vec![device("0403", "6010", "Something else")]),
            [
                ("generic-a".to_string(), false),
                ("generic-b".to_string(), false)
            ]
        );
        // Ids are compared regardless of case.
        assert_eq!(
            connected(vec![device("1d50", "6130", "")]),
            [("tinyfpga".to_string(), false)]
        );
    }

    #[test]
    fn lists_each_board_once_exact_first() {
        assert_eq!(
            connected(vec![
                device("1d50", "6130", ""),
                device("0403", "6010", "iCEBreaker V1.0e"),
                device("0403", "6010", "iCEBreaker V1.0e"),
            ]),
            [
                ("icebreaker".to_string(), true),
                ("tinyfpga".to_string(), false)
            ]
        );
    }

    #[test]
    fn says_which_configuration_matched() {
        let found = match_boards(&usb_rules(&boards()), &[device("0403", "6014", "")]);
        assert_eq!(found[0].board, "stick");
        assert_eq!(
            found[0].variants,
            [Variant::from([("connection".into(), "ft232h".into())])]
        );
        let plain = match_boards(&usb_rules(&boards()), &[device("1d50", "6130", "")]);
        assert_eq!(plain[0].variants, [Variant::new()]);
    }

    /// An Arty (FTDI ids, "Digilent USB Device") also matches the Basys3's
    /// product pattern: the Arty still matches, as the engine sees it, so
    /// the toolbar shows it as connected when it's the selected board.
    #[test]
    fn every_matching_board_is_listed_suggested_or_not() {
        let mut catalog = boards();
        catalog["basys3"] = json!({"variants": {"configurations": [
            {"usb": {"vid": "0403", "pid": "6010", "product_regex": "^Digilent USB Device"}}]}});
        catalog["arty"] = json!({"variants": {"configurations": [
            {"usb": {"vid": "0403", "pid": "6010"}}]}});
        let found = match_boards(
            &usb_rules(&catalog),
            &[device("0403", "6010", "Digilent USB Device")],
        );
        let list: Vec<(&str, bool)> = found
            .iter()
            .map(|c| (c.board.as_str(), c.suggested))
            .collect();
        // Suggested first; the generic FTDI boards match too.
        assert_eq!(list[0], ("basys3", true));
        assert!(list.contains(&("arty", false)));
        assert!(list.contains(&("generic-a", false)));
        // Not the boards whose product pattern doesn't match.
        assert!(!list.iter().any(|(b, _)| *b == "alhambra-ii"));
    }

    #[test]
    fn nothing_for_unknown_devices() {
        assert!(connected(vec![device("05ac", "8262", "Apple Keyboard")]).is_empty());
    }

    #[test]
    fn parses_the_engine_device_list() {
        let result = json!({"devices": [{
            "kind": "usb", "vid": "0403", "pid": "6010",
            "manufacturer": "AlhambraBits", "product": "Alhambra II v1.0A",
            "serial_number": "FTATANKC", "bus": 1, "device": 7
        }, {"kind": "usb", "vid": "05ac", "pid": "8262"}]});
        let devices = parse_devices(&result).unwrap();
        assert_eq!(devices[0].product, "Alhambra II v1.0A");
        assert_eq!(devices[0].serial_number, "FTATANKC");
        assert_eq!(devices[1].product, "");
        assert!(parse_devices(&json!({})).is_err());
    }
}
