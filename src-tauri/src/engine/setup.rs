//! Engine setup at startup: read Logix IDE's remote config (cached for 24
//! hours, and used offline), make sure the engine release it names is
//! downloaded, start it, and let it download its packages (toolchains) if
//! any are missing. Once they are downloaded, startup needs no network
//! until the config is due again, and offline the cached one is used.
//! Before an operation, `update_if_due` reads the config again when it's
//! due, so a new engine release is used from the next operation.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::client::{tool_error, Engine, ProgressSink};
use super::phase_sink;
use super::{
    dev_engine_binary, set_in_use, ConfigInfo, EngineInUse, InUse, Layout, DEV_ENGINE_VARIABLE,
};
use crate::package;
use crate::platform::{self, Platform};
use crate::remote_config::{self, Cached, ConfigSource, Dependency};
use crate::setup_events::{SetupEvent, SetupStage};
use crate::version::shown;

// User-facing messages: the outermost message is plain
// language, and the full chain (shown in the console) keeps the details.
const MSG_CONFIG: &str =
    "Couldn't check which Logix Engine to use. Check your internet connection, then click Retry.";
const MSG_DOWNLOAD: &str =
    "Couldn't download Logix Engine or the examples. Check your internet connection, then click Retry.";
const MSG_UNPACK: &str = "Couldn't set up Logix Engine on this computer.";
const MSG_EXAMPLES: &str = "Couldn't download the examples.";
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

/// The dependencies in place, as `read_config` and the downloads left them.
fn record(engine: EngineInUse, cached: &Cached, source: &ConfigSource) {
    set_in_use(InUse {
        engine,
        examples: cached.examples().release.clone(),
        config: ConfigInfo {
            source: source.location.clone(),
            is_override: source.is_override,
            channel: cached.channel.clone(),
            read_at: cached.fetched,
        },
    });
}

/// Downloads the examples package the config names if it isn't in place
/// yet, and uses it. Returns the line saying it was updated, if it was.
async fn ensure_examples(
    platform: Platform,
    layout: &Layout,
    config: &Cached,
    log: &(dyn Fn(String) + Send + Sync),
) -> Result<Option<String>> {
    let dir = layout.examples_dir();
    let changed = download_dependency(
        platform,
        layout,
        config.examples(),
        &DependencyKind::EXAMPLES,
        &dir,
        log,
        &no_progress,
    )
    .await
    .context(MSG_EXAMPLES)?;
    crate::examples::set_root(&dir);
    Ok(changed)
}

