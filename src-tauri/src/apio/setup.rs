//! First-run (and every-run) Apio setup: make sure the pinned Apio bundle is
//! installed, then let Apio install or update its own packages (offline is
//! fine once they are installed).

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

use super::{bundle, validate_apio_home, Apio, Layout};
use super::{APIO_RELEASE_REPO, APIO_RELEASE_TAG, DEV_APIO_BIN_ENV};
use crate::platform;

// User-facing messages. Errors are wrapped with these at each step, so the
// outermost message is plain language and the full chain (shown in the
// console) keeps the technical details.
const MSG_DOWNLOAD: &str =
    "Couldn't download Apio. Check your internet connection, then click Retry.";
const MSG_UNPACK: &str = "Couldn't install Apio on this computer.";
const MSG_PREPARE: &str = "Couldn't prepare the folder for the FPGA tools.";
const MSG_START: &str = "Couldn't start Apio.";
const MSG_PACKAGES: &str =
    "Couldn't install the FPGA tools. Check your internet connection, then click Retry.";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupStage {
    Checking,
    DownloadingApio,
    InstallingPackages,
    Ready,
}

/// Progress reported to the UI while setting up.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SetupEvent {
    Stage { stage: SetupStage },
    Download { received: u64, total: Option<u64> },
    Log { line: String },
}

/// Makes Apio ready to use. `emit` receives progress events. Doesn't emit
/// `Ready`; the caller does once it has stored the result.
pub async fn run_setup<F>(emit: F) -> Result<Apio>
where
    F: Fn(SetupEvent) + Send + Sync + Clone + 'static,
{
    let log = {
        let emit = emit.clone();
        move |line: String| emit(SetupEvent::Log { line })
    };

    emit(SetupEvent::Stage {
        stage: SetupStage::Checking,
    });
    let platform = platform::current_implemented()?;
    let layout = Layout::new(&platform.data_dir()?);
    let home = layout.home_dir();
    validate_apio_home(&home).context(MSG_PREPARE)?;

    let exe = match std::env::var_os(DEV_APIO_BIN_ENV) {
        Some(dev_exe) => {
            let exe = std::path::PathBuf::from(dev_exe);
            log(format!(
                "Using developer Apio from {DEV_APIO_BIN_ENV}: {}",
                exe.display()
            ));
            exe
        }
        None => {
            let spec = bundle::bundle_spec(platform, APIO_RELEASE_REPO, APIO_RELEASE_TAG);
            let bundle_dir = layout.bundle_dir();
            if !bundle::is_installed(&bundle_dir, platform.apio_exe_name(), &spec.url) {
                emit(SetupEvent::Stage {
                    stage: SetupStage::DownloadingApio,
                });
                log(format!("Downloading Apio {APIO_RELEASE_TAG}..."));
                let tmp_dir = layout.tmp_dir();
                tokio::fs::create_dir_all(&tmp_dir)
                    .await
                    .with_context(|| format!("can't create {}", tmp_dir.display()))
                    .context(MSG_PREPARE)?;
                let archive = tmp_dir.join(&spec.file_name);
                let emit_dl = emit.clone();
                bundle::download(&spec.url, &archive, |received, total| {
                    emit_dl(SetupEvent::Download { received, total })
                })
                .await
                .context(MSG_DOWNLOAD)?;
                log("Unpacking Apio...".into());
                let bundle_dir_for_task = bundle_dir.clone();
                tokio::task::spawn_blocking(move || {
                    bundle::install(platform, &archive, &spec, &tmp_dir, &bundle_dir_for_task)
                })
                .await
                .context(MSG_UNPACK)?
                .context(MSG_UNPACK)?;
                log("Apio installed.".into());
            }
            bundle_dir.join(platform.apio_exe_name())
        }
    };

    tokio::fs::create_dir_all(&home)
        .await
        .with_context(|| format!("can't create {}", home.display()))
        .context(MSG_PREPARE)?;
    let apio = Apio {
        exe,
        home,
        tmp: layout.tmp_dir(),
    };

    emit(SetupEvent::Stage {
        stage: SetupStage::InstallingPackages,
    });
    log("Checking FPGA tools (the first run can take a few minutes)...".into());
    // Not `apio packages install`: that always needs a fresh online config
    // and fails offline. `apio api get-system` also installs missing
    // packages, but falls back to Apio's cached config when offline, so the
    // IDE starts without internet once the tools are installed.
    let tmp_dir = layout.tmp_dir();
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .with_context(|| format!("can't create {}", tmp_dir.display()))
        .context(MSG_PREPARE)?;
    let get_system = super::args([
        std::ffi::OsString::from("api"),
        "get-system".into(),
        "-f".into(),
        "-o".into(),
        tmp_dir.join("system.json").into_os_string(),
    ]);
    // An info command, so the command line isn't shown, but its output is:
    // on the first run, it's the progress of installing the FPGA tools.
    let status = apio
        .run_output_only(&get_system, None, log.clone())
        .await
        .context(MSG_START)?;
    if !status.success() {
        return Err(anyhow!("Apio exited with {status}").context(MSG_PACKAGES));
    }
    Ok(apio)
}
