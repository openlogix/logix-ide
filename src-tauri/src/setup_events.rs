//! Progress events sent to the UI while the IDE sets up its tools at
//! startup: downloading Logix Engine, then its packages.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupStage {
    /// Checking what's downloaded.
    Checking,
    DownloadingEngine,
    DownloadingEnginePackages,
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
    /// The engine is running and the board and example lists are ready
    /// (`get_boards`, `get_examples`), before the FPGA tools are downloaded.
    CatalogReady,
    /// A download's progress: the current phase's message (such as
    /// "Downloading Logix Engine", or the engine's "Downloading logix-core")
    /// and its percentage, when known: 0 to 60 downloading, 60 to 100
    /// unpacking (the org's rule).
    Progress {
        message: String,
        percent: Option<u32>,
    },
    Log {
        line: String,
    },
    /// Developer settings are in effect (such as `LOGIX_DEV_ENGINE_BINARY`,
    /// `LOGIX_IDE_CHANNEL`, `LOGIX_IDE_DEV_REMOTE_CONFIG`, or the engine's
    /// own): their names, Logix IDE's first, then the engine's once it runs
    /// (sent again with all of them). The UI shows a banner for the whole
    /// session, pointing to Help > System Info for the details.
    DeveloperSettings {
        names: Vec<String>,
    },
}
