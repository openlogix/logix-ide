//! Help > System Info: the report for bug reports, in the layout every Open
//! Logix product uses (the website repo's `CLAUDE.md`, System info and the
//! version tree; the Logix Engine's recipe "Show system info",
//! docs/tool-api.md): Logix IDE's own facts, the developer settings when any
//! is set, then the version tree: Logix IDE, with the engine's own tree
//! (`get_engine_info` `versions`) and the examples under it.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::{self, Engine, EngineInUse, InUse};
use crate::platform::Platform;
use crate::remote_config;

/// One entry of the version tree, as the engine gives its own
/// (`get_engine_info` `versions`): a program, a package or a third-party
/// release, with its own dependencies under it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct VersionEntry {
    pub name: String,
    pub version: String,
    /// program, package or third_party.
    pub kind: String,
    #[serde(default)]
    pub channel: Option<String>,
    /// The developer setting that replaces this entry.
    #[serde(default)]
    pub overridden_by: Option<String>,
    /// Lines under the entry in the text: only what the name and version
    /// don't tell.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Why the entry is incomplete, shown on its row in the dialog (its
    /// notes, in the text): set by Logix IDE when the engine can't be asked.
    #[serde(default)]
    pub problem: Option<String>,
    /// The rest, shown on hover; left out of the text.
    #[serde(default)]
    pub details: Vec<Detail>,
    #[serde(default)]
    pub children: Vec<VersionEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detail {
    pub label: String,
    pub value: String,
}

/// A developer setting in effect: an environment variable and its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeveloperSetting {
    pub name: String,
    pub value: String,
}

/// What the dialog shows, and `text`, what Copy gives.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    /// Logix IDE's own facts, as label and value: the operating system and
    /// the log file.
    pub facts: Vec<Detail>,
    pub developer_settings: Vec<DeveloperSetting>,
    pub versions: VersionEntry,
    pub text: String,
}

/// The engine's part: from `get_engine_info`, or why there's none.
pub enum EnginePart {
    Info(serde_json::Value),
    Missing(String),
}

fn detail(label: &str, value: impl Into<String>) -> Detail {
    Detail {
        label: label.into(),
        value: value.into(),
    }
}

/// Logix IDE's developer settings in effect (set and not empty). The
/// engine it starts inherits its environment and reports its own.
pub fn ide_developer_settings() -> Vec<DeveloperSetting> {
    [
        engine::DEV_ENGINE_VARIABLE,
        remote_config::CHANNEL_VARIABLE,
        remote_config::DEV_REMOTE_CONFIG_VARIABLE,
    ]
    .iter()
    .filter_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|v| !v.is_empty())
            .map(|value| DeveloperSetting {
                name: name.to_string(),
                value,
            })
    })
    .collect()
}

