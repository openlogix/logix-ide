//! Engine setup at startup: make sure the pinned engine release is
//! downloaded, start it, and let it download its packages (toolchains) if
//! any are missing. Once they are downloaded, startup needs no network: the
//! engine checks its packages locally.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine, ProgressSink};
use super::phase_sink;
use super::{
    bundle, dev_engine_binary, Layout, DEV_ENGINE_VARIABLE, ENGINE_RELEASE_REPO, ENGINE_RELEASE_TAG,
};
use crate::platform;
use crate::setup_events::{SetupEvent, SetupStage};

// User-facing messages: the outermost message is plain
// language, and the full chain (shown in the console) keeps the details.
const MSG_DOWNLOAD: &str =
    "Couldn't download Logix Engine. Check your internet connection, then click Retry.";
const MSG_UNPACK: &str = "Couldn't set up Logix Engine on this computer.";
const MSG_PREPARE: &str = "Couldn't prepare the folder for Logix Engine.";
const MSG_START: &str = "Couldn't start Logix Engine.";
const MSG_PACKAGES: &str =
    "Couldn't download the FPGA tools. Check your internet connection, then click Retry.";

/// Downloads the pinned engine release if needed and starts it, returning it
/// with its `get_engine_info`. It can already answer what needs no FPGA
/// tools, such as `get_catalog`; its packages may still be missing.
pub async fn start<F>(emit: F) -> Result<(Engine, Value)>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let log = {
        let emit = emit.clone();
        move |line: String| emit(SetupEvent::Log { line })
    };
    let platform = platform::current_implemented()?;
    let layout = Layout::new(&platform.data_dir()?);

    let dev = dev_engine_binary(std::env::var_os(DEV_ENGINE_VARIABLE)).context(MSG_START)?;
    let (exe, folder) = match dev {
        Some(exe) => {
            emit(SetupEvent::DevEngine {
                path: exe.display().to_string(),
            });
            log(format!(
                "Using the local Logix Engine {} ({DEV_ENGINE_VARIABLE}).",
                exe.display()
            ));
            (exe, layout.dev_engine_folder())
        }
        None => (
            download_release(platform, &layout, &emit).await?,
            layout.engine_folder(),
        ),
    };

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
        "Logix Engine {} started, log file {}, boards schema {}",
        info["engine_version"].as_str().unwrap_or("?"),
        info["log_file"].as_str().unwrap_or("?"),
        info["boards_schema"].as_str().unwrap_or("(not reported)")
    );

    Ok((engine, info))
}

/// Lets the started engine download its missing packages (the FPGA tools),
/// with progress. Asks the engine which are missing now: the board list,
/// loaded first, has the engine download the small logix-catalog package
/// itself, so the status read at startup is out of date.
pub async fn download_missing<F>(engine: &Engine, emit: F) -> Result<()>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let info = engine
        .call("get_engine_info", json!({}), None)
        .await
        .context(MSG_PACKAGES)?
        .map_err(|e| tool_error("get_engine_info", &e))
        .context(MSG_PACKAGES)?;
    let info = &info;
    let log = {
        let emit = emit.clone();
        move |line: String| emit(SetupEvent::Log { line })
    };
    let missing = missing_packages(info);
    if !missing.is_empty() {
        emit(SetupEvent::Stage {
            stage: SetupStage::DownloadingEnginePackages,
        });
        log(format!(
            "Downloading FPGA tools for Logix Engine ({}). The first run can take a few minutes...",
            missing.join(", ")
        ));
        engine
            .call(
                "download_packages",
                json!({}),
                Some(progress_sink(emit.clone())),
            )
            .await
            .context(MSG_PACKAGES)?
            .map_err(|e| tool_error("download_packages", &e))
            .context(MSG_PACKAGES)?;
        log("FPGA tools for Logix Engine downloaded.".into());
    }
    Ok(())
}

