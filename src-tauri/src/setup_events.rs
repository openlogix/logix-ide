//! Progress events sent to the UI while the IDE sets up its tools at
//! startup: downloading Logix Engine, then installing its packages.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupStage {
    /// Checking what's installed.
    Checking,
    DownloadingEngine,
    InstallingEnginePackages,
    Ready,
}

/// Progress reported to the UI while setting up.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SetupEvent {
    Stage {
        stage: SetupStage,
    },
    /// Bytes of the engine package being downloaded.
    Download {
        received: u64,
        total: Option<u64>,
    },
    /// The engine's install progress: the current phase's message (such as
    /// "Installing oss-cad-suite") and its percentage, when known.
    Progress {
        message: String,
        percent: Option<u32>,
    },
    Log {
        line: String,
    },
}
