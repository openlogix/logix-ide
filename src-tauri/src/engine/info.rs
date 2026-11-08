//! What the IDE takes from `get_engine_info`: where the engine's packages
//! keep their programs (each installed package's `bin_folder`), for
//! programs the IDE runs itself, such as Verible's language server, and the
//! boards schema the engine reads (`boards_schema`), shown in About.

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine};

/// The path of `program` (without `.exe`) from the installed `package`.
pub async fn program_path(engine: &Engine, package: &str, program: &str) -> Result<PathBuf> {
    let info = engine
        .call("get_engine_info", json!({}), None)
        .await?
        .map_err(|e| tool_error("get_engine_info", &e))?;
    find_program(&info, package, program)
}

static BOARDS_SCHEMA: OnceLock<String> = OnceLock::new();

/// Keeps the engine's boards schema address from its `get_engine_info`, the
/// JSON Schema `custom-boards.json` may name in `"$schema"`. An engine
/// before 2026-11-08 doesn't report it.
pub fn remember_boards_schema(info: &Value) {
    if let Some(url) = boards_schema_of(info) {
        let _ = BOARDS_SCHEMA.set(url);
    }
}

/// The boards schema the running engine reads, once setup has started it.
pub fn boards_schema() -> Option<&'static str> {
    BOARDS_SCHEMA.get().map(String::as_str)
}

fn boards_schema_of(info: &Value) -> Option<String> {
    info["boards_schema"].as_str().map(String::from)
}

fn find_program(info: &Value, package: &str, program: &str) -> Result<PathBuf> {
    let folder = info["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["name"] == package)
        .with_context(|| format!("Logix Engine has no {package} package"))?["bin_folder"]
        .as_str()
        .with_context(|| format!("Logix Engine's {package} package isn't installed"))?;
    Ok(PathBuf::from(folder).join(format!("{program}{}", std::env::consts::EXE_SUFFIX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_boards_schema() {
        let url = "https://www.openlogix.org/schemas/logix-boards-v1.json";
        assert_eq!(
            boards_schema_of(&json!({"boards_schema": url})).as_deref(),
            Some(url)
        );
        // An older engine: none.
        assert_eq!(
            boards_schema_of(&json!({"engine_version": "2026-11-08"})),
            None
        );
    }

    #[test]
    fn finds_a_program_in_its_package() {
        let info = json!({"packages": [
            {"name": "oss-cad-suite", "status": "missing"},
            {"name": "verible", "status": "installed", "bin_folder": "/Users/you/.logix/engine/packages/verible/2026-10-22/bin"}
        ]});
        assert_eq!(
            find_program(&info, "verible", "verible-verilog-ls").unwrap(),
            PathBuf::from(format!(
                "/Users/you/.logix/engine/packages/verible/2026-10-22/bin/verible-verilog-ls{}",
                std::env::consts::EXE_SUFFIX
            ))
        );
        assert!(find_program(&info, "oss-cad-suite", "yosys").is_err());
        assert!(find_program(&info, "nope", "x").is_err());
    }
}