/// Downloads the pinned engine release if it isn't yet (download and
/// unpack, with progress), and returns its executable.
async fn download_release<F>(
    platform: platform::Platform,
    layout: &Layout,
    emit: &F,
) -> Result<std::path::PathBuf>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let log = |line: String| emit(SetupEvent::Log { line });
    let spec = bundle::package_spec(platform, ENGINE_RELEASE_REPO, ENGINE_RELEASE_TAG);
    let bin_dir = layout.bin_dir();
    if !bundle::is_unpacked(&bin_dir, platform.engine_exe_name(), &spec.url) {
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
            bundle::unpack(platform, &archive, &spec, &tmp_dir, &bin_dir)
        })
        .await
        .context(MSG_UNPACK)?
        .context(MSG_UNPACK)?;
        log("Logix Engine downloaded.".into());
    }
    Ok(bin_dir.join(platform.engine_exe_name()))
}

/// Names of the packages `get_engine_info` reports as not ready
/// (missing or broken).
fn missing_packages(info: &Value) -> Vec<String> {
    info["packages"]
        .as_array()
        .map(|packages| {
            packages
                .iter()
                .filter(|p| p["status"] != "ready")
                .filter_map(|p| p["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Turns the engine's progress into setup events: the phase message (kept
/// until the next phase starts) with its percentage, and each new phase as
/// a console line.
fn progress_sink<F>(emit: F) -> ProgressSink
where
    F: Fn(SetupEvent) + Send + Sync + 'static,
{
    phase_sink(move |message, percent, new_phase| {
        if new_phase {
            emit(SetupEvent::Log {
                line: format!("{message}..."),
            });
        }
        emit(SetupEvent::Progress {
            message: message.to_string(),
            percent,
        });
    })
}

#[cfg(test)]
mod tests {
    use super::super::client::Progress;
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn finds_missing_packages() {
        let info = json!({"packages": [
            {"name": "logix-catalog", "version": "2026-11-10", "status": "ready"},
            {"name": "logix-core", "version": "2026-11-10", "status": "broken"},
        ]});
        assert_eq!(missing_packages(&info), vec!["logix-core"]);
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
            message: Some("Downloading logix-core".into()),
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
                    line: "Downloading logix-core...".into()
                },
                SetupEvent::Progress {
                    message: "Downloading logix-core".into(),
                    percent: None
                },
                SetupEvent::Progress {
                    message: "Downloading logix-core".into(),
                    percent: Some(40)
                },
            ]
        );
    }

    /// Runs the setup flow's engine part against a real engine binary, with
    /// automatic download off so it needs no network:
    /// `LOGIX_DEV_ENGINE_BINARY=<logix-engine> cargo test real_engine_starts -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn real_engine_starts() {
        let exe = std::env::var("LOGIX_DEV_ENGINE_BINARY").expect("set LOGIX_DEV_ENGINE_BINARY");
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
            .call("download_packages", json!({"packages": ["nope"]}), None)
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(err["category"], "request");
        // A stopped engine (as after a crash) is noticed, so the IDE can
        // start it again.
        assert!(!engine.has_stopped());
        engine.kill_for_test();
        for _ in 0..50 {
            if engine.has_stopped() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(engine.has_stopped());
        assert!(engine
            .call("get_engine_info", json!({}), None)
            .await
            .is_err());
    }

    /// Downloads the real `logix-catalog` package (30 KB) through the setup's
    /// progress path and checks the banner gets percentages:
    /// `LOGIX_DEV_ENGINE_BINARY=<logix-engine> cargo test real_engine_download_progress -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_engine_download_progress() {
        let exe = std::env::var("LOGIX_DEV_ENGINE_BINARY").expect("set LOGIX_DEV_ENGINE_BINARY");
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
                "download_packages",
                json!({"packages": ["logix-catalog"]}),
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
        assert!(!percents.is_empty(), "{events:?}");
        assert!(percents.windows(2).all(|w| w[1] >= w[0]), "{percents:?}");
    }
}