/// The whole report: Logix IDE's entry, with the engine's tree and the
/// examples under it.
pub fn build(
    version: &str,
    commit: &str,
    os: &str,
    log_file: Option<&Path>,
    in_use: Option<&InUse>,
    engine: EnginePart,
    mut developer_settings: Vec<DeveloperSetting>,
) -> SystemInfo {
    let facts = vec![
        detail("Operating system", os),
        detail(
            "Log file",
            log_file.map_or("(none)".to_string(), |p| p.display().to_string()),
        ),
    ];
    let mut ide = VersionEntry {
        name: "Logix IDE".into(),
        version: version.into(),
        kind: "program".into(),
        channel: None,
        overridden_by: None,
        notes: Vec::new(),
        problem: None,
        details: vec![
            detail("Repo", remote_config::CONFIG_REPO),
            if remote_config::IS_RELEASE_BUILD {
                detail("Release", version.replace('.', "-"))
            } else {
                detail("Build", "local build")
            },
        ],
        children: Vec::new(),
    };
    if !commit.is_empty() {
        ide.details.push(detail("Commit", commit));
    }
    if let EnginePart::Info(info) = &engine {
        if let Ok(list) =
            serde_json::from_value::<Vec<DeveloperSetting>>(info["developer_settings"].clone())
        {
            developer_settings.extend(list);
        }
    }
    match in_use {
        None => ide
            .notes
            .push("Logix Engine: not chosen yet (setup hasn't finished)".into()),
        Some(u) => {
            let c = &u.config.channel;
            ide.channel = Some(c.name.clone());
            let mut summary = vec![format!("channel {}", c.name)];
            if !commit.is_empty() {
                summary.push(format!("commit {commit}"));
            }
            summary.push(format!("remote config {}", remote_config::CONFIG_VERSION));
            ide.notes.push(summary.join(", "));
            ide.details.push(detail(
                "Channel",
                format!("{} ({}), reads {}", c.name, c.how, c.file),
            ));
            ide.details.push(detail(
                "Remote config",
                format!(
                    "{}, from {}",
                    remote_config::CONFIG_VERSION,
                    u.config.source
                ),
            ));
            ide.details
                .push(detail("Read", remote_config::utc_time(u.config.read_at)));
            let named_by = format!(
                "Logix IDE's remote config {}, {}",
                remote_config::CONFIG_VERSION,
                c.file
            );
            ide.children.push(engine_entry(u, &engine, &named_by));
            ide.children.push(VersionEntry {
                name: remote_config::EXAMPLES.into(),
                version: crate::version::shown(&u.examples),
                kind: "package".into(),
                channel: None,
                overridden_by: None,
                notes: Vec::new(),
                problem: None,
                details: vec![
                    detail("Repo", remote_config::CONFIG_REPO),
                    detail("Release", &u.examples),
                    detail("Pinned in", &named_by),
                ],
                children: Vec::new(),
            });
        }
    }
    let text = text(&facts, &developer_settings, &ide);
    SystemInfo {
        facts,
        developer_settings,
        versions: ide,
        text,
    }
}

/// The engine's entry: its own tree when it runs, marked when it's a local
/// binary; otherwise the release in use, and why there's no more.
fn engine_entry(u: &InUse, engine: &EnginePart, named_by: &str) -> VersionEntry {
    let mut entry = match engine {
        EnginePart::Info(info) => serde_json::from_value::<VersionEntry>(info["versions"].clone())
            .unwrap_or_else(|e| VersionEntry {
                name: "Logix Engine".into(),
                version: info["engine_version"].as_str().unwrap_or("unknown").into(),
                kind: "program".into(),
                channel: None,
                overridden_by: None,
                notes: vec![format!("get_engine_info gave no version tree: {e}")],
                problem: Some("no version tree (an engine older than 2026.11.15)".into()),
                details: Vec::new(),
                children: Vec::new(),
            }),
        EnginePart::Missing(why) => VersionEntry {
            name: "Logix Engine".into(),
            version: match &u.engine {
                EngineInUse::Release(r) => crate::version::shown(r),
                EngineInUse::Local(_) => "local build".into(),
            },
            kind: "program".into(),
            channel: None,
            overridden_by: None,
            notes: vec![why.clone()],
            problem: Some(why.clone()),
            details: Vec::new(),
            children: Vec::new(),
        },
    };
    match &u.engine {
        EngineInUse::Release(r) => {
            entry.details.push(detail("Release", r));
            entry.details.push(detail("Pinned in", named_by));
        }
        EngineInUse::Local(path) => {
            entry.overridden_by = Some(engine::DEV_ENGINE_VARIABLE.into());
            entry
                .notes
                .insert(0, format!("dev override: {}", engine::DEV_ENGINE_VARIABLE));
            entry
                .details
                .insert(0, detail("Binary", path.display().to_string()));
        }
    }
    entry
}

/// The report as text, what Copy gives: Logix IDE's facts, the developer
/// settings when any is set, then the version tree.
pub fn text(facts: &[Detail], settings: &[DeveloperSetting], versions: &VersionEntry) -> String {
    let mut lines = vec![format!("{} {}", versions.name, versions.version)];
    lines.extend(facts.iter().map(|f| format!("{}: {}", f.label, f.value)));
    if !settings.is_empty() {
        lines.push(String::new());
        lines.push("Developer settings:".into());
        lines.extend(
            settings
                .iter()
                .map(|s| format!("  {} = {}", s.name, s.value)),
        );
    }
    lines.push(String::new());
    lines.push("Versions:".into());
    lines.extend(tree_lines(versions));
    lines.join("\n")
}

