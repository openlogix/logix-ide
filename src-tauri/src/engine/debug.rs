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
    Ok(serde_json::from_value(result["nets"].clone())?)
}

/// Ends a debug session: the engine releases its driven nets and its port.
pub async fn close(engine: &Engine, session: &str) -> Result<()> {
    call(engine, "close_debug", json!({ "session": session })).await?;
    Ok(())
}
