//! The project's debug file, `debug.toml`: the nets Debug watches and drives.
//!
//! The file in the project folder is the only copy (no settings, no JSON
//! copy). Logix Engine reads it in a debug build (`build`'s `debug_file`) and
//! owns its format (its `docs/debugger.md`, schema `logix-debug-v1.json`).
//! The IDE reads it here to know which nets are chosen, and its net picker
//! rewrites only the parts it owns (`clock`, a missing `uplink`, the
//! `[[watch]]` and `[[drive]]` entries), with `toml_edit`, so the user's
//! comments and other settings (triggers, counters, formats, names) stay.

use std::path::Path;

use serde::Serialize;
use toml_edit::{value, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

use crate::engine::DEBUG_FILE;

/// The first line of a new debug file, so editors can check it.
const SCHEMA_LINE: &str = "#:schema https://www.openlogix.org/schemas/logix-debug-v1.json\n";

/// What the IDE needs from a project's debug file.
#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DebugConfig {
    /// Whether the project has a debug file.
    pub exists: bool,
    /// The clock net, empty when unset.
    pub clock: String,
    /// The watched nets, by path (only entries with one net).
    pub watch: Vec<String>,
    /// The driven nets, by path (only entries with one net).
    pub drive: Vec<String>,
    /// Whether any net is chosen at all, including entries the picker
    /// doesn't show (a net list).
    pub has_nets: bool,
}

/// Reads the project's debug file; a missing file is an empty config.
pub fn read(dir: &Path) -> Result<DebugConfig, String> {
    let path = dir.join(DEBUG_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DebugConfig::default()),
        Err(e) => return Err(format!("Can't read {DEBUG_FILE}: {e}")),
    };
    parse(&text)
}

fn parse(text: &str) -> Result<DebugConfig, String> {
    let doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("{DEBUG_FILE} isn't valid TOML: {e}"))?;
    let nets = |key: &str| -> (Vec<String>, usize) {
        let Some(arr) = doc.get(key).and_then(Item::as_array_of_tables) else {
            return (Vec::new(), 0);
        };
        let single = arr
            .iter()
            .filter_map(|t| t.get("net").and_then(Item::as_str).map(str::to_string))
            .collect();
        (single, arr.len())
    };
    let (watch, nw) = nets("watch");
    let (drive, nd) = nets("drive");
    Ok(DebugConfig {
        exists: true,
        clock: doc
            .get("clock")
            .and_then(Item::as_str)
            .unwrap_or("")
            .to_string(),
        watch,
        drive,
        has_nets: nw + nd > 0,
    })
}

/// Writes the picker's choice into the project's debug file, keeping the
/// rest of it.
pub fn save(dir: &Path, clock: &str, watch: &[String], drive: &[String]) -> Result<(), String> {
    let path = dir.join(DEBUG_FILE);
    let old = match std::fs::read_to_string(&path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("Can't read {DEBUG_FILE}: {e}")),
    };
    let text = update(old.as_deref(), clock, watch, drive)?;
    std::fs::write(&path, text).map_err(|e| format!("Can't save {DEBUG_FILE}: {e}"))
}

fn update(
    old: Option<&str>,
    clock: &str,
    watch: &[String],
    drive: &[String],
) -> Result<String, String> {
    let mut doc: DocumentMut = old
        .unwrap_or("")
        .parse()
        .map_err(|e| format!("{DEBUG_FILE} isn't valid TOML: {e}"))?;
    doc["clock"] = value(clock);
    if !doc.contains_key("uplink") {
        // The board's UART, as the catalog names it.
        let mut up = InlineTable::new();
        up.insert("kind", "uart".into());
        up.insert("resource", "uart".into());
        up.insert("baud", 115200i64.into());
        doc["uplink"] = value(up);
    }
    set_nets(&mut doc, "watch", watch);
    set_nets(&mut doc, "drive", drive);
    // A new file starts with the schema line (as a document's only line,
    // toml_edit would keep it after the values).
    let text = doc.to_string();
    Ok(if old.is_none() {
        format!("{SCHEMA_LINE}{text}")
    } else {
        text
    })
}

/// Keeps the entries whose net is still chosen (and those with a net list,
/// which the picker doesn't show), then adds an entry per new net.
fn set_nets(doc: &mut DocumentMut, key: &str, chosen: &[String]) {
    let mut kept = ArrayOfTables::new();
    let mut have: Vec<String> = Vec::new();
    if let Some(arr) = doc.get(key).and_then(Item::as_array_of_tables) {
        for t in arr.iter() {
            match t.get("net").and_then(Item::as_str) {
                Some(n) if chosen.iter().any(|c| c == n) => {
                    have.push(n.to_string());
                    kept.push(t.clone());
                }
                Some(_) => {}
                None => kept.push(t.clone()),
            }
        }
    }
    for net in chosen {
        if !have.contains(net) {
            let mut t = Table::new();
            t["net"] = value(net.as_str());
            kept.push(t);
            have.push(net.clone());
        }
    }
    if kept.is_empty() {
        doc.remove(key);
    } else {
        doc[key] = Item::ArrayOfTables(kept);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn a_new_file_has_the_schema_line_and_the_uart() {
        let text = update(None, "clk", &s(&["main.count"]), &s(&["main.led"])).unwrap();
        assert!(text.starts_with(SCHEMA_LINE), "{text}");
        assert!(text.contains("uplink = { kind = \"uart\", resource = \"uart\", baud = 115200 }"));
        let c = parse(&text).unwrap();
        assert_eq!(c.clock, "clk");
        assert_eq!(c.watch, s(&["main.count"]));
        assert_eq!(c.drive, s(&["main.led"]));
        assert!(c.has_nets);
    }

    #[test]
    fn comments_and_other_settings_stay() {
        let old = "# My debug setup\nclock = \"clk\"\nuplink = { kind = \"uart\", resource = \"uart2\" }\n\n\
                   [[watch]]\n# the counter\nname = \"counter\"\nnet = \"main.count\"\nformat = \"dec\"\n\n\
                   [[watch]]\nnet = \"main.old\"\n\n[[watch]]\nname = \"bus\"\nnet = [\"main.a\", \"main.b\"]\n";
        let text = update(Some(old), "clk", &s(&["main.count", "main.new"]), &[]).unwrap();
        assert!(text.contains("# My debug setup"));
        assert!(text.contains("# the counter"));
        assert!(text.contains("format = \"dec\""));
        assert!(
            text.contains("resource = \"uart2\""),
            "the uplink isn't replaced"
        );
        assert!(!text.contains("main.old"));
        assert!(
            text.contains("[\"main.a\", \"main.b\"]"),
            "a net list stays"
        );
        assert!(!text.contains("[[drive]]"));
        let c = parse(&text).unwrap();
        assert_eq!(c.watch, s(&["main.count", "main.new"]));
    }

    #[test]
    fn a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(dir.path()).unwrap(), DebugConfig::default());
        save(dir.path(), "clk", &s(&["main.x"]), &[]).unwrap();
        let c = read(dir.path()).unwrap();
        assert!(c.exists && c.has_nets);
    }

    #[test]
    fn bad_toml_is_an_error() {
        assert!(parse("clock = ").is_err());
    }
}