/// The tree as text lines, drawn as the engine draws its own (its
/// `versions.rs`): one entry a line, its notes indented under it, children
/// with tree lines.
pub fn tree_lines(entry: &VersionEntry) -> Vec<String> {
    let mut lines = vec![format!("{} {}", entry.name, entry.version)];
    let bar = if entry.children.is_empty() {
        "     "
    } else {
        "│    "
    };
    lines.extend(entry.notes.iter().map(|n| format!("{bar}{n}")));
    let count = entry.children.len();
    for (i, child) in entry.children.iter().enumerate() {
        let last = i + 1 == count;
        for (j, line) in tree_lines(child).into_iter().enumerate() {
            let prefix = match (j == 0, last) {
                (true, false) => "├─ ",
                (true, true) => "└─ ",
                (false, false) => "│  ",
                (false, true) => "   ",
            };
            lines.push(format!("{prefix}{line}"));
        }
    }
    lines
}

/// The operating system and its version, such as "macOS 26.5 (darwin-arm64)".
pub fn os_description(platform: Option<Platform>) -> String {
    let id = platform.map_or("unknown platform", |p| p.engine_platform_id());
    let version = match platform {
        Some(Platform::MacosArm64) => std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim())),
        // "Microsoft Windows [Version 10.0.26100.2314]" -> "Windows 10.0.26100.2314".
        Some(Platform::WindowsX86_64) => {
            let mut cmd = std::process::Command::new("cmd.exe");
            cmd.args(["/c", "ver"]);
            crate::platform::hide_console(&mut cmd);
            cmd.output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| {
                    let text = String::from_utf8_lossy(&o.stdout).to_string();
                    let v = text
                        .split("Version ")
                        .nth(1)?
                        .trim()
                        .trim_end_matches(']')
                        .to_string();
                    Some(format!("Windows {v}"))
                })
        }
        _ => None,
    };
    match version {
        Some(v) => format!("{v} ({id})"),
        None => id.to_string(),
    }
}