/// Reads the remote config (cached while fresh), downloads the examples and
/// the engine release it names if needed, and starts the engine, returning
/// it with its `get_engine_info`. It can already answer what needs no FPGA
/// tools, such as `get_catalog`; its packages may still be missing. With a
/// local engine (`LOGIX_DEV_ENGINE_BINARY`) the config still gives the
/// examples.
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

    // Logix IDE's own developer settings, shown from the start; the
    // engine's are added once it runs.
    let mut developer_settings: Vec<String> = crate::system_info::ide_developer_settings()
        .into_iter()
        .map(|s| s.name)
        .collect();
    if !developer_settings.is_empty() {
        emit(SetupEvent::DeveloperSettings {
            names: developer_settings.clone(),
        });
    }
    if let Some(channel) = remote_config::channel_variable().context(MSG_CONFIG)? {
        log(format!(
            "Using the release channel {channel} ({}).",
            remote_config::CHANNEL_VARIABLE
        ));
    }
    if source.is_override {
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
    if let Some(line) = remote_config::channel_line(&config.channel) {
        log(line);
    }
    ensure_examples(platform, &layout, &config, &log).await?;

    let (exe, folder, in_use) = match dev {
        Some(exe) => {
            log(format!(
                "Using the local Logix Engine {} ({DEV_ENGINE_VARIABLE}).",
                exe.display()
            ));
            (
                exe.clone(),
                layout.dev_engine_folder(),
                EngineInUse::Local(exe),
            )
        }
        None => {
            let dep = config.engine().clone();
            let progress = {
                let emit = emit.clone();
                let message = format!("Downloading Logix Engine {}", shown(&dep.release));
                move |p| match p {
                    DownloadProgress::Started => emit(SetupEvent::Stage {
                        stage: SetupStage::DownloadingEngine,
                    }),
                    // One percentage for the download and the unpacking
                    // (60% and 40%, the org's rule), as the engine reports
                    // its own packages.
                    DownloadProgress::Percent(percent) => emit(SetupEvent::Progress {
                        message: message.clone(),
                        percent,
                    }),
                }
            };
            let bin_dir = layout.bin_dir(&dep.release);
            download_dependency(
                platform,
                &layout,
                &dep,
                &DependencyKind::engine(platform),
                &bin_dir,
                &log,
                &progress,
            )
            .await?;
            // Earlier releases: none runs now, before this one starts.
            remove_other_releases(&layout, &dep.release);
            (
                bin_dir.join(platform.engine_exe_name()),
                layout.engine_folder(),
                EngineInUse::Release(dep.release.clone()),
            )
        }
    };
    record(in_use, &config, &source);

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
    // The engine's own developer settings, inherited from Logix IDE's
    // environment or not, join the banner.
    let engine_settings: Vec<String> = info["developer_settings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["name"].as_str().map(str::to_string))
        .collect();
    if !engine_settings.is_empty() {
        developer_settings.extend(engine_settings);
        emit(SetupEvent::DeveloperSettings {
            names: developer_settings,
        });
    }

    Ok((engine, info))
}

/// Deletes everything in the engine releases folder but `release`'s
/// folder: earlier releases (the org rule: switching deletes the old
/// release). A folder Windows still locks, as the engine that runs from it
/// does, stays; the next start deletes it.
fn remove_other_releases(layout: &Layout, release: &str) {
    let Ok(entries) = std::fs::read_dir(layout.releases_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name() == release {
            continue;
        }
        let path = entry.path();
        let gone = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(e) = gone {
            crate::log!("Can't delete {} yet: {e}", path.display());
        }
    }
}

/// What `update_if_due` changed.
#[derive(Debug, Default)]
pub struct Updated {
    /// A new engine release is in place: the caller starts it.
    pub engine: bool,
    /// What changed, a line each, such as "Updated logix-ide-examples to 2026.11.13."
    pub lines: Vec<String>,
}

/// Before an operation: reads the remote config again when it's due (or
/// always, with `force`: Check for Updates) and downloads the examples and
/// the engine release it names if they're new ones. The caller then starts
/// the new engine (`Updated::engine`); the operation runs on it. A local
/// engine (`LOGIX_DEV_ENGINE_BINARY`) is never replaced. Without `force`, a
/// config that can't be read (such as offline) isn't an error: what's in
/// place stays, and the next check waits an hour.
pub async fn update_if_due(
    log: impl Fn(String) + Send + Sync,
    force: bool,
    progress: &(dyn Fn(String, Option<u32>) + Send + Sync),
) -> Result<Updated> {
    let dev = dev_engine_binary(std::env::var_os(DEV_ENGINE_VARIABLE))?;
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
            return Ok(Updated::default());
        }
    }
    let config = match read_config(&layout, &source, platform, true).await {
        Ok(c) => c,
        Err(e) if !force => {
            crate::log!("Couldn't check for updates: {e:#}");
            *LAST_FAILED_CHECK.lock().unwrap() = Some(Instant::now());
            return Ok(Updated::default());
        }
        Err(e) => return Err(e.context(MSG_CONFIG)),
    };
    *LAST_FAILED_CHECK.lock().unwrap() = None;
    let mut updated = Updated::default();
    updated
        .lines
        .extend(ensure_examples(platform, &layout, &config, &log).await?);
    let in_use = match dev {
        Some(exe) => EngineInUse::Local(exe),
        None => {
            let dep = config.engine().clone();
            // Its progress, for Check for Updates' status bar (one
            // percentage for the download and the unpacking).
            let message = format!(
                "Downloading {} {}",
                DependencyKind::engine(platform).name,
                crate::version::shown(&dep.release)
            );
            let line = download_dependency(
                platform,
                &layout,
                &dep,
                &DependencyKind::engine(platform),
                &layout.bin_dir(&dep.release),
                &log,
                &|p| match p {
                    DownloadProgress::Started => progress(message.clone(), None),
                    DownloadProgress::Percent(percent) => progress(message.clone(), percent),
                },
            )
            .await?;
            // The engine still running is of an earlier release: on Windows
            // its folder is locked and stays until the next start.
            remove_other_releases(&layout, &dep.release);
            updated.engine = line.is_some();
            updated.lines.extend(line);
            EngineInUse::Release(dep.release)
        }
    };
    record(in_use, &config, &source);
    Ok(updated)
}

