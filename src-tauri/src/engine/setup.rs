//! Engine setup at startup: read Logix IDE's remote config (cached for 24
//! hours, and used offline), make sure the engine release it names is
//! downloaded, start it, and let it download its packages (toolchains) if
//! any are missing. Once they are downloaded, startup needs no network
//! until the config is due again, and offline the cached one is used.
//! Before an operation, `update_if_due` reads the config again when it's
//! due, so a new engine release is used from the next operation.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine, ProgressSink};
use super::phase_sink;
use super::{
    bundle, dev_engine_binary, set_in_use, ConfigInfo, InUse, Layout, DEV_ENGINE_VARIABLE,
};
use crate::platform::{self, Platform};
use crate::remote_config::{self, Cached, ConfigSource, Dependency};
use crate::setup_events::{SetupEvent, SetupStage};
use crate::version::shown;

// User-facing messages: the outermost message is plain
// language, and the full chain (shown in the console) keeps the details.
const MSG_CONFIG: &str =
    "Couldn't check which Logix Engine to use. Check your internet connection, then click Retry.";
const MSG_DOWNLOAD: &str =
    "Couldn't download Logix Engine. Check your internet connection, then click Retry.";
const MSG_UNPACK: &str = "Couldn't set up Logix Engine on this computer.";
const MSG_PREPARE: &str = "Couldn't prepare the folder for Logix Engine.";
const MSG_START: &str = "Couldn't start Logix Engine.";
const MSG_PACKAGES: &str =
    "Couldn't download the FPGA tools. Check your internet connection, then click Retry.";

/// After a failed check before an operation (such as offline), the next
/// one waits this long, so operations don't each wait for the network.
const RETRY_AFTER: Duration = Duration::from_secs(60 * 60);
static LAST_FAILED_CHECK: Mutex<Option<Instant>> = Mutex::new(None);

/// Reads the remote config: the cached one while it's fresh (unless
/// `force`), else from its source. Offline (a regular config that can't be
/// read), the cached one is used if there is one. An override is always
/// read from its source, and failing to is an error.
async fn read_config(
    layout: &Layout,
    source: &ConfigSource,
    platform: Platform,
    force: bool,
) -> Result<Cached> {
    let cached = remote_config::load_cache(&layout.config_dir).filter(|c| c.matches(source));
    if let Some(c) = &cached {
        if !force && !c.is_due() {
            return Ok(c.clone());
        }
    }
    match remote_config::fetch(source, platform.engine_platform_id()).await {
        Ok(fresh) => {
            if let Err(e) = remote_config::save_cache(&layout.config_dir, &fresh) {
                crate::log!("Couldn't save the remote config: {e:#}");
            }
            crate::log!(
                "Remote config {} read: Logix Engine {}",
                source.location,
                fresh.engine().release
            );
            Ok(fresh)
        }
        Err(e) => match cached {
            Some(c) if !source.is_override => {
                crate::log!(
                    "Couldn't read the remote config, using the one read {}: {e:#}",
                    remote_config::utc_time(c.fetched)
                );
                Ok(c)
            }
            _ => Err(e),
        },
    }
}

fn record(dep: &Dependency, cached: &Cached, source: &ConfigSource) {
    set_in_use(InUse::Release {
        release: dep.release.clone(),
        config: ConfigInfo {
            source: source.location.clone(),
            is_override: source.is_override,
            read_at: cached.fetched,
        },
    });
}

/// Downloads the engine release the remote config names if needed and
/// starts it, returning it with its `get_engine_info`. It can already answer
/// what needs no FPGA tools, such as `get_catalog`; its packages may still
/// be missing.
pub async fn start<F>(emit: F) -> Result<(Engine, Value)>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let log = {
        let emit = emit.clone();
        move |line: String| emit(SetupEvent::Log { line })
    };
    let platform = platform::current_implemented()?;
    let source = ConfigSource::from_env().context(MSG_CONFIG)?;
    let layout = Layout::new(&platform.data_dir()?, &source);

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
            set_in_use(InUse::Local(exe.clone()));
            (exe, layout.dev_engine_folder())
        }
        None => {
            if source.is_override {
                emit(SetupEvent::DevRemoteConfig {
                    location: source.location.clone(),
                });
                log(format!(
                    "Using the remote config {} ({}), with its own folder {}.",
                    source.location,
                    remote_config::DEV_REMOTE_CONFIG_VARIABLE,
                    layout.config_dir.display()
                ));
            }
            let config = read_config(&layout, &source, platform, source.is_override)
                .await
                .context(MSG_CONFIG)?;
            let dep = config.engine().clone();
            let progress = {
                let emit = emit.clone();
                move |p| match p {
                    DownloadProgress::Started => emit(SetupEvent::Stage {
                        stage: SetupStage::DownloadingEngine,
                    }),
                    DownloadProgress::Bytes { received, total } => {
                        emit(SetupEvent::Download { received, total })
                    }
                }
            };
            let exe = download_release(platform, &layout, &dep, &log, &progress).await?;
            record(&dep, &config, &source);
            (exe, layout.engine_folder())
        }
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

