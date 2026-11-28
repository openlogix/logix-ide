//! The project's debug file, `debug.toml`: the nets Debug watches and drives.
//!
//! A project file held in memory like the sources (`project::is_debug_file`),
//! so these work on its text. Logix Engine reads it in a debug build
//! (`build`'s `debug_file`, from the build folder's copy) and
//! owns its format (its `docs/debugger.md`, schema `logix-debug-v1.json`).
//! The IDE reads it here to know which nets are chosen, and its net picker
//! rewrites only the parts it owns (`clock`, a missing `uplink`, the
//! `[[watch]]` and `[[drive]]` entries), with `toml_edit`, so the user's
//! comments and other settings (triggers, counters, formats, names) stay.
//! The Debugger window's layout (its rows dragged into a new order, its
//! read rate and chart span) is written the same way (`set_layout`): it
//! reorders the entries, as they are, and sets `[panel]`.

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

/// Reads the project's debug file; none is an empty config.
pub fn read(text: Option<&str>) -> Result<DebugConfig, String> {
    text.map_or_else(|| Ok(DebugConfig::default()), parse)
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

/// The project's debug file with the picker's choice written in, keeping
/// the rest of it; a new file when there's none.
pub fn update(
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

/// The project's debug file with the Debugger window's layout written in:
/// the `[[watch]]` and `[[drive]]` entries in the panel's order (by their
/// `name`, else their net; entries it didn't list keep their place after
/// the listed ones), each kept as it is, and `[panel]`'s `refresh_ms` and
/// `window_s` (the read rate and the chart span). None when nothing
/// changes, so an unchanged layout isn't an edit.
pub fn set_layout(
    old: &str,
    watch: &[String],
    drive: &[String],
    refresh_ms: Option<u32>,
    window_s: Option<u32>,
) -> Result<Option<String>, String> {
    let mut doc: DocumentMut = old
        .parse()
        .map_err(|e| format!("{DEBUG_FILE} isn't valid TOML: {e}"))?;
    reorder(&mut doc, "watch", watch);
    reorder(&mut doc, "drive", drive);
    for (key, v) in [("refresh_ms", refresh_ms), ("window_s", window_s)] {
        let Some(v) = v else { continue };
        if !doc.contains_table("panel") {
            doc["panel"] = Item::Table(Table::new());
        }
        let panel = &mut doc["panel"];
        if panel.get(key).and_then(Item::as_integer) != Some(i64::from(v)) {
            panel[key] = value(i64::from(v));
        }
    }
    let text = doc.to_string();
    Ok((text != old).then_some(text))
}

/// The name the panel shows an entry by: its `name`, else its net.
fn entry_name(t: &Table) -> Option<&str> {
    t.get("name")
        .and_then(Item::as_str)
        .or_else(|| t.get("net").and_then(Item::as_str))
}

/// Puts the entries of `key` in the order of `names`.
fn reorder(doc: &mut DocumentMut, key: &str, names: &[String]) {
    let Some(arr) = doc.get(key).and_then(Item::as_array_of_tables) else {
        return;
    };
    let tables: Vec<Table> = arr.iter().cloned().collect();
    let rank = |t: &Table| {
        entry_name(t)
            .and_then(|n| names.iter().position(|x| x == n))
            .unwrap_or(names.len())
    };
    let mut order: Vec<usize> = (0..tables.len()).collect();
    // Stable: unlisted entries keep their order, after the listed ones.
    order.sort_by_key(|&i| rank(&tables[i]));
    if order.iter().enumerate().all(|(i, &j)| i == j) {
        return;
    }
    // toml_edit writes tables by their place in the document, so the
    // entries trade places: each takes the place of the one it replaces.
    let places: Vec<Option<isize>> = tables.iter().map(Table::position).collect();
    let mut out = ArrayOfTables::new();
    for (place, i) in places.into_iter().zip(order) {
        let mut t = tables[i].clone();
        t.set_position(place);
        out.push(t);
    }
    doc[key] = Item::ArrayOfTables(out);
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
        assert_eq!(read(None).unwrap(), DebugConfig::default());
        let text = update(None, "clk", &s(&["main.x"]), &[]).unwrap();
        let c = read(Some(&text)).unwrap();
        assert!(c.exists && c.has_nets);
    }

    #[test]
    fn bad_toml_is_an_error() {
        assert!(parse("clock = ").is_err());
    }

    const LAID_OUT: &str = r#"#:schema https://www.openlogix.org/schemas/logix-debug-v1.json
clock = "main.clk"

# The counter.
[[watch]]
net = "main.count"
format = "hex"

[[watch]]
name = "LED"
net = "main.led"

[[watch]]
net = "main.btn"

[[drive]]
net = "main.a"

[[drive]]
net = "main.b"
"#;

    #[test]
    fn the_layout_reorders_the_entries_as_they_are() {
        let text = set_layout(
            LAID_OUT,
            &s(&["LED", "main.count"]),
            &s(&["main.b", "main.a"]),
            None,
            None,
        )
        .unwrap()
        .unwrap();
        let cfg = parse(&text).unwrap();
        assert_eq!(cfg.watch, s(&["main.led", "main.count", "main.btn"]));
        assert_eq!(cfg.drive, s(&["main.b", "main.a"]));
        assert!(text.contains("# The counter.\n[[watch]]\nnet = \"main.count\"\nformat = \"hex\""));
        assert!(text.contains("name = \"LED\""));
        assert!(!text.contains("[panel]"));
    }

    #[test]
    fn the_layout_sets_the_rate_and_span() {
        let text = set_layout(LAID_OUT, &[], &[], Some(500), Some(30))
            .unwrap()
            .unwrap();
        assert!(
            text.contains("[panel]\nrefresh_ms = 500\nwindow_s = 30\n"),
            "{text}"
        );
        // The order didn't change.
        assert_eq!(
            parse(&text).unwrap().watch,
            s(&["main.count", "main.led", "main.btn"])
        );
        // The same again changes nothing.
        assert_eq!(
            set_layout(&text, &[], &[], Some(500), Some(30)).unwrap(),
            None
        );
        let text = set_layout(&text, &[], &[], Some(250), None)
            .unwrap()
            .unwrap();
        assert!(text.contains("refresh_ms = 250\nwindow_s = 30"), "{text}");
    }

    #[test]
    fn an_unchanged_layout_is_no_edit() {
        let names = s(&["main.count", "LED", "main.btn"]);
        assert_eq!(
            set_layout(LAID_OUT, &names, &s(&["main.a"]), None, None).unwrap(),
            None
        );
    }
}
