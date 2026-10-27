//! Which boards are connected: Apio's board USB rules matched against the
//! connected USB devices, for the board list's "Connected" group.
//!
//! TODO(revisit): `apio api get-boards` doesn't include the boards' USB ids,
//! so the rules are read from Apio's `definitions/boards.jsonc`, and the
//! matching follows Apio's (equal vid and pid, and the optional
//! "product-regex" searched in the device's product name). Remove this once
//! Apio reports the matching boards itself (requested upstream).

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use super::devices::Devices;

/// One board's USB identity, from Apio's board definitions.
#[derive(Debug, Clone)]
pub struct UsbRule {
    pub board: String,
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
}

/// A device whose chip fits more boards than this (and none by product
/// name) is too generic to suggest any of them.
const MAX_GENERIC_MATCHES: usize = 3;

/// Reads the USB rules from Apio's `boards.jsonc`. Boards without a "usb"
/// entry, or with a pattern Rust can't read, are skipped.
pub fn parse_usb_rules(jsonc: &str) -> Result<Vec<UsbRule>> {
    let json: Value =
        serde_json::from_str(&strip_jsonc_comments(jsonc)).context("unreadable boards.jsonc")?;
    let boards = json.as_object().context("unexpected boards.jsonc")?;
    let mut rules = Vec::new();
    for (board, def) in boards {
        let Some(usb) = def.get("usb") else { continue };
        let text = |key: &str| usb.get(key).and_then(Value::as_str).map(str::to_lowercase);
        let (Some(vid), Some(pid)) = (text("vid"), text("pid")) else {
            continue;
        };
        let product = match usb.get("product-regex").and_then(Value::as_str) {
            Some(pattern) => match Regex::new(pattern) {
                Ok(re) => Some(re),
                Err(_) => continue,
            },
            None => None,
        };
        rules.push(UsbRule {
            board: board.clone(),
            vid,
            pid,
            product,
        });
    }
    Ok(rules)
}

/// The boards that match the connected USB devices: per device, the boards
/// whose product name matches, or else (if only a few) the boards with the
/// same chip ids and no product name. Each board is listed once, exact
/// matches first.
pub fn match_boards(rules: &[UsbRule], devices: &Devices) -> Vec<ConnectedBoard> {
    let mut found: Vec<ConnectedBoard> = Vec::new();
    for dev in &devices.usb {
        let (vid, pid) = (dev.vid.to_lowercase(), dev.pid.to_lowercase());
        let same_chip = rules.iter().filter(|r| r.vid == vid && r.pid == pid);
        let exact: Vec<&UsbRule> = same_chip
            .clone()
            .filter(|r| {
                r.product
                    .as_ref()
                    .is_some_and(|re| re.is_match(&dev.product))
            })
            .collect();
        let generic: Vec<&UsbRule> = same_chip.filter(|r| r.product.is_none()).collect();
        let (chosen, is_exact) = if !exact.is_empty() {
            (exact, true)
        } else if generic.len() <= MAX_GENERIC_MATCHES {
            (generic, false)
        } else {
            continue;
        };
        for rule in chosen {
            match found.iter_mut().find(|c| c.board == rule.board) {
                Some(c) => c.exact |= is_exact,
                None => found.push(ConnectedBoard {
                    board: rule.board.clone(),
                    exact: is_exact,
                }),
            }
        }
    }
    found.sort_by_key(|c| !c.exact);
    found
}

/// Removes `//` comments outside strings (JSON with comments).
fn strip_jsonc_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apio::devices::UsbDevice;

    const BOARDS: &str = r#"
// Apio boards definitions.
{
  // https://github.com/FPGAwars/Alhambra-II-FPGA
  "alhambra-ii": {
    "description": "Alhambra II",
    "usb": { "vid": "0403", "pid": "6010", "product-regex": "^Alhambra II.*" }
  },
  "icebreaker": {
    "description": "iCEBreaker // with a slash in a string",
    "usb": { "vid": "0403", "pid": "6010", "product-regex": "^(Dual RS232-HS)|(iCEBreaker.*)" }
  },
  "generic-a": { "usb": { "vid": "0403", "pid": "6010" } },
  "generic-b": { "usb": { "vid": "0403", "pid": "6010" } },
  "tinyfpga": { "usb": { "vid": "1D50", "pid": "6130" } },
  "no-usb": { "description": "No USB" }
}
"#;

    fn device(vid: &str, pid: &str, product: &str) -> UsbDevice {
        UsbDevice {
            vid: vid.into(),
            pid: pid.into(),
            manufacturer: String::new(),
            product: product.into(),
            serial_number: String::new(),
            device_type: String::new(),
        }
    }

    fn connected(devices: Vec<UsbDevice>) -> Vec<(String, bool)> {
        let rules = parse_usb_rules(BOARDS).unwrap();
        let devices = Devices {
            usb: devices,
            serial: vec![],
        };
        match_boards(&rules, &devices)
            .into_iter()
            .map(|c| (c.board, c.exact))
            .collect()
    }

    #[test]
    fn reads_the_rules_with_comments() {
        let rules = parse_usb_rules(BOARDS).unwrap();
        let mut names: Vec<_> = rules.iter().map(|r| r.board.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            [
                "alhambra-ii",
                "generic-a",
                "generic-b",
                "icebreaker",
                "tinyfpga"
            ]
        );
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

    /// All of Apio's real rules must parse: `LOGIX_BOARDS_JSONC=<apio
    /// packages>/definitions/boards.jsonc cargo test real_usb_rules --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_usb_rules() {
        let path = std::env::var("LOGIX_BOARDS_JSONC").expect("LOGIX_BOARDS_JSONC");
        let text = std::fs::read_to_string(path).unwrap();
        let json: Value = serde_json::from_str(&strip_jsonc_comments(&text)).unwrap();
        let with_usb = json
            .as_object()
            .unwrap()
            .values()
            .filter(|b| b.get("usb").is_some())
            .count();
        let rules = parse_usb_rules(&text).unwrap();
        println!("{} of {with_usb} USB rules read", rules.len());
        assert_eq!(rules.len(), with_usb);
    }

    #[test]
    fn nothing_for_unknown_devices() {
        assert!(connected(vec![device("05ac", "8262", "Apple Keyboard")]).is_empty());
    }
}
