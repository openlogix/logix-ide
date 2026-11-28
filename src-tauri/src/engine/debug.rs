//! Debug's engine calls besides the task's own (`task.rs` builds with the
//! debug file, programs, then `open_debug` and `open_debug_panel`):
//! Logix Engine's `list_nets`, for the net picker, and `close_debug`, for
//! Stop and closing the debug window (its docs/tool-api.md, Debugger).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::client::{Engine, ToolFailure};
use super::{boards_fields, BoardsFolder, Variant};

/// A net the debug file can name, as `list_nets` gives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Net {
    /// Its path from the top module, such as `main.count`.
    pub path: String,
    /// Its width in bits.
    pub width: u32,
    /// input, output or inout for a port of the top module; internal
    /// otherwise.
    pub kind: String,
    /// Whether a debug file may watch it. Logix Engine releases before
    /// 2026-11-28 don't say (None until `list_nets` fills it in).
    #[serde(default)]
    pub can_watch: Option<bool>,
    /// Whether a debug file may drive it (the same).
    #[serde(default)]
    pub can_drive: Option<bool>,
    /// Why it can't be watched or driven, when either is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_not: Option<String>,
}

impl Net {
    /// Fills in what an older engine doesn't say, as its rules were: any
    /// net can be watched, and only internal nets driven.
    fn with_defaults(mut self) -> Self {
        self.can_watch.get_or_insert(true);
        let internal = self.kind == "internal";
        self.can_drive.get_or_insert(internal);
        if !internal && self.why_not.is_none() && self.can_drive == Some(false) {
            self.why_not = Some(
                "It is a port of the top module: debug files can watch it, but drive internal nets only."
                    .into(),
            );
        }
        self
    }
}

async fn call(engine: &Engine, tool: &str, args: Value) -> Result<Value> {
    engine
        .call(tool, args, None)
        .await?
        .map_err(|e| anyhow::Error::from(ToolFailure::from_value(&e)))
}

/// The design's nets: yosys reads and elaborates it only, so it's quick.
pub async fn list_nets(
    engine: &Engine,
    board: &str,
    variant: &Variant,
    custom_boards: Option<&BoardsFolder>,
    source_folder: &std::path::Path,
    files: &[String],
) -> Result<Vec<Net>> {
    let sources = super::task::design_files(files);
    let result = call(
        engine,
        "list_nets",
        json!({
            "board": board,
            "board_variant": variant,
            "custom_boards_folder": boards_fields(custom_boards).0,
            "top_module": super::TOP_MODULE,
            "source_folder": source_folder,
            "source_files": sources,
        }),
    )
    .await?;
    let nets: Vec<Net> = serde_json::from_value(result["nets"].clone())?;
    Ok(nets.into_iter().map(Net::with_defaults).collect())
}

/// Ends a debug session: the engine releases its driven nets and its port.
pub async fn close(engine: &Engine, session: &str) -> Result<()> {
    call(engine, "close_debug", json!({ "session": session })).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_engines_get_the_old_rule() {
        let nets: Vec<Net> = serde_json::from_value(json!([
            {"path": "main.led", "width": 1, "kind": "output"},
            {"path": "main.count", "width": 8, "kind": "internal"},
        ]))
        .unwrap();
        let nets: Vec<Net> = nets.into_iter().map(Net::with_defaults).collect();
        assert_eq!(nets[0].can_watch, Some(true));
        assert_eq!(nets[0].can_drive, Some(false));
        assert!(nets[0]
            .why_not
            .as_deref()
            .unwrap()
            .contains("port of the top module"));
        assert_eq!(
            (nets[1].can_watch, nets[1].can_drive),
            (Some(true), Some(true))
        );
        assert_eq!(nets[1].why_not, None);
    }

    #[test]
    fn what_the_engine_says_is_kept() {
        let net: Net = serde_json::from_value(json!({
            "path": "main.rgb", "width": 3, "kind": "internal",
            "can_watch": false, "can_drive": false, "why_not": "Pin only."
        }))
        .unwrap();
        let net = net.with_defaults();
        assert_eq!((net.can_watch, net.can_drive), (Some(false), Some(false)));
        assert_eq!(net.why_not.as_deref(), Some("Pin only."));
    }
}