/// Before an operation: reads the remote config again when it's due (or
/// always, with `force`: Check for Updates) and downloads the engine release
/// it names if that's a new one. Returns true when it did, so the caller
/// starts the new engine; the operation then runs on it. A local engine
/// (`LOGIX_DEV_ENGINE_BINARY`) is never replaced. Without `force`, a config
/// that can't be read (such as offline) isn't an error: the current engine
/// stays, and the next check waits an hour.
pub async fn update_if_due(log: impl Fn(String) + Send + Sync, force: bool) -> Result<bool> {
    if dev_engine_binary(std::env::var_os(DEV_ENGINE_VARIABLE))?.is_some() {
        return Ok(false);
    }
    let platform = platform::current_implemented()?;
    let source = ConfigSource::from_env()?;
    let layout = Layout::new(&platform.data_dir()?, &source);
    if !force {
        let fresh = remote_config::load_cache(&layout.config_dir)
            .filter(|c| c.matches(&source))
            .is_some_and(|c| !c.is_due());
        let waiting = LAST_FAILED_CHECK
            .lock()
            .unwrap()
            .is_some_and(|t| t.elapsed() < RETRY_AFTER);
        if fresh || waiting {
            return Ok(false);
        }
    }
    let config = match read_config(&layout, &source, platform, true).await {
        Ok(c) => c,
        Err(e) if !force => {
            crate::log!("Couldn't check for a new Logix Engine: {e:#}");
            *LAST_FAILED_CHECK.lock().unwrap() = Some(Instant::now());
            return Ok(false);
        }
        Err(e) => return Err(e.context(MSG_CONFIG)),
    };
    *LAST_FAILED_CHECK.lock().unwrap() = None;
    let dep = config.engine().clone();
    let new = !bundle::is_unpacked(&layout.bin_dir(), platform, &dep);
    if new {
        download_release(platform, &layout, &dep, &log, &no_progress).await?;
    }
    record(&dep, &config, &source);
    Ok(new)
}

/// Help > Check for Updates, after `update_if_due(.., true)`: asks the
/// running engine to check its own packages now (`check_updates`), with its
/// progress, and returns one line per package whose release changed, in
/// the words the user knows.
pub async fn check_for_updates(
    engine: &Engine,
    log: impl Fn(String) + Send + Sync + 'static,
) -> Result<Vec<String>> {
    let log = std::sync::Arc::new(log);
    let sink = {
        let log = log.clone();
        phase_sink(move |message, _percent, new_phase| {
            if new_phase {
                log(format!("{message}..."));
            }
        })
    };
    let out = engine
        .call("check_updates", json!({}), Some(sink))
        .await
        .context(MSG_PACKAGES)?
        .map_err(|e| tool_error("check_updates", &e))
        .context(MSG_PACKAGES)?;
    Ok(update_lines(&out))
}

/// `check_updates`' `updates`, as lines such as "Updated the boards and
/// FPGAs to 2026.11.24." (the engine gives versions as yyyy.mm.dd).
fn update_lines(out: &Value) -> Vec<String> {
    out["updates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|u| {
            let to = u["to"].as_str()?;
            let what = match u["name"].as_str()? {
                "logix-catalog" => "the boards and FPGAs".to_string(),
                "logix-core" => "the FPGA tools".to_string(),
                other => other.to_string(),
            };
            Some(format!("Updated {what} to {to}."))
        })
        .collect()
}

/// Lets the started engine download its missing packages (the FPGA tools),
/// with progress. Always asks the engine (`download_packages`): it reads its
/// own remote config first when that's due, and downloads only what's
/// missing, so when everything is there it returns at once.
pub async fn download_missing<F>(engine: &Engine, emit: F) -> Result<()>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let out = engine
        .call(
            "download_packages",
            json!({}),
            Some(progress_sink(emit.clone())),
        )
        .await
        .context(MSG_PACKAGES)?
        .map_err(|e| tool_error("download_packages", &e))
        .context(MSG_PACKAGES)?;
    let downloaded = downloaded_packages(&out);
    if !downloaded.is_empty() {
        emit(SetupEvent::Log {
            line: format!(
                "FPGA tools for Logix Engine downloaded ({}).",
                downloaded.join(", ")
            ),
        });
    }
    Ok(())
}