/// Help > Check for Updates' second part (the website repo's `CLAUDE.md`,
/// Remote updates, A newer version of the product): whether a newer Logix
/// IDE exists on this build's channel. Logix IDE never installs itself: it
/// only says so, with the release's page. Returns the lines to show (the
/// page, then the status line), or none when this one is the newest. A
/// failure (offline, a broken file) is logged and shown in `log`, and isn't
/// an error: the dependencies' part has already run.
pub async fn newer_ide_lines(log: impl Fn(String)) -> Vec<String> {
    match newer_ide_release().await {
        Ok(latest) => newer_lines(latest.as_ref(), crate::version::APP_VERSION),
        Err(e) => {
            crate::log!("Couldn't check for a newer Logix IDE: {e:#}");
            log(format!("Couldn't check for a newer Logix IDE: {e:#}"));
            Vec::new()
        }
    }
}

/// The newest Logix IDE release on this build's channel (the channel of
/// the cached config, else `LOGIX_IDE_CHANNEL` or `dev`), as the remote
/// config's `latest` names it; for Check for Updates and the newer-version
/// banner (`newer.rs`).
pub async fn newer_ide_release() -> Result<Option<remote_config::Entry>> {
    let source = ConfigSource::from_env()?;
    let layout = Layout::new(&platform::current_implemented()?.data_dir()?, &source);
    let channel = match remote_config::load_cache(&layout.config_dir) {
        Some(cached) if cached.matches(&source) => cached.channel.name,
        _ => remote_config::channel_variable()?.unwrap_or_else(|| "dev".into()),
    };
    remote_config::newer_release(&source, &channel).await
}

/// The lines for a newer Logix IDE: its page, then "Logix IDE 2026.12.01 is
/// available (you have 2026.11.17)."; none when `latest` isn't newer than
/// `own` (this build's version, yyyy.mm.dd).
fn newer_lines(latest: Option<&remote_config::Entry>, own: &str) -> Vec<String> {
    let Some(latest) = latest.filter(|l| l.release > own.replace('.', "-")) else {
        return Vec::new();
    };
    vec![
        format!(
            "Download it from {}",
            crate::newer::release_page(&latest.repo, &latest.release)
        ),
        format!(
            "Logix IDE {} is available (you have {own}).",
            shown(&latest.release)
        ),
    ]
}

/// Help > Check for Updates, after `update_if_due(.., true)`: asks the
/// running engine to check its own packages now (`check_updates`), with its
/// progress, and returns what changed, in the engine's words (its
/// `summary`: "Updated logix-catalog to 2026.11.30."); none when nothing
/// did, so Logix IDE's own "Everything is up to date." covers its engine
/// and examples too.
pub async fn check_for_updates(
    engine: &Engine,
    log: impl Fn(String) + Send + Sync + 'static,
    progress: impl Fn(String, Option<u32>) + Send + Sync + 'static,
) -> Result<Vec<String>> {
    let log = std::sync::Arc::new(log);
    let sink = {
        let log = log.clone();
        // Each package's phase as a console line, and its percentage for
        // the status bar, as setup shows them (the user's report,
        // 2026-11-22: only a spinner showed while logix-core downloaded).
        phase_sink(move |message, percent, new_phase| {
            if new_phase {
                log(format!("{message}..."));
            }
            progress(message.to_string(), percent);
        })
    };
    let out = engine
        .call("check_updates", json!({}), Some(sink))
        .await
        .context(MSG_PACKAGES)?
        .map_err(|e| tool_error("check_updates", &e))
        .context(MSG_PACKAGES)?;
    Ok(update_summary(&out))
}

