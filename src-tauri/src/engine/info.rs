//! Where the engine's tools keep their programs, from `get_engine_info`
//! (the downloaded `logix-core` package's `components`, each with its
//! `bin_folder`), for programs the IDE runs itself, such as Verible's
//! language server.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine};

/// The path of `program` (without `.exe`) from the core's `component`,
/// such as `verible`.
pub async fn program_path(engine: &Engine, component: &str, program: &str) -> Result<PathBuf> {
    let info = engine
        .call("get_engine_info", json!({}), None)
        .await?
        .map_err(|e| tool_error("get_engine_info", &e))?;
    find_program(&info, component, program)
}

fn find_program(info: &Value, component: &str, program: &str) -> Result<PathBuf> {
    let core = info["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["name"] == "logix-core")
        .context("Logix Engine has no logix-core package")?;
    let folder = core["components"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["name"] == component)
        .and_then(|c| c["bin_folder"].as_str())
        .with_context(|| format!("Logix Engine's {component} isn't downloaded"))?;
    Ok(PathBuf::from(folder).join(format!("{program}{}", std::env::consts::EXE_SUFFIX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_program_in_the_core() {
        let info = json!({"packages": [
            {"name": "logix-catalog", "status": "ready"},
            {"name": "logix-core", "status": "ready", "components": [
                {"name": "verible", "description": "Verible", "bin_folder": "/Users/you/.logix/ide/engine/packages/logix-core/2026-11-10/verible/bin"}
            ]}
        ]});
        assert_eq!(
            find_program(&info, "verible", "verible-verilog-ls").unwrap(),
            PathBuf::from(format!(
                "/Users/you/.logix/ide/engine/packages/logix-core/2026-11-10/verible/bin/verible-verilog-ls{}",
                std::env::consts::EXE_SUFFIX
            ))
        );
        assert!(find_program(&info, "oss-cad-suite", "yosys").is_err());
        let missing = json!({"packages": [{"name": "logix-core", "status": "missing"}]});
        assert!(find_program(&missing, "verible", "verible-verilog-ls").is_err());
        assert!(find_program(&info, "nope", "x").is_err());
    }
}
