//! Help > System Info: a plain text report for bug reports, as the Logix
//! Engine's recipe "Show system info" describes (docs/tool-api.md), so it
//! reads the same from every frontend: the IDE's own lines, a blank line,
//! then the engine's `report` from `get_engine_info`, as is.

use std::path::Path;

use serde_json::{json, Value};

use crate::engine::{self, Engine, InUse};
use crate::platform::Platform;
use crate::remote_config;

/// The IDE's own lines: name and version, the operating system, the IDE's
/// log file, which engine it runs and the remote config that named it.
pub fn ide_lines(
    name: &str,
    version: &str,
    os: &str,
    log_file: Option<&Path>,
    engine: Option<&InUse>,
) -> Vec<String> {
    let mut lines = vec![
        format!("{name} {version}"),
        format!("Operating system: {os}"),
        format!(
            "Log file: {}",
            log_file.map_or("(none)".to_string(), |p| p.display().to_string())
        ),
    ];
    match engine {
        Some(InUse::Release { release, config }) => {
            lines.push(format!(
                "Logix Engine used: release {}",
                crate::version::shown(release)
            ));
            lines.push(format!(
                "Remote config: config {}, {}{}, read {}",
                remote_config::CONFIG_VERSION,
                config.source,
                if config.is_override {
                    format!(" ({})", remote_config::DEV_REMOTE_CONFIG_VARIABLE)
                } else {
                    String::new()
                },
                remote_config::utc_time(config.read_at)
            ));
        }
        Some(InUse::Local(path)) => lines.push(format!(
            "Logix Engine used: local binary {} ({})",
            path.display(),
            engine::DEV_ENGINE_VARIABLE
        )),
        None => lines.push("Logix Engine used: not chosen yet (setup hasn't finished)".into()),
    }
    lines
}

/// The engine's part: its `report`, as `get_engine_info` gives it.
pub fn engine_part(info: &Value) -> String {
    info["report"]
        .as_str()
        .unwrap_or("Logix Engine: get_engine_info gave no report")
        .to_string()
}

/// The whole report: the IDE's lines, a blank line, the engine's part.
pub fn report(ide: &[String], engine: &str) -> String {
    format!("{}\n\n{engine}", ide.join("\n"))
}

/// The operating system and its version, such as "macOS 26.5 (macos-arm64)".
pub fn os_description(platform: Option<Platform>) -> String {
    let id = platform.map_or("unknown platform", |p| p.id());
    let version = match platform {
        Some(Platform::MacosArm64) => std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim())),
        _ => None,
    };
    match version {
        Some(v) => format!("{v} ({id})"),
        None => id.to_string(),
    }
}

/// The engine's part from the running engine, or why there's none.
pub async fn engine_part_from(engine: Option<&Engine>) -> String {
    let Some(engine) = engine else {
        return "Logix Engine: not running (setup hasn't finished).".to_string();
    };
    match engine.call("get_engine_info", json!({}), None).await {
        Ok(Ok(info)) => engine_part(&info),
        Ok(Err(e)) => format!(
            "Logix Engine: get_engine_info failed: {}",
            e["message"].as_str().unwrap_or("no message")
        ),
        Err(e) => format!("Logix Engine: get_engine_info failed: {e:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_has_the_ide_lines_then_the_engines() {
        let ide = ide_lines(
            "Logix IDE",
            "2026.11.12",
            "macOS 26.5 (macos-arm64)",
            Some(Path::new("/Users/you/.logix/ide/logs/logix.log")),
            Some(&InUse::Release {
                release: "2026-11-08".into(),
                config: engine::ConfigInfo {
                    source: "https://raw.githubusercontent.com/openlogix/logix-ide/main/remote-config/logix-ide/config-001.jsonc".into(),
                    is_override: false,
                    read_at: 1_794_441_600,
                },
            }),
        );
        let engine = engine_part(&json!({
            "engine_version": "2026.11.08",
            "report": "Logix Engine 2026.11.08\nPlatform: darwin-arm64"
        }));
        assert_eq!(
            report(&ide, &engine),
            "Logix IDE 2026.11.12
Operating system: macOS 26.5 (macos-arm64)
Log file: /Users/you/.logix/ide/logs/logix.log
Logix Engine used: release 2026.11.08
Remote config: config 001, https://raw.githubusercontent.com/openlogix/logix-ide/main/remote-config/logix-ide/config-001.jsonc, read 2026-11-12 00:00 UTC

Logix Engine 2026.11.08
Platform: darwin-arm64"
        );
    }

    #[test]
    fn names_a_local_engine() {
        let ide = ide_lines(
            "Logix IDE",
            "2026.11.12",
            "macos-arm64",
            None,
            Some(&InUse::Local("/w/logix-engine".into())),
        );
        assert_eq!(ide[2], "Log file: (none)");
        assert_eq!(
            ide[3],
            "Logix Engine used: local binary /w/logix-engine (LOGIX_DEV_ENGINE_BINARY)"
        );
    }
}