/// `check_updates`' `summary` when a package changed (`updates` or
/// `downloaded`); none otherwise, when the summary only says the packages
/// are up to date.
fn update_summary(out: &Value) -> Vec<String> {
    let changed = |key: &str| out[key].as_array().is_some_and(|l| !l.is_empty());
    if changed("updates") || changed("downloaded") {
        strings(&out["summary"])
    } else {
        Vec::new()
    }
}

/// A list of texts, such as the engine's `summary`.
fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(String::from))
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
    // What it downloaded, in the engine's words ("Downloaded logix-core
    // 2026.11.10."); nothing when everything was there.
    for line in strings(&out["summary"]) {
        emit(SetupEvent::Log { line });
    }
    Ok(())
}

/// A kind of dependency package: how it's named for people and laid out.
struct DependencyKind {
    /// As in "Downloading Logix Engine 2026.11.12...", "Updated
    /// logix-ide-examples to ...": a program by its product name, a data
    /// package by its package name (the org's rule, 2026-11-15).
    name: &'static str,
    /// The package's one top-level folder.
    top: &'static str,
    /// A path that must be in it.
    needed: &'static str,
    /// The engine's executable needs preparing (made executable, no
    /// quarantine).
    executable: bool,
}

impl DependencyKind {
    const EXAMPLES: DependencyKind = DependencyKind {
        name: "logix-ide-examples",
        top: "logix-ide-examples",
        needed: "README.md",
        executable: false,
    };

    fn engine(platform: Platform) -> DependencyKind {
        DependencyKind {
            name: "Logix Engine",
            top: "logix-engine",
            needed: platform.engine_exe_name(),
            executable: true,
        }
    }
}

/// Downloads the dependency `dep` into `dest` if its receipt there doesn't
/// match (download, check and unpack, with progress and console lines),
/// replacing the release there before. Returns a line saying what it
/// downloaded ("Updated Logix Engine to 2026.11.13." when it replaced
/// another release), or None when it was in place.
async fn download_dependency(
    platform: Platform,
    layout: &Layout,
    dep: &Dependency,
    kind: &DependencyKind,
    dest: &std::path::Path,
    log: &(dyn Fn(String) + Send + Sync),
    progress: &(dyn Fn(DownloadProgress) + Send + Sync),
) -> Result<Option<String>> {
    if package::is_unpacked(dest, platform, dep, kind.needed) {
        return Ok(None);
    }
    let previous = package::receipt(dest).map(|r| r.release);
    progress(DownloadProgress::Started);
    log(format!(
        "Downloading {} {}...",
        kind.name,
        shown(&dep.release)
    ));
    let tmp_dir = layout.tmp_dir();
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .with_context(|| format!("can't create {}", tmp_dir.display()))
        .context(MSG_PREPARE)?;
    let archive = tmp_dir.join(&dep.file);
    crate::download::download(&dep.url, &archive, |received, total| {
        progress(DownloadProgress::Percent(
            crate::download::download_percent(received, total),
        ))
    })
    .await
    .context(MSG_DOWNLOAD)?;
    // Unpacking: the rest of the percentage, by how much of the archive has
    // been read, reported every 200 ms while it runs (as the engine does).
    let size = tokio::fs::metadata(&archive)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    let read = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (dep2, dest2, read2) = (dep.clone(), dest.to_path_buf(), read.clone());
    let (top, needed, executable) = (kind.top, kind.needed, kind.executable);
    let mut unpacking = tokio::task::spawn_blocking(move || {
        package::unpack(
            platform,
            &archive,
            &dep2,
            top,
            needed,
            &tmp_dir,
            &dest2,
            &read2,
            |dir| {
                if executable {
                    platform.prepare_unpacked_bundle(dir, &dir.join(needed))
                } else {
                    Ok(())
                }
            },
        )
    });
    let unpacked = loop {
        tokio::select! {
            r = &mut unpacking => break r,
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                let done = read.load(std::sync::atomic::Ordering::Relaxed);
                progress(DownloadProgress::Percent(Some(
                    crate::download::unpack_percent(done, size),
                )));
            }
        }
    };
    unpacked.context(MSG_UNPACK)?.context(MSG_UNPACK)?;
    progress(DownloadProgress::Percent(Some(100)));
    let line = match previous {
        Some(p) if p != dep.release => format!("Updated {} to {}.", kind.name, shown(&dep.release)),
        _ => format!("Downloaded {} {}.", kind.name, shown(&dep.release)),
    };
    log(line.clone());
    Ok(Some(line))
}

