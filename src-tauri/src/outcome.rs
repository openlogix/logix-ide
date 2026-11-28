//! What a build, upload, test, simulation or diagram returns to the UI,
//! and the events it streams while running. Plain types, independent of
//! the engine's own result format.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Task {
    /// Verify: lint, synthesize, place and route.
    Build,
    /// Build, then program the board.
    Upload,
    /// Run one or all testbenches.
    Test,
    /// Simulate one testbench and show it in the Waveforms window.
    Simulate,
    /// Show Diagram: draw the design (every module) for the Diagram
    /// window.
    Diagram,
    /// Board Utilization: build without lint, for the report of the files
    /// as they are now (the engine skips a build that's up to date).
    Report,
    /// Debug: a debug build (the project's `debug.toml`) into its own
    /// folder, `_build_debug`, then program the board and open a debug
    /// session and its panel (the engine's debugger: `docs/debugger.md`).
    Debug,
}

/// A testbench that ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestbenchResult {
    pub name: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOutcome {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    /// For `Test`: each testbench that ran, in order.
    pub testbenches: Vec<TestbenchResult>,
    /// For a successful `Build` or `Upload`: resource usage and clocks.
    pub report: Option<BuildReport>,
    /// Stopped by the user (the Stop button).
    pub cancelled: bool,
    /// For a successful `Diagram`, and a Verify that redrew an open
    /// Diagram window: the engine's page. Not sent to the UI: the backend
    /// shows it in the Diagram window.
    #[serde(skip)]
    pub diagram_page: Option<String>,
    pub seconds: f64,
    /// The engine's suggested action when it failed, such as
    /// `connect_board` (see the engine's tool API), or `select_device` when
    /// Upload found several devices that fit the board (in `devices`).
    pub action: Option<String>,
    /// For Upload with `select_device`: the devices that fit the board, best
    /// first, for the user to choose from.
    pub devices: Vec<crate::engine::UsbDevice>,
    /// For Upload with `install_drivers`: the device that needs a driver
    /// (Windows: WinUSB) or a udev rule (Linux) first, for the user to agree;
    /// also with `confirm_same_ids`, when the engine refused because other
    /// connected devices have the same USB IDs.
    pub driver_device: Option<DriverDevice>,
    /// For Upload with `confirm_same_ids`: the engine's question, naming
    /// the other devices with the same USB IDs (they'd get WinUSB too).
    pub same_ids: Option<crate::engine::Consent>,
    /// For Simulate: the engine's waveform viewer page. Not sent to the
    /// UI: the backend shows it in the waveform window.
    #[serde(skip)]
    pub wave_page: Option<String>,
    /// The files the engine read and checked (its `checked_files`, or a
    /// design error's `data.checked_files`), so the UI replaces exactly
    /// those files' earlier problems with these (a pin file's problems stay
    /// after Run Tests, which never reads it). Empty when no tool ran.
    pub checked_files: Vec<String>,
    /// For a successful `Debug`: the engine's debug session, which stays
    /// open until Stop, Upload, another Debug or the panel closes.
    pub debug_session: Option<String>,
    /// For a successful `Debug`: the panel's address. Not sent to the UI:
    /// the backend shows it in the Debugger window.
    #[serde(skip)]
    pub debug_url: Option<String>,
}

/// A device that needs a driver before Upload can program it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverDevice {
    /// The engine's device ID, for Upload with `install_driver`.
    pub id: String,
    /// The engine's name for people.
    pub label: String,
    /// The engine's name for it in a sentence, such as "the Alhambra II".
    pub who: String,
    /// The engine's `driver` for it: what it needs and has, with the texts
    /// and the question to ask before adding it.
    pub driver: crate::engine::Driver,
}

/// Streamed to the UI while a task runs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TaskEvent {
    /// A console line (may contain ANSI colors). `tool` is the program that
    /// printed it (None for the IDE's and engine's own lines); `redraw`
    /// means the tool prints over it: the console replaces it with that
    /// tool's next line, as a terminal does; `location` is the place in a
    /// project file the line names, by the engine (`logix/output`'s
    /// `location`), so the console makes it a link.
    Line {
        line: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool: Option<String>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        redraw: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<Location>,
    },
    /// The current phase, such as "Placing and routing", and its
    /// percentage when known (status bar).
    Progress {
        message: String,
        percent: Option<u32>,
    },
    /// What the task is doing now, for the status bar, when it isn't what
    /// the task's name says (Upload installing the board's driver first);
    /// None: back to the task's own text ("Uploading…").
    Status { text: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    /// Project file name, if the message points into one.
    pub file: Option<String>,
    /// 1-based line, if known.
    pub line: Option<u32>,
    /// 1-based column, if known.
    pub column: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    /// FPGA family, e.g. "ice40", "ecp5", "gw1n", "artix7".
    pub family: String,
    /// Every resource, from the engine's `resources`: the toolchain's name,
    /// its description, used, available and the share (None when not
    /// valid), sorted by name. The Board Utilization window shows them.
    pub resources: Vec<Resource>,
    /// The design's clocks, from the engine's `clocks_summary`.
    pub clocks: Vec<Clock>,
    /// The engine's report for people (`report`), a line each, for the
    /// output pane.
    pub text: String,
    /// The engine's one-line report (`report_summary`), for the status bar.
    pub summary_line: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub name: String,
    pub description: Option<String>,
    pub used: u64,
    pub available: u64,
    pub percent: Option<f64>,
    /// The engine's share for people, such as "1%" (`display_percent`).
    pub display_percent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    pub name: String,
    pub net: String,
    pub fmax_mhz: f64,
}

/// A place in one of the request's files that a tool's output line names,
/// as Logix Engine gives it (`logix/output`'s `location`): the file named
/// as diagnostics name files, the line and, when the line gives one, the
/// column (both from 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

impl TaskEvent {
    /// A console line of the IDE's or the engine's own.
    pub fn line(line: String) -> Self {
        TaskEvent::Line {
            line,
            tool: None,
            redraw: false,
            location: None,
        }
    }
}
