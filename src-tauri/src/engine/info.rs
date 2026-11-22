//! What `get_engine_info` tells the IDE beyond versions: how to run
//! Verible's language server (`language_server`: its command, arguments
//! and environment, so the IDE runs it exactly as the engine says), and
//! which files are Verilog sources and includes (`hdl_extensions`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use super::client::{tool_error, Engine};

/// A program the IDE runs itself, as the engine gives it: the program's
/// absolute path, its arguments in order, and environment variables to set
/// on top of the IDE's own.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerCommand {
    pub command: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

async fn engine_info(engine: &Engine) -> Result<Value> {
    engine
        .call("get_engine_info", json!({}), None)
        .await?
        .map_err(|e| tool_error("get_engine_info", &e))
}

/// Verible's language server, as the engine runs it: `get_engine_info`'s
/// `language_server` (there once the FPGA tools are downloaded).
pub async fn language_server(engine: &Engine) -> Result<ServerCommand> {
    parse_language_server(&engine_info(engine).await?)
}

fn parse_language_server(info: &Value) -> Result<ServerCommand> {
    let server = info
        .get("language_server")
        .filter(|s| s.is_object())
        .context("Logix Engine's language server isn't downloaded yet")?;
    serde_json::from_value(server.clone()).context("Logix Engine's language_server isn't readable")
}

/// The extensions of Verilog sources and includes (`hdl_extensions`):
/// (source, include), without the dot; empty lists when the engine
/// doesn't say.
pub async fn hdl_extensions(engine: &Engine) -> Result<(Vec<String>, Vec<String>)> {
    let info = engine_info(engine).await?;
    let list = |key: &str| -> Vec<String> {
        info["hdl_extensions"][key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| e.as_str().map(String::from))
            .collect()
    };
    Ok((list("source"), list("include")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_language_server() {
        let info = json!({"language_server": {
            "command": "/Users/you/.logix/ide/engine-data/packages/logix-core/2026-11-10/verible/bin/verible-verilog-ls",
            "args": ["--ruleset=none", "--lsp_enable_hover=true"],
            "env": {}
        }});
        let server = parse_language_server(&info).unwrap();
        assert!(server.command.ends_with("verible-verilog-ls"));
        assert_eq!(server.args, ["--ruleset=none", "--lsp_enable_hover=true"]);
        assert!(server.env.is_empty());
        // Before logix-core is downloaded, the engine has none.
        assert!(parse_language_server(&json!({"packages": []})).is_err());
    }
}
