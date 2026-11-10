//! Where the engine's packages keep their programs, from `get_engine_info`
//! (each downloaded package's `bin_folder`), for programs the IDE runs
//! itself, such as Verible's language server.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine};

/// The path of `program` (without `.exe`) from the downloaded `package`.
pub async fn program_path(engine: &Engine, package: &str, program: &str) -> Result<PathBuf> {
    let info = engine
        .call("get_engine_info", json!({}), None)
        .await?
        .map_err(|e| tool_error("get_engine_info", &e))?;
    find_program(&info, package, program)
}

fn find_program(info: &Value, package: &str, program: &str) -> Result<PathBuf> {
    let folder = info["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["name"] == package)
        .with_context(|| format!("Logix Engine has no {package} package"))?["bin_folder"]
        .as_str()
        .with_context(|| format!("Logix Engine's {package} package isn't downloaded"))?;
    Ok(PathBuf::from(folder).join(format!("{program}{}", std::env::consts::EXE_SUFFIX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_program_in_its_package() {
        let info = json!({"packages": [
            {"name": "oss-cad-suite", "status": "missing"},
            {"name": "verible", "status": "ready", "bin_folder": "/Users/you/.logix/engine/packages/verible/2026-10-22/bin"}
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
