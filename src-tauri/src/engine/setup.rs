//! Engine setup at startup: make sure the pinned engine release is
//! installed, start it, and let it install its packages (toolchains) if any
//! are missing. Once they are installed, startup needs no network: the
//! engine checks its packages locally.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine, Progress};
use super::{bundle, Layout, ENGINE_RELEASE_REPO, ENGINE_RELEASE_TAG};
use crate::platform;
use crate::setup_events::{SetupEvent, SetupStage};

// User-facing messages, as in Apio's setup: the outermost message is plain
// language, and the full chain (shown in the console) keeps the details.
const MSG_DOWNLOAD: &str =
    "Couldn't download Logix Engine. Check your internet connection, then click Retry.";
const MSG_UNPACK: &str = "Couldn't install Logix Engine on this computer.";
const MSG_PREPARE: &str = "Couldn't prepare the folder for Logix Engine.";
const MSG_START: &str = "Couldn't start Logix Engine.";
const MSG_PACKAGES: &str = "Couldn't install the FPGA tools for Logix Engine. Check your internet connection, then click Retry.";

/// Makes the engine ready to use and returns it running. `emit` receives
/// progress events. Runs after Apio's setup, so it doesn't emit `Checking`
/// or `Ready`.
pub async fn run_setup<F>(emit: F) -> Result<Engine>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let log = {
        let emit = emit.clone();
        move |line: String| emit(SetupEvent::Log { line })
    };
    let platform = platform::current_implemented()?;
    let layout = Layout::new(&platform.data_dir()?);

    let spec = bundle::package_spec(platform, ENGINE_RELEASE_REPO, ENGINE_RELEASE_TAG);
    let bin_dir = layout.bin_dir();
    if !bundle::is_installed(&bin_dir, platform.engine_exe_name(), &spec.url) {
        emit(SetupEvent::Stage {
            stage: SetupStage::DownloadingEngine,
        });
        log(format!("Downloading Logix Engine {ENGINE_RELEASE_TAG}..."));
        let tmp_dir = layout.tmp_dir();
        tokio::fs::create_dir_all(&tmp_dir)
            .await
            .with_context(|| format!("can't create {}", tmp_dir.display()))
            .context(MSG_PREPARE)?;
        let archive = tmp_dir.join(&spec.file_name);
        let emit_dl = emit.clone();
        crate::download::download(&spec.url, &archive, |received, total| {
            emit_dl(SetupEvent::Download { received, total })
        })
        .await
        .context(MSG_DOWNLOAD)?;
        let bin_dir = bin_dir.clone();
        tokio::task::spawn_blocking(move || {
            bundle::install(platform, &archive, &spec, &tmp_dir, &bin_dir)
        })
        .await
        .context(MSG_UNPACK)?
        .context(MSG_UNPACK)?;
        log("Logix Engine installed.".into());
    }
    let exe = bin_dir.join(platform.engine_exe_name());

    let folder = layout.engine_folder();
    tokio::fs::create_dir_all(&folder)
        .await
        .with_context(|| format!("can't create {}", folder.display()))
        .context(MSG_PREPARE)?;
    let engine = Engine::start(&exe, &folder).await.context(MSG_START)?;

    let info = engine
        .call("get_engine_info", json!({}), None)
        .await
        .context(MSG_START)?
        .map_err(|e| tool_error("get_engine_info", &e))
        .context(MSG_START)?;
    crate::log!(
        "Logix Engine {} started, log file {}",
        info["engine_version"].as_str().unwrap_or("?"),
        info["log_file"].as_str().unwrap_or("?")
    );

    let missing = missing_packages(&info);
    if !missing.is_empty() {
        emit(SetupEvent::Stage {
            stage: SetupStage::InstallingEnginePackages,
        });
        log(format!(
            "Installing FPGA tools for Logix Engine ({}). The first run can take a few minutes...",
            missing.join(", ")
        ));
        engine
            .call(
                "install_packages",
                json!({}),
                Some(progress_sink(emit.clone())),
            )
            .await
            .context(MSG_PACKAGES)?
            .map_err(|e| tool_error("install_packages", &e))
            .context(MSG_PACKAGES)?;
        log("FPGA tools for Logix Engine installed.".into());
    }
    Ok(engine)
}

