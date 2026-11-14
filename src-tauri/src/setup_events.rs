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
    /// Setup uses a local engine (`LOGIX_DEV_ENGINE_BINARY`) instead of the
    /// release the remote config names, for diagnosis: the UI shows it for
    /// the whole session.
    DevEngine {
        path: String,
    },
    /// Setup reads the remote config from the override
    /// (`LOGIX_IDE_DEV_REMOTE_CONFIG`), a URL or a local file, for testing a
    /// config change: the UI shows it for the whole session.
    DevRemoteConfig {
        location: String,
    },
    /// Setup reads the release channel the override names
    /// (`LOGIX_IDE_CHANNEL`) instead of the one `channels.jsonc` gives: the
    /// UI shows it for the whole session.
    ChannelOverride {
        channel: String,
    },
}
