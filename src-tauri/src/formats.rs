//! Which files are Verilog sources, includes and pin files, and each pin
//! file format's line comment. Plain Rust, no Tauri.
//!
//! These are Logix Engine's facts: `get_engine_info`'s `hdl_extensions`
//! and `get_catalog`'s `families` (`constraint_extension`,
//! `constraint_comment`), set once the engine answers (`set`). Until then
//! a built-in copy stands in (`fallback`): Logix IDE reopens the last
//! project at startup, before the engine is up (on a first start, before
//! it's even downloaded), and its tabs, pin file icons and Comment/Uncomment
//! need them right away. The copy is checked against the engine's catalog
//! (the UI tests' recorded copy) by a test, so it only covers that moment.

use std::path::Path;
use std::sync::RwLock;

use serde::Serialize;

/// The file formats Logix IDE knows, by extension (without the dot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFormats {
    /// Design and testbench files, such as `v` and `sv`.
    pub source: Vec<String>,
    /// Files they include, such as `vh` and `svh`.
    pub include: Vec<String>,
    /// Pin constraint files, one per FPGA family's format, each with its
    /// line comment (`#`, or `//` for Gowin's `.cst`).
    pub pins: Vec<PinFormat>,
}

/// A pin constraint file format: its extension and line comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PinFormat {
    pub extension: String,
    pub comment: String,
}

/// The engine's facts before it has answered (see the module's comment).
pub fn fallback() -> FileFormats {
    let list = |l: &[&str]| l.iter().map(|s| s.to_string()).collect();
    let pin = |extension: &str, comment: &str| PinFormat {
        extension: extension.into(),
        comment: comment.into(),
    };
    FileFormats {
        source: list(&["v", "sv"]),
        include: list(&["vh", "svh"]),
        pins: vec![
            pin("pcf", "#"),
            pin("lpf", "#"),
            pin("cst", "//"),
            pin("xdc", "#"),
        ],
    }
}

static CURRENT: RwLock<Option<FileFormats>> = RwLock::new(None);

/// The formats in use: the engine's once set, else the fallback.
pub fn current() -> FileFormats {
    CURRENT
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(fallback)
}

/// Uses the engine's formats from now on. Lists the engine leaves empty
/// (an engine that doesn't say) keep the fallback's.
pub fn set(mut formats: FileFormats) {
    let base = fallback();
    if formats.source.is_empty() {
        formats.source = base.source;
    }
    if formats.include.is_empty() {
        formats.include = base.include;
    }
    if formats.pins.is_empty() {
        formats.pins = base.pins;
    }
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = Some(formats);
}

fn extension(name: &str) -> Option<&str> {
    Path::new(name).extension().and_then(|e| e.to_str())
}

fn has(list: &[String], name: &str) -> bool {
    extension(name).is_some_and(|e| list.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

impl FileFormats {
    /// A design or testbench file (`.v`, `.sv`), what the tools build.
    pub fn is_source(&self, name: &str) -> bool {
        has(&self.source, name)
    }

    /// Verilog or SystemVerilog, sources and includes (what Auto Format
    /// takes).
    pub fn is_hdl(&self, name: &str) -> bool {
        has(&self.source, name) || has(&self.include, name)
    }

    /// A pin constraint file of any FPGA family's format.
    pub fn is_pin_file(&self, name: &str) -> bool {
        extension(name).is_some_and(|e| {
            self.pins
                .iter()
                .any(|p| p.extension.eq_ignore_ascii_case(e))
        })
    }

    /// Every extension, sources first, for messages (".v .sv .vh …").
    pub fn extensions(&self) -> Vec<String> {
        let pins = self.pins.iter().map(|p| p.extension.clone());
        self.source
            .iter()
            .chain(&self.include)
            .cloned()
            .chain(pins)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_files_apart() {
        let f = fallback();
        assert!(f.is_source("main.v") && f.is_source("uart.SV"));
        assert!(!f.is_source("defs.vh"));
        assert!(f.is_hdl("defs.SVH") && !f.is_hdl("pinout.pcf") && !f.is_hdl("notes"));
        assert!(f.is_pin_file("pinout.PCF") && f.is_pin_file("pins.cst"));
        assert!(!f.is_pin_file("pins.pdc") && !f.is_pin_file("main.v"));
        assert_eq!(f.extensions().join(" "), "v sv vh svh pcf lpf cst xdc");
        // An engine that leaves a list out keeps the fallback's.
        set(FileFormats {
            source: Vec::new(),
            include: Vec::new(),
            pins: Vec::new(),
        });
        assert_eq!(current(), fallback());
    }

    #[test]
    fn the_fallback_is_the_engines_formats() {
        // Every pin file format in the engine's catalog (the UI tests'
        // recorded copy, `get_boards`' pinExtension and pinComment) is the
        // fallback's, with its comment, and the fallback has no other.
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../ui/e2e/fixtures/catalog.json")).unwrap();
        let mut used: Vec<(String, String)> = fixture["boards"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|b| {
                Some((
                    b["pinExtension"].as_str()?.to_string(),
                    b["pinComment"].as_str()?.to_string(),
                ))
            })
            .collect();
        used.sort();
        used.dedup();
        let mut offered: Vec<(String, String)> = fallback()
            .pins
            .into_iter()
            .map(|p| (p.extension, p.comment))
            .collect();
        offered.sort();
        assert_eq!(used, offered);
    }
}