/// Names of the packages `get_engine_info` reports as not installed
/// (missing or broken).
fn missing_packages(info: &Value) -> Vec<String> {
    info["packages"]
        .as_array()
        .map(|packages| {
            packages
                .iter()
                .filter(|p| p["status"] != "installed")
                .filter_map(|p| p["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Turns the engine's progress into setup events: the phase message (kept
/// until the next phase starts) with its percentage, and each new phase as
/// a console line.
fn progress_sink<F>(emit: F) -> Arc<dyn Fn(Progress) + Send + Sync>
where
    F: Fn(SetupEvent) + Send + Sync + 'static,
{
    let message = Mutex::new(String::new());
    Arc::new(move |p: Progress| {
        let mut current = message.lock().unwrap();
        if let Some(new) = p.message {
            if *current != new {
                emit(SetupEvent::Log {
                    line: format!("{new}..."),
                });
                *current = new;
            }
        }
        emit(SetupEvent::Progress {
            message: current.clone(),
            percent: p.percent,
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_missing_packages() {
        let info = json!({"packages": [
            {"name": "oss-cad-suite", "version": "2026-10-22", "status": "installed"},
            {"name": "openxc7", "version": "2026-10-22", "status": "missing"},
            {"name": "verible", "version": "2026-10-22", "status": "broken"},
        ]});
        assert_eq!(missing_packages(&info), vec!["openxc7", "verible"]);
        assert!(missing_packages(&json!({})).is_empty());
    }

    #[test]
    fn progress_becomes_setup_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let events = events.clone();
            progress_sink(move |e| events.lock().unwrap().push(e))
        };
        sink(Progress {
            message: Some("Installing oss-cad-suite".into()),
            percent: None,
        });
        sink(Progress {
            message: None,
            percent: Some(40),
        });
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                SetupEvent::Log {
                    line: "Installing oss-cad-suite...".into()
                },
                SetupEvent::Progress {
                    message: "Installing oss-cad-suite".into(),
                    percent: None
                },
                SetupEvent::Progress {
                    message: "Installing oss-cad-suite".into(),
                    percent: Some(40)
                },
            ]
        );
    }

    /// Runs the setup flow's engine part against a real engine binary, with
    /// automatic install off so it needs no network:
    /// `LOGIX_TEST_ENGINE_EXE=<logix-engine> cargo test real_engine_starts -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn real_engine_starts() {
        let exe = std::env::var("LOGIX_TEST_ENGINE_EXE").expect("set LOGIX_TEST_ENGINE_EXE");
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("engine");
        std::fs::create_dir_all(&folder).unwrap();
        let engine = Engine::start(std::path::Path::new(&exe), &folder)
            .await
            .unwrap();
        let info = engine
            .call("get_engine_info", json!({}), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info["platform"], "darwin-arm64");
        assert!(!missing_packages(&info).is_empty(), "{info}");
        // An unknown package is a tool error, not a failed call.
        let err = engine
            .call("install_packages", json!({"packages": ["nope"]}), None)
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(err["category"], "request");
    }

    /// Installs the real `verible` package (8 MB) through the setup's
    /// progress path and checks the banner gets percentages:
    /// `LOGIX_TEST_ENGINE_EXE=<logix-engine> cargo test real_engine_install_progress -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_engine_install_progress() {
        let exe = std::env::var("LOGIX_TEST_ENGINE_EXE").expect("set LOGIX_TEST_ENGINE_EXE");
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("engine");
        std::fs::create_dir_all(&folder).unwrap();
        let engine = Engine::start(std::path::Path::new(&exe), &folder)
            .await
            .unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let events = events.clone();
            progress_sink(move |e| events.lock().unwrap().push(e))
        };
        engine
            .call(
                "install_packages",
                json!({"packages": ["verible"]}),
                Some(sink),
            )
            .await
            .unwrap()
            .unwrap();
        let events = events.lock().unwrap().clone();
        for e in &events {
            println!("{e:?}");
        }
        let percents: Vec<u32> = events
            .iter()
            .filter_map(|e| match e {
                SetupEvent::Progress { percent, .. } => *percent,
                _ => None,
            })
            .collect();
        assert!(percents.len() > 5, "{events:?}");
        assert!(percents.windows(2).all(|w| w[1] >= w[0]), "{percents:?}");
    }
}