/// The engine's part from the running engine, or why there's none.
pub async fn engine_part_from(engine: Option<&Engine>) -> EnginePart {
    let Some(engine) = engine else {
        return EnginePart::Missing("not running (setup hasn't finished)".into());
    };
    match engine.call("get_engine_info", json!({}), None).await {
        Ok(Ok(info)) => EnginePart::Info(info),
        Ok(Err(e)) => EnginePart::Missing(format!(
            "get_engine_info failed: {}",
            e["message"].as_str().unwrap_or("no message")
        )),
        Err(e) => EnginePart::Missing(format!("get_engine_info failed: {e:#}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_use(engine: EngineInUse, how: &str, file: &str) -> InUse {
        InUse {
            engine,
            examples: "2026-11-15".into(),
            config: engine::ConfigInfo {
                source: "https://raw.githubusercontent.com/openlogix/logix-ide/main/remote-config/logix-ide/001/".into(),
                is_override: false,
                channel: remote_config::Channel {
                    name: "prod".into(),
                    how: how.into(),
                    file: file.into(),
                },
                read_at: 1_794_441_600,
            },
        }
    }

    fn engine_info() -> serde_json::Value {
        json!({
            "engine_version": "2026.11.14",
            "versions": {
                "name": "Logix Engine", "version": "2026.11.14", "kind": "program", "channel": "prod",
                "notes": ["channel prod, commit a1b2c3d, remote config 001"],
                "details": [{"label": "Repo", "value": "openlogix/logix-engine"}],
                "children": [
                    {"name": "logix-core", "version": "2026.11.14", "kind": "package", "children": [
                        {"name": "OSS CAD Suite", "version": "2026-11-04", "kind": "third_party",
                         "notes": ["from YosysHQ/oss-cad-suite-build"]}
                    ]},
                    {"name": "logix-catalog", "version": "2026.11.14", "kind": "package"}
                ]
            }
        })
    }

    #[test]
    fn the_report_has_the_facts_then_the_version_tree() {
        let u = in_use(
            EngineInUse::Release("2026-11-14".into()),
            "listed",
            "prod.jsonc",
        );
        let info = build(
            "2026.11.15",
            "973dd78",
            "macOS 26.5 (darwin-arm64)",
            Some(Path::new(
                "/Users/you/.logix/ide/logs/20261115-171205-4412/ide.log",
            )),
            Some(&u),
            EnginePart::Info(engine_info()),
            Vec::new(),
        );
        assert_eq!(
            info.text,
            "Logix IDE 2026.11.15
Operating system: macOS 26.5 (darwin-arm64)
Log file: /Users/you/.logix/ide/logs/20261115-171205-4412/ide.log

Versions:
Logix IDE 2026.11.15
│    channel prod, commit 973dd78, remote config 001
├─ Logix Engine 2026.11.14
│  │    channel prod, commit a1b2c3d, remote config 001
│  ├─ logix-core 2026.11.14
│  │  └─ OSS CAD Suite 2026-11-04
│  │          from YosysHQ/oss-cad-suite-build
│  └─ logix-catalog 2026.11.14
└─ logix-ide-examples 2026.11.15"
        );
        // Which config named the engine, on hover only.
        let engine = &info.versions.children[0];
        assert!(engine
            .details
            .iter()
            .any(|d| d.label == "Pinned in"
                && d.value == "Logix IDE's remote config 001, prod.jsonc"));
    }

    #[test]
    fn developer_settings_come_from_both_and_mark_a_local_engine() {
        let u = in_use(
            EngineInUse::Local("/w/logix-engine".into()),
            "LOGIX_IDE_CHANNEL",
            "prod.jsonc",
        );
        let mut engine = engine_info();
        engine["developer_settings"] = json!([{"name": "LOGIX_ENGINE_CHANNEL", "value": "dev"}]);
        let info = build(
            "2026.11.15",
            "",
            "macOS 26.5 (darwin-arm64)",
            None,
            Some(&u),
            EnginePart::Info(engine),
            vec![DeveloperSetting {
                name: "LOGIX_DEV_ENGINE_BINARY".into(),
                value: "/w/logix-engine".into(),
            }],
        );
        let text: Vec<&str> = info.text.lines().collect();
        assert_eq!(
            text[2..10],
            [
                "Log file: (none)",
                "",
                "Developer settings:",
                "  LOGIX_DEV_ENGINE_BINARY = /w/logix-engine",
                "  LOGIX_ENGINE_CHANNEL = dev",
                "",
                "Versions:",
                "Logix IDE 2026.11.15",
            ]
        );
        assert_eq!(text[10], "│    channel prod, remote config 001");
        assert_eq!(text[11], "├─ Logix Engine 2026.11.14");
        assert_eq!(text[12], "│  │    dev override: LOGIX_DEV_ENGINE_BINARY");
        assert_eq!(
            info.versions.children[0].overridden_by.as_deref(),
            Some("LOGIX_DEV_ENGINE_BINARY")
        );
    }

    #[test]
    fn says_why_when_the_engine_isnt_running() {
        let u = in_use(
            EngineInUse::Release("2026-11-14".into()),
            "listed",
            "prod.jsonc",
        );
        let info = build(
            "2026.11.15",
            "",
            "darwin-arm64",
            None,
            Some(&u),
            EnginePart::Missing("not running (setup hasn't finished)".into()),
            Vec::new(),
        );
        assert!(
            info.text.contains(
                "├─ Logix Engine 2026.11.14\n│       not running (setup hasn't finished)"
            ),
            "{}",
            info.text
        );
        let none = build(
            "2026.11.15",
            "",
            "darwin-arm64",
            None,
            None,
            EnginePart::Missing("x".into()),
            Vec::new(),
        );
        assert!(
            none.text.ends_with(
                "Logix IDE 2026.11.15\n     Logix Engine: not chosen yet (setup hasn't finished)"
            ),
            "{}",
            none.text
        );
    }
}
