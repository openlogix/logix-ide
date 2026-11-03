//! Progress events sent to the UI while the IDE sets up its tools at
//! startup (the Apio bundle and its packages, then the Logix Engine and its
//! packages). Shared by both adapters.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupStage {
    Checking,
    DownloadingApio,
    InstallingPackages,
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
    /// Bytes of the file being downloaded (Apio or the engine).
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
