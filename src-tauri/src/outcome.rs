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
    /// Simulate one testbench and open GTKWave.
    Simulate,
    /// Draw a module of the design as an SVG.
    Graph,
    /// Show Build Report: build without lint, for the report of the files
    /// as they are now (the engine skips a build that's up to date).
    Report,
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
    /// For a successful `Graph`: the diagram as SVG.
    pub graph_svg: Option<String>,
    pub seconds: f64,
    /// The engine's suggested action when it failed, such as
    /// `connect_board` (see the engine's tool API), or `select_device` when
    /// Upload found several devices that fit the board (in `devices`).
    pub action: Option<String>,
    /// For Upload with `select_device`: the devices that fit the board, best
    /// first, for the user to choose from.
    pub devices: Vec<crate::engine::UsbDevice>,
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
    /// tool's next line, as a terminal does.
    Line {
        line: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool: Option<String>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        redraw: bool,
    },
    /// The current phase, such as "Placing and routing", and its
    /// percentage when known (status bar).
    Progress {
        message: String,
        percent: Option<u32>,
    },
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
    /// valid), sorted by name. The Build Report window shows them.
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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    pub name: String,
    pub net: String,
    pub fmax_mhz: f64,
}

/// Drops repeated reports of the same problem (e.g. lint and the build
/// both flagging one line), keeping the first.
pub fn dedup(diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut kept: Vec<Diagnostic> = Vec::new();
    for d in diags {
        let repeat = kept.iter().any(|k| {
            k.severity == d.severity
                && k.file == d.file
                && k.line == d.line
                && (d.file.is_some() || k.message == d.message)
        });
        if !repeat {
            kept.push(d);
        }
    }
    kept
}

impl TaskEvent {
    /// A console line of the IDE's or the engine's own.
    pub fn line(line: String) -> Self {
        TaskEvent::Line {
            line,
            tool: None,
            redraw: false,
        }
    }
}