/// Downloads the engine release `dep` if it isn't yet (download, check
/// and unpack, with progress in the console), replacing the release there
/// before, and returns its executable.
async fn download_release(
    platform: Platform,
    layout: &Layout,
    dep: &Dependency,
    log: &(dyn Fn(String) + Send + Sync),
    progress: &(dyn Fn(DownloadProgress) + Send + Sync),
) -> Result<PathBuf> {
    let bin_dir = layout.bin_dir();
    if !bundle::is_unpacked(&bin_dir, platform, dep) {
        let previous = bundle::receipt(&bin_dir).map(|r| r.release);
        progress(DownloadProgress::Started);
        log(format!(
            "Downloading Logix Engine {}...",
            shown(&dep.release)
        ));
        let tmp_dir = layout.tmp_dir();
        tokio::fs::create_dir_all(&tmp_dir)
            .await
            .with_context(|| format!("can't create {}", tmp_dir.display()))
            .context(MSG_PREPARE)?;
        let archive = tmp_dir.join(&dep.file);
        crate::download::download(&dep.url, &archive, |received, total| {
            progress(DownloadProgress::Bytes { received, total })
        })
        .await
        .context(MSG_DOWNLOAD)?;
        let (dep2, bin2) = (dep.clone(), bin_dir.clone());
        tokio::task::spawn_blocking(move || {
            bundle::unpack(platform, &archive, &dep2, &tmp_dir, &bin2)
        })
        .await
        .context(MSG_UNPACK)?
        .context(MSG_UNPACK)?;
        log(match previous {
            Some(p) if p != dep.release => {
                format!("Updated Logix Engine to {}.", shown(&dep.release))
            }
            _ => format!("Logix Engine {} downloaded.", shown(&dep.release)),
        });
    }
    Ok(bin_dir.join(platform.engine_exe_name()))
}

/// How far the engine package's download is, for setup's progress.
enum DownloadProgress {
    Started,
    Bytes { received: u64, total: Option<u64> },
}

/// No progress to show (a download before an operation, with console
/// lines only).
fn no_progress(_: DownloadProgress) {}

/// The names `download_packages` lists as downloaded.
fn downloaded_packages(out: &Value) -> Vec<String> {
    out["downloaded"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p.as_str().map(String::from))
        .collect()
}

/// Turns the engine's progress into setup events: the downloading stage
/// when the first phase starts (nothing when there's nothing to download),
/// the phase message (kept until the next phase starts) with its
/// percentage, and each new phase as a console line.
fn progress_sink<F>(emit: F) -> ProgressSink
where
    F: Fn(SetupEvent) + Send + Sync + 'static,
{
    let started = std::sync::atomic::AtomicBool::new(false);
    phase_sink(move |message, percent, new_phase| {
        if new_phase {
            if !started.swap(true, std::sync::atomic::Ordering::Relaxed) {
                emit(SetupEvent::Stage {
                    stage: SetupStage::DownloadingEnginePackages,
                });
                emit(SetupEvent::Log {
                    line: "Downloading the FPGA tools. The first time can take a few minutes."
                        .into(),
                });
            }
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
    fn lists_downloaded_packages_and_updates() {
        let out = json!({"downloaded": ["logix-core"], "packages": []});
        assert_eq!(downloaded_packages(&out), vec!["logix-core"]);
        assert!(downloaded_packages(&json!({})).is_empty());
        let out = json!({"updates": [
            {"name": "logix-catalog", "from": "2026.11.24", "to": "2026.11.30"},
            {"name": "logix-core", "to": "2026.12.01"},
        ]});
        assert_eq!(
            update_lines(&out),
            vec![
                "Updated the boards and FPGAs to 2026.11.30.",
                "Updated the FPGA tools to 2026.12.01."
            ]
        );
        assert!(update_lines(&json!({"updates": []})).is_empty());
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
                SetupEvent::Stage {
                    stage: SetupStage::DownloadingEnginePackages
                },
                SetupEvent::Log {
                    line: "Downloading the FPGA tools. The first time can take a few minutes."
                        .into()
                },
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

    /// The whole setup on the network, in a temporary data folder, with the
    /// repo's own remote config file as the override (so it tests what the
    /// repo names, before it's on main): downloads the engine release it
    /// names from GitHub, checks its SHA-256 and receipt, and starts it, all
    /// in the override's own folder; a forced check right after finds
    /// nothing new:
    /// `cargo test real_setup_from_the_remote_config -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_setup_from_the_remote_config() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var(crate::platform::HOME_VARIABLE, tmp.path());
        std::env::remove_var(DEV_ENGINE_VARIABLE);
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(remote_config::repo_path());
        std::env::set_var(remote_config::DEV_REMOTE_CONFIG_VARIABLE, &file);
        let events = Arc::new(Mutex::new(Vec::new()));
        let emit = {
            let events = events.clone();
            move |e: SetupEvent| events.lock().unwrap().push(e)
        };
        let (_engine, info) = start(emit).await.unwrap();
        for e in events.lock().unwrap().iter() {
            println!("{e:?}");
        }
        assert_eq!(info["platform"], "darwin-arm64");
        let dir = tmp.path().join("dev/config-override");
        let cached = remote_config::load_cache(&dir).expect("the cached config");
        let receipt = bundle::receipt(&dir.join("engine-release")).unwrap();
        assert!(dir.join("engine").is_dir());
        assert!(
            !tmp.path().join("engine").exists(),
            "the regular folders stay untouched"
        );
        assert!(receipt.matches(cached.engine(), Platform::MacosArm64));
        assert!(matches!(
            super::super::in_use(),
            Some(InUse::Release { .. })
        ));
        assert!(!update_if_due(|l| println!("{l}"), true).await.unwrap());
    }
}