/// How far the engine package's download is, for setup's progress: its
/// percentage covers the download (0 to 60) and the unpacking (60 to 100),
/// None while the download's size is unknown.
enum DownloadProgress {
    Started,
    Percent(Option<u32>),
}

/// No progress to show (a download before an operation, with console
/// lines only).
fn no_progress(_: DownloadProgress) {}

/// Turns the engine's progress into setup events: the downloading stage
/// when the first phase starts (nothing when there's nothing to download;
/// the status bar says it can take a few minutes, the console doesn't),
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
    fn shows_the_engines_summary_when_something_changed() {
        let out = json!({"updates": [
            {"name": "logix-catalog", "from": "2026.11.24", "to": "2026.11.30"},
        ], "downloaded": ["logix-catalog"], "summary": ["Updated logix-catalog to 2026.11.30."]});
        assert_eq!(
            update_summary(&out),
            ["Updated logix-catalog to 2026.11.30."]
        );
        let out = json!({"updates": [], "downloaded": ["logix-core"], "summary": ["Downloaded logix-core 2026.12.01."]});
        assert_eq!(update_summary(&out), ["Downloaded logix-core 2026.12.01."]);
        // Nothing changed: Logix IDE says it, for everything.
        let out = json!({"updates": [], "downloaded": [], "summary": ["Logix Engine's packages are up to date."]});
        assert!(update_summary(&out).is_empty());
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
    /// progress path and checks the setup's progress gets percentages:
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
        let receipt =
            package::receipt(&dir.join("engine-program").join(&cached.engine().release)).unwrap();
        let examples = package::receipt(&dir.join("examples")).unwrap();
        assert!(examples.matches(cached.examples(), Platform::MacosArm64));
        assert!(dir.join("examples/icesugar-1-5/blinky").is_dir());
        assert!(dir.join("engine-data").is_dir());
        assert!(
            !tmp.path().join("engine-data").exists(),
            "the regular folders stay untouched"
        );
        assert!(receipt.matches(cached.engine(), Platform::MacosArm64));
        assert!(matches!(
            super::super::in_use().map(|u| u.engine),
            Some(EngineInUse::Release(_))
        ));
        let again = update_if_due(|l| println!("{l}"), true, &|_, _| {})
            .await
            .unwrap();
        assert!(!again.engine && again.lines.is_empty(), "{again:?}");
    }

    #[test]
    fn a_newer_logix_ide_is_reported_with_its_page() {
        let latest = remote_config::Entry {
            repo: "openlogix/logix-ide".into(),
            release: "2026-12-01".into(),
            file: String::new(),
        };
        assert_eq!(
            newer_lines(Some(&latest), "2026.11.17"),
            [
                "Download it from https://github.com/openlogix/logix-ide/releases/tag/2026-12-01",
                "Logix IDE 2026.12.01 is available (you have 2026.11.17).",
            ]
        );
        assert!(newer_lines(Some(&latest), "2026.12.01").is_empty());
        assert!(newer_lines(Some(&latest), "2026.12.05").is_empty());
        assert!(newer_lines(None, "2026.11.17").is_empty());
    }
}
