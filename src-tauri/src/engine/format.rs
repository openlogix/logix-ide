//! Auto Format with the engine's `format` tool (Verible), on a file in a
//! scratch folder (`build_dir::prepare_single`), since the editor's text
//! may not be saved.

use std::path::Path;

use anyhow::Result;
use serde_json::json;

use super::client::{tool_error, CallOptions, Engine};
use super::task::{diagnostics, Console};
use crate::outcome::{Diagnostic, TaskEvent};

/// The result of formatting one file.
#[derive(Debug, Clone)]
pub struct FormatOutcome {
    /// The file is formatted now (rewritten or already formatted).
    pub ok: bool,
    /// Why it couldn't be formatted, such as a syntax error.
    pub diagnostics: Vec<Diagnostic>,
    /// The files the engine checked (`checked_files`): the file.
    pub checked_files: Vec<String>,
}

/// Formats `folder/file` in place, with Verible's lines in the console
/// (`on_event`, with their locations for links).
pub async fn format_file<F>(
    engine: &Engine,
    folder: &Path,
    file: &str,
    on_event: F,
) -> Result<FormatOutcome>
where
    F: Fn(TaskEvent) + Send + Sync + Clone + 'static,
{
    let console = Console::new(on_event);
    let options = CallOptions {
        on_output: Some(console.sink()),
        ..Default::default()
    };
    let result = engine
        .call_with(
            "format",
            json!({
                "source_folder": folder.to_string_lossy(),
                "source_files": [file],
            }),
            options,
        )
        .await?;
    let done = result.map_err(|e| tool_error("format", &e))?;
    Ok(FormatOutcome {
        ok: done["passed"] == true,
        diagnostics: diagnostics(&done["diagnostics"]),
        checked_files: done["checked_files"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f.as_str().map(String::from))
            .collect(),
    })
}
