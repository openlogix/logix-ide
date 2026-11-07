//! Verify, Upload, Run Tests, Simulate and Show Diagram with the engine, in
//! a prepared build folder (see `build_dir`): Verify is `lint` (unless
//! turned off) then `build`; Upload is `build` then `program_board`; the
//! others are `test`, `simulate` and `graph`. The tools' live output goes to
//! the console and the engine's phases to the status bar.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::client::{CallOptions, Engine, OutputKind, OutputLevel, OutputLine, ToolResult};
use super::phase_sink;
use crate::outcome::{
    dedup, BuildReport, Clock, Diagnostic, Resource, Severity, Task, TaskEvent, TaskOutcome,
    TestbenchResult,
};
use crate::project;

/// The design's top module: always main (see CLAUDE.md, Project on disk).
const TOP_MODULE: &str = "main";

/// Shown when the project has no pin file. The UI explains it in plain
/// words (`friendly` in `tasks.ts`).
pub const NO_CONSTRAINT_FILE: &str = "No constraint file (pin file) for the board.";

/// Upload's outcome action when several devices fit the board.
pub const SELECT_DEVICE: &str = "select_device";

/// Upload's device from the devices that fit the build: the only one, or
/// else the error to report (as the engine's errors look) and, with
/// several, the candidates.
fn pick_device(
    list: &super::DeviceList,
) -> std::result::Result<String, (Value, Vec<super::UsbDevice>)> {
    match list.devices.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => {
            let error = match list.unreachable.first() {
                Some(u) => json!({
                    "category": "setup",
                    "message": format!("The board was found but can't be opened. {}", u.action_description),
                    "action": {"code": u.action},
                }),
                None => json!({
                    "category": "setup",
                    "message": "The board isn't connected. Connect it by USB, then Upload again.",
                    "action": {"code": "connect_board"},
                }),
            };
            Err((error, Vec::new()))
        }
        several => Err((
            json!({
                "category": "setup",
                "message": format!("{} connected devices could be this board. Choose one to upload to.", several.len()),
                "action": {"code": SELECT_DEVICE},
            }),
            several.to_vec(),
        )),
    }
}

/// What `choose_device` decided.
enum Choice {
    Device(String),
    /// The outcome says why Upload can't go on.
    Stop,
    Cancelled,
}

/// What to run the task on.
pub struct TaskInput<'a> {
    pub board: &'a str,
    /// The board variant, as the engine's selector (e.g. `{"connection":
    /// "usb", "memory": "ram"}`); `None` for the board's default.
    pub board_variant: Option<&'a super::Variant>,
    /// The project's custom boards file (`custom-boards.json` in the
    /// project folder, on disk), as `custom_boards_file` with every engine
    /// call that deals with boards, when the project has one. Problems in
    /// it stop the task before it runs (`boards_problem_outcome`).
    pub custom_boards_file: Option<&'a Path>,
    /// The board's FPGA architecture, for the build report.
    pub arch: &'a str,
    /// The prepared build folder: the project's files and other contents.
    pub source_folder: &'a Path,
    /// The project's file names (top-level files).
    pub files: &'a [String],
    /// The pin constraint file, if the project has one.
    pub constraint_file: Option<&'a str>,
    /// Lint before building (Verify only; Settings).
    pub lint: bool,
    /// The testbench for Run Tests (all if `None`) and Simulate, or the
    /// module to draw for Show Diagram (`main` if `None`).
    pub target: Option<&'a str>,
    /// For Upload: the device the user chose (an engine device ID), when
    /// several fit the board. `None` lets Upload pick the only one.
    pub device: Option<&'a str>,
}

/// The engine's build folder inside the prepared folder: `_build` (the
/// engine wants "build" in one of its last two names). It is kept between
/// builds of the same project (`build_dir`), so up-to-date builds are fast.
pub fn build_folder(source_folder: &Path) -> PathBuf {
    source_folder.join("_build")
}

/// The design files: Verilog and SystemVerilog sources, without
/// testbenches and include files (the engine finds those itself).
fn design_files(files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter(|f| {
            let lower = f.to_ascii_lowercase();
            (lower.ends_with(".v") || lower.ends_with(".sv")) && !project::is_testbench(f)
        })
        .cloned()
        .collect()
}

/// The GTKWave layout for a testbench, next to it: `main_tb.v` ->
/// `main_tb.gtkw`. The engine writes a default one there unless the user
/// saved their own (`build_dir::save_back_gtkw` copies those back).
fn layout_file(testbench: &str) -> String {
    let stem = Path::new(testbench)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| testbench.to_string());
    format!("{stem}.gtkw")
}

/// A task that can't start because the project's custom boards file has
/// problems: they are its diagnostics, marked in the file like build
/// errors.
pub fn boards_problem_outcome(problem: &super::catalog::BoardsProblem) -> TaskOutcome {
    let mut diagnostics = problem.diagnostics.clone();
    if diagnostics.is_empty() {
        diagnostics.push(error(&problem.message));
    }
    TaskOutcome {
        ok: false,
        diagnostics,
        testbenches: Vec::new(),
        report: None,
        cancelled: false,
        graph_svg: None,
        seconds: 0.0,
        action: None,
        devices: Vec::new(),
    }
}

/// Runs `task` with the engine. `on_event` gets console lines and
/// progress.
pub async fn run<F>(
    engine: &Engine,
    task: Task,
    input: &TaskInput<'_>,
    on_event: F,
    cancel: &CancellationToken,
) -> Result<TaskOutcome>
where
    F: Fn(TaskEvent) + Send + Sync + Clone + 'static,
{
    let started = Instant::now();
    let runner = Runner {
        engine,
        on_event,
        cancel,
        sources: design_files(input.files),
        custom_boards_file: json!(input.custom_boards_file),
        folder: input.source_folder.to_string_lossy().into_owned(),
        build_folder: build_folder(input.source_folder),
    };
    let mut outcome = TaskOutcome {
        ok: false,
        diagnostics: Vec::new(),
        testbenches: Vec::new(),
        report: None,
        cancelled: false,
        graph_svg: None,
        seconds: 0.0,
        action: None,
        devices: Vec::new(),
    };
    let finished = match task {
        Task::Build | Task::Upload => runner.build(task, input, &mut outcome).await?,
        Task::Test => runner.test(input, &mut outcome).await?,
        Task::Simulate => runner.simulate(input, &mut outcome).await?,
        Task::Graph => runner.graph(input, &mut outcome).await?,
    };
    if !finished {
        outcome.ok = false;
        outcome.cancelled = true;
    }
    outcome.diagnostics = dedup(outcome.diagnostics);
    outcome.seconds = started.elapsed().as_secs_f64();
    Ok(outcome)
}

/// Runs one task's engine calls. Each step returns `Ok(false)` when it was
/// stopped (the Stop button), so the remaining steps are skipped.
struct Runner<'a, F> {
    engine: &'a Engine,
    on_event: F,
    cancel: &'a CancellationToken,
    /// The design files (`design_files`).
    sources: Vec<String>,
    /// The prepared folder, as the engine's `source_folder`.
    folder: String,
    /// The project's custom boards file (or null), for program_board and
    /// list_devices.
    custom_boards_file: Value,
    build_folder: PathBuf,
}

impl<F> Runner<'_, F>
where
    F: Fn(TaskEvent) + Send + Sync + Clone + 'static,
{
    /// Calls an engine tool, with its live output in the console and its
    /// phases in the status bar.
    async fn call(&self, name: &str, args: Value) -> Result<ToolResult> {
        let console = Console::new(self.on_event.clone());
        let on_event = self.on_event.clone();
        let progress = phase_sink(move |message, percent, _new| {
            on_event(TaskEvent::Progress {
                message: message.to_string(),
                percent,
            })
        });
        let options = CallOptions {
            on_progress: Some(progress),
            on_output: Some(console.sink()),
            cancel: Some(self.cancel.clone()),
        };
        let result = self.engine.call_with(name, args, options).await;
        console.flush();
        result
    }

    fn line(&self, line: String) {
        (self.on_event)(TaskEvent::Line { line });
    }

    fn build_folder(&self) -> String {
        self.build_folder.to_string_lossy().into_owned()
    }

    /// Verify (lint, then build) or Upload (build, then program the board).
    async fn build(
        &self,
        task: Task,
        input: &TaskInput<'_>,
        outcome: &mut TaskOutcome,
    ) -> Result<bool> {
        let Some(constraint_file) = input.constraint_file else {
            outcome.diagnostics.push(error(NO_CONSTRAINT_FILE));
            return Ok(true);
        };
        if task == Task::Build && input.lint {
            let result = self
                .call(
                    "lint",
                    json!({
                        "board": input.board,
                        "board_variant": input.board_variant,
                        "custom_boards_file": input.custom_boards_file,
                        "top_module": TOP_MODULE,
                        "source_folder": self.folder,
                        "source_files": self.sources,
                    }),
                )
                .await?;
            match result {
                // Lint fails for warnings too, so its result never stops
                // the build; only the build decides success.
                Ok(lint) => outcome
                    .diagnostics
                    .extend(diagnostics(&lint["diagnostics"])),
                Err(e) if is_cancelled(&e) => return Ok(false),
                Err(e) => self.line(red(&format!("Lint: {}", message(&e)))),
            }
        }

        let result = self
            .call(
                "build",
                json!({
                    "board": input.board,
                    "board_variant": input.board_variant,
                        "custom_boards_file": input.custom_boards_file,
                    "top_module": TOP_MODULE,
                    "source_folder": self.folder,
                    "source_files": self.sources,
                    "constraint_file": constraint_file,
                    "build_folder": self.build_folder(),
                }),
            )
            .await?;
        match result {
            Ok(build) => {
                outcome.diagnostics.extend(diagnostics(&build["warnings"]));
                outcome.report = Some(report(input.arch, &build));
                outcome.ok = true;
            }
            Err(e) if is_cancelled(&e) => return Ok(false),
            Err(e) => self.failed(outcome, &e),
        }

        if task == Task::Upload && outcome.ok {
            let device = match input.device {
                Some(id) => id.to_string(),
                None => match self.choose_device(outcome).await? {
                    Choice::Device(id) => id,
                    Choice::Stop => return Ok(true),
                    Choice::Cancelled => return Ok(false),
                },
            };
            let result = self
                .call(
                    "program_board",
                    json!({
                        "build_folder": self.build_folder(),
                        "device": device,
                        "custom_boards_file": self.custom_boards_file,
                    }),
                )
                .await?;
            match result {
                Ok(done) => {
                    let device = &done["device"];
                    let name = device["product"]
                        .as_str()
                        .or(device["manufacturer"].as_str())
                        .unwrap_or("the board");
                    self.line(format!("Programmed {name}."));
                }
                Err(e) if is_cancelled(&e) => return Ok(false),
                Err(e) => self.failed(outcome, &e),
            }
        }
        Ok(true)
    }

    /// Upload's device: the only one that fits the build's board (the
    /// engine's `list_devices` with the build folder). With none, or with
    /// several (`select_device`, the candidates in `outcome.devices` for the
    /// user to choose), the outcome says so and Upload stops.
    async fn choose_device(&self, outcome: &mut TaskOutcome) -> Result<Choice> {
        let result = self
            .call(
                "list_devices",
                json!({
                    "build_folder": self.build_folder(),
                    "custom_boards_file": self.custom_boards_file,
                }),
            )
            .await?;
        let list = match result {
            Ok(v) => super::devices::parse_devices(&v)?,
            Err(e) if is_cancelled(&e) => return Ok(Choice::Cancelled),
            Err(e) => {
                self.failed(outcome, &e);
                return Ok(Choice::Stop);
            }
        };
        match pick_device(&list) {
            Ok(id) => Ok(Choice::Device(id)),
            Err((error, candidates)) => {
                self.failed(outcome, &error);
                outcome.devices = candidates;
                Ok(Choice::Stop)
            }
        }
    }

    /// Run Tests: the chosen testbench, or all of the project's.
    async fn test(&self, input: &TaskInput<'_>, outcome: &mut TaskOutcome) -> Result<bool> {
        let testbenches: Vec<String> = match input.target {
            Some(tb) => vec![tb.to_string()],
            None => input
                .files
                .iter()
                .filter(|f| project::is_testbench(f))
                .cloned()
                .collect(),
        };
        if testbenches.is_empty() {
            outcome.diagnostics.push(error(
                "This project has no testbenches. Testbench file names end with _tb.v or _tb.sv.",
            ));
            return Ok(true);
        }
        let result = self
            .call(
                "test",
                json!({
                    "board": input.board,
                    "board_variant": input.board_variant,
                        "custom_boards_file": input.custom_boards_file,
                    "source_folder": self.folder,
                    "source_files": self.sources,
                    "build_folder": self.build_folder(),
                    "testbench_files": testbenches,
                }),
            )
            .await?;
        match result {
            Ok(test) => {
                for tb in test["testbenches"].as_array().into_iter().flatten() {
                    self.testbench_result(tb, outcome);
                }
                outcome.ok = test["passed"] == true;
            }
            Err(e) if is_cancelled(&e) => return Ok(false),
            Err(e) => self.failed(outcome, &e),
        }
        Ok(true)
    }

    /// Simulate: runs one testbench to the end and opens GTKWave on it,
    /// failing or not (the waveform is how the user finds the problem).
    async fn simulate(&self, input: &TaskInput<'_>, outcome: &mut TaskOutcome) -> Result<bool> {
        let Some(testbench) = input.target else {
            outcome
                .diagnostics
                .push(error("Choose the testbench to simulate."));
            return Ok(true);
        };
        let result = self
            .call(
                "simulate",
                json!({
                    "board": input.board,
                    "board_variant": input.board_variant,
                        "custom_boards_file": input.custom_boards_file,
                    "source_folder": self.folder,
                    "source_files": self.sources,
                    "build_folder": self.build_folder(),
                    "testbench_file": testbench,
                    "layout_file": layout_file(testbench),
                }),
            )
            .await?;
        match result {
            Ok(sim) => {
                let mut tb = sim.clone();
                tb["file"] = json!(testbench);
                self.testbench_result(&tb, outcome);
                outcome.ok = sim["passed"] == true;
            }
            Err(e) if is_cancelled(&e) => return Ok(false),
            Err(e) => self.failed(outcome, &e),
        }
        Ok(true)
    }

    /// Show Diagram: draws a module as an SVG.
    async fn graph(&self, input: &TaskInput<'_>, outcome: &mut TaskOutcome) -> Result<bool> {
        let result = self
            .call(
                "graph",
                json!({
                    "top_module": input.target.unwrap_or(TOP_MODULE),
                    "source_folder": self.folder,
                    "source_files": self.sources,
                    "build_folder": self.build_folder(),
                }),
            )
            .await?;
        match result {
            Ok(graph) => {
                outcome
                    .diagnostics
                    .extend(diagnostics(&graph["diagnostics"]));
                let file = graph["graph_file"].as_str().unwrap_or("graph.svg");
                let path = self.build_folder.join(file);
                let svg = tokio::fs::read_to_string(&path)
                    .await
                    .with_context(|| format!("can't read {}", path.display()))?;
                outcome.graph_svg = Some(relative_paths(&svg, &self.folder));
                outcome.ok = true;
            }
            Err(e) if is_cancelled(&e) => return Ok(false),
            Err(e) => self.failed(outcome, &e),
        }
        Ok(true)
    }

    /// Records one testbench's result (from `test`, or `simulate`'s
    /// result with its `file`): passed or not, its problems, and why it
    /// ended if it didn't end itself.
    fn testbench_result(&self, tb: &Value, outcome: &mut TaskOutcome) {
        let name = tb["file"].as_str().unwrap_or("").to_string();
        let passed = tb["passed"] == true;
        outcome.diagnostics.extend(diagnostics(&tb["diagnostics"]));
        if let Some(note) = ending_note(&name, tb["ending"].as_str().unwrap_or("")) {
            self.line(red(&note));
            if !passed {
                outcome.diagnostics.push(Diagnostic {
                    file: Some(name.clone()),
                    ..error(&note)
                });
            }
        }
        outcome.testbenches.push(TestbenchResult { name, passed });
    }

    /// Records a failed step: the error's diagnostics, or else its message
    /// as an error without a location, and the suggested action.
    fn failed(&self, outcome: &mut TaskOutcome, error: &Value) {
        failed(outcome, error, &|l| self.line(l));
    }
}

/// The diagram names the design's processes by file and line. The engine
/// passes yosys absolute paths, so those read like
/// `/Users/you/.logix/build/main.v:5.14-5.25`; shows them as `main.v:...`.
/// TODO(revisit): ask the engine to label them relative to the source
/// folder.
fn relative_paths(svg: &str, folder: &str) -> String {
    let mut svg = svg.replace(&format!("{folder}/"), "");
    // The engine resolves symbolic links (e.g. /var -> /private/var).
    if let Ok(real) = std::fs::canonicalize(folder) {
        svg = svg.replace(&format!("{}/", real.to_string_lossy()), "");
    }
    svg
}

/// Why a testbench run ended, when the engine stopped it or it ran out of
/// things to simulate (`ending` in the engine's test results).
fn ending_note(testbench: &str, ending: &str) -> Option<String> {
    match ending {
        "timeout" => Some(format!(
            "{testbench} was stopped because it ran too long. Does it end with $finish?"
        )),
        "sim_time_limit" => Some(format!(
            "{testbench} was stopped at its simulation time limit. Does it end with $finish?"
        )),
        "no_finish" => Some(format!(
            "{testbench} ended without calling $finish, which usually means it stopped early by mistake."
        )),
        _ => None,
    }
}

fn is_cancelled(error: &Value) -> bool {
    error["category"] == "cancelled"
}

/// The engine error's message, with its suggested action when there is
/// one (e.g. "No Alhambra II is connected. Connect the Alhambra II by USB,
/// then retry.").
fn message(error: &Value) -> String {
    let message = error["message"].as_str().unwrap_or("Unknown error.");
    match error["action"]["description"].as_str() {
        Some(action) => format!("{message} {action}"),
        None => message.to_string(),
    }
}

/// Records a failed step: the error's diagnostics, or else its message as
/// an error without a location, and the suggested action.
fn failed(outcome: &mut TaskOutcome, error: &Value, line: &impl Fn(String)) {
    outcome.ok = false;
    let found = diagnostics(&error["diagnostics"]);
    let has_error = found.iter().any(|d| d.severity == Severity::Error);
    outcome.diagnostics.extend(found);
    if !has_error {
        outcome.diagnostics.push(self::error(&message(error)));
    }
    outcome.action = error["action"]["code"].as_str().map(String::from);
    // A design error's message repeats the tool's error line, which the
    // console already shows.
    if error["category"] != "design" {
        line(red(&message(error)));
    }
}

fn error(message: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        file: None,
        line: None,
        column: None,
        message: message.to_string(),
    }
}

/// The engine's diagnostics (file relative to the source folder, which
/// holds the project's files under their own names). Its `info` ones are
/// shown as warnings, since the editor has only errors and warnings.
pub(super) fn diagnostics(list: &Value) -> Vec<Diagnostic> {
    let Some(list) = list.as_array() else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|d| {
            let number = |key: &str| d[key].as_u64().and_then(|n| u32::try_from(n).ok());
            Some(Diagnostic {
                severity: if d["severity"] == "error" {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                file: d["file"].as_str().map(String::from),
                line: number("line"),
                column: number("column"),
                message: d["text"].as_str()?.to_string(),
            })
        })
        .collect()
}

/// The build's resource use and clock speeds.
fn report(arch: &str, build: &Value) -> BuildReport {
    let resources = build["resources_usage"]
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(name, r)| Resource {
                    name: name.clone(),
                    used: r["used"].as_u64().unwrap_or(0),
                    available: r["available"].as_u64().unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default();
    // The engine's plain-language summary: `name` is its label.
    let summary = build["resources_summary"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|r| Resource {
                    name: r["label"].as_str().unwrap_or_default().to_string(),
                    used: r["used"].as_u64().unwrap_or(0),
                    available: r["available"].as_u64().unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default();
    // The engine's clocks with plain names (`clocks_summary`), such as
    // "sys_clk" for the net sys_clk$SB_IO_IN_$glb_clk.
    let clocks = match build["clocks_summary"].as_array() {
        Some(list) => list
            .iter()
            .filter_map(|c| {
                Some(Clock {
                    name: c["name"].as_str()?.to_string(),
                    fmax_mhz: c["max_mhz"].as_f64()?,
                })
            })
            .collect(),
        // TODO(revisit): an engine without clocks_summary (before
        // 2026-11-05): the net name up to its first `$`. Drop this once the
        // pinned engine has it.
        None => build["timing"]
            .as_object()
            .map(|map| {
                map.iter()
                    .filter_map(|(net, mhz)| {
                        let name = net.split('$').next().filter(|n| !n.is_empty());
                        Some(Clock {
                            name: name.unwrap_or(net).to_string(),
                            fmax_mhz: mhz.as_f64()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    BuildReport {
        arch: arch.to_string(),
        resources,
        summary,
        clocks,
    }
}

fn red(text: &str) -> String {
    format!("\x1b[31m{text}\x1b[0m")
}

/// Turns live output into console lines: command lines dimmed with `$`,
/// errors red, warnings yellow. A line the tool prints over (`redraw`, such
/// as a progress bar's) is held, and dropped when the same tool's next line
/// arrives, so a progress bar stays on one line, as in a terminal; anything
/// else first, or the end of the call, shows it.
#[derive(Clone)]
pub(super) struct Console<F> {
    on_event: F,
    /// The held redraw line: its tool and console text.
    held: Arc<Mutex<Option<(String, String)>>>,
}

impl<F> Console<F>
where
    F: Fn(TaskEvent) + Send + Sync + Clone + 'static,
{
    pub(super) fn new(on_event: F) -> Self {
        Console {
            on_event,
            held: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn sink(&self) -> super::client::OutputSink {
        let console = self.clone();
        Arc::new(move |line: OutputLine| console.take(line))
    }

    fn send(&self, line: String) {
        (self.on_event)(TaskEvent::Line { line });
    }

    fn take(&self, line: OutputLine) {
        let text = match (line.kind, line.level) {
            (OutputKind::Command, _) => format!("\x1b[2m$ {}\x1b[0m", line.text),
            (_, OutputLevel::Error) => red(&line.text),
            (_, OutputLevel::Warning) => format!("\x1b[33m{}\x1b[0m", line.text),
            (_, OutputLevel::Info) => line.text.clone(),
        };
        let output = line.kind == OutputKind::Output;
        let mut held = self.held.lock().unwrap();
        if let Some((tool, previous)) = held.take() {
            // The tool printed over it: drop it; otherwise show it.
            if !(output && tool == line.tool) {
                self.send(previous);
            }
        }
        if output && line.redraw {
            *held = Some((line.tool, text));
        } else {
            self.send(text);
        }
    }

    /// Sends a held redraw line (at the end of a call).
    pub(super) fn flush(&self) {
        if let Some((_, text)) = self.held.lock().unwrap().take() {
            self.send(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_picks_the_only_device_or_asks() {
        let list = |devices: serde_json::Value| {
            super::super::devices::parse_devices(&json!({ "devices": devices })).unwrap()
        };
        let dev = |id: &str| json!({"id": id, "kind": "usb", "vid": "0403", "pid": "6010", "build_match": "strong"});
        assert_eq!(pick_device(&list(json!([dev("a")]))).unwrap(), "a");

        let (err, candidates) = pick_device(&list(json!([dev("a"), dev("b")]))).unwrap_err();
        assert_eq!(err["action"]["code"], SELECT_DEVICE);
        assert_eq!(candidates.len(), 2);

        let (err, candidates) = pick_device(&list(json!([]))).unwrap_err();
        assert_eq!(err["action"]["code"], "connect_board");
        assert!(candidates.is_empty());

        let locked = super::super::devices::parse_devices(&json!({"devices": [], "unreachable": [
            {"vid": "0403", "pid": "6010", "action": {"code": "fix_device_permissions", "description": "Allow it."}}
        ]})).unwrap();
        let (err, _) = pick_device(&locked).unwrap_err();
        assert_eq!(err["action"]["code"], "fix_device_permissions");
        assert!(err["message"].as_str().unwrap().contains("Allow it."));
    }

    /// The default configuration of the test boards.

    #[test]
    fn picks_design_files() {
        let files: Vec<String> = ["main.v", "main_tb.v", "uart.sv", "defs.vh", "pinout.pcf"]
            .map(String::from)
            .into();
        assert_eq!(design_files(&files), ["main.v", "uart.sv"]);
    }

    #[test]
    fn reads_engine_diagnostics() {
        let list = json!([
            {"file": "main.v", "line": 25, "column": 3, "severity": "error", "text": "syntax error", "tool": "yosys"},
            {"severity": "info", "text": "note"},
            {"severity": "warning"}
        ]);
        let found = diagnostics(&list);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].file.as_deref(), Some("main.v"));
        assert_eq!((found[0].line, found[0].column), (Some(25), Some(3)));
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(found[1].severity, Severity::Warning);
        assert!(diagnostics(&Value::Null).is_empty());
    }

    #[test]
    fn reads_the_build_report() {
        let build = json!({
            "resources_usage": {"ICESTORM_LC": {"available": 7680, "used": 27}},
            "resources_summary": [
                {"kind": "logic_cells", "label": "logic cells", "used": 27, "available": 7680}
            ],
            "timing": {"sys_clk$SB_IO_IN_$glb_clk": 194.36},
            "clocks_summary": [
                {"name": "sys_clk", "net": "sys_clk$SB_IO_IN_$glb_clk", "max_mhz": 194.36}
            ]
        });
        let r = report("ice40", &build);
        assert_eq!(
            r.summary,
            [Resource {
                name: "logic cells".into(),
                used: 27,
                available: 7680
            }]
        );
        assert_eq!(r.arch, "ice40");
        assert_eq!(
            r.resources,
            [Resource {
                name: "ICESTORM_LC".into(),
                used: 27,
                available: 7680
            }]
        );
        assert_eq!(
            r.clocks,
            [Clock {
                name: "sys_clk".into(),
                fmax_mhz: 194.36
            }]
        );
        // An engine without clocks_summary: the net name up to its first $.
        let old = json!({"timing": {"sys_clk$SB_IO_IN_$glb_clk": 194.36, "$abc$1$o100": 300.0}});
        let names: Vec<_> = report("ice40", &old)
            .clocks
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["$abc$1$o100", "sys_clk"]);
    }

    #[test]
    fn failures_keep_their_message_and_action() {
        let mut outcome = TaskOutcome {
            ok: true,
            diagnostics: Vec::new(),
            testbenches: Vec::new(),
            report: None,
            cancelled: false,
            graph_svg: None,
            seconds: 0.0,
            action: None,
            devices: Vec::new(),
        };
        let lines = Mutex::new(Vec::new());
        let e = json!({
            "category": "setup",
            "message": "No Alhambra II is connected.",
            "action": {"code": "connect_board", "description": "Connect the Alhambra II by USB, then retry."},
            "diagnostics": []
        });
        failed(&mut outcome, &e, &|l| lines.lock().unwrap().push(l));
        assert!(!outcome.ok);
        assert_eq!(outcome.action.as_deref(), Some("connect_board"));
        assert_eq!(
            outcome.diagnostics[0].message,
            "No Alhambra II is connected. Connect the Alhambra II by USB, then retry."
        );
        // A design error's located diagnostics are enough.
        let mut outcome2 = outcome.clone();
        outcome2.diagnostics.clear();
        let e = json!({
            "category": "design",
            "message": "The design has errors. main.v:25: syntax error",
            "diagnostics": [{"file": "main.v", "line": 25, "severity": "error", "text": "syntax error"}]
        });
        failed(&mut outcome2, &e, &|_| {});
        assert_eq!(outcome2.diagnostics.len(), 1);
        assert_eq!(outcome2.diagnostics[0].file.as_deref(), Some("main.v"));
    }

    fn out(tool: &str, kind: OutputKind, level: OutputLevel, text: &str) -> OutputLine {
        OutputLine {
            tool: tool.into(),
            kind,
            level,
            text: text.into(),
            redraw: false,
        }
    }

    /// A line the tool prints over (`redraw`).
    fn redraw(tool: &str, text: &str) -> OutputLine {
        OutputLine {
            redraw: true,
            ..out(tool, OutputKind::Output, OutputLevel::Info, text)
        }
    }

    #[test]
    fn console_colors_lines_and_replaces_redraws() {
        let got = Arc::new(Mutex::new(Vec::new()));
        let console = {
            let got = got.clone();
            Console::new(move |e| {
                if let TaskEvent::Line { line } = e {
                    got.lock().unwrap().push(line)
                }
            })
        };
        let sink = console.sink();
        use OutputKind::*;
        use OutputLevel::*;
        let ofl = "openFPGALoader";
        sink(out(
            ofl,
            Command,
            Info,
            "openFPGALoader -b ice40_generic hardware.bin",
        ));
        sink(redraw(ofl, "Writing: [==>     ] 10.00%"));
        sink(redraw(ofl, "Writing: [=====>  ] 60.00%"));
        sink(out(ofl, Output, Info, "Writing: [========] 100.00% Done"));
        sink(redraw(ofl, "Verifying: [==>     ] 10.00%"));
        sink(redraw(ofl, "Verifying: [========] 100.00%"));
        console.flush();
        // iceprog's progress: the address lines are redraws, and the last
        // one is printed over by "done.", as in a terminal.
        sink(out("iceprog", Output, Info, "programming.."));
        sink(redraw("iceprog", "addr 0x000000   0%"));
        sink(redraw("iceprog", "addr 0x020F00  99%"));
        sink(out("iceprog", Output, Info, "done."));
        // A redraw followed by another tool's line or a message stays.
        sink(redraw("iceprog", "addr 0x000100   0%"));
        sink(out("logix-engine", Message, Info, "Stopped."));
        sink(out(
            "yosys",
            Output,
            Error,
            "main.v:25: ERROR: syntax error",
        ));
        sink(out("yosys", Output, Warning, "a warning"));
        assert_eq!(
            *got.lock().unwrap(),
            [
                "\x1b[2m$ openFPGALoader -b ice40_generic hardware.bin\x1b[0m",
                "Writing: [========] 100.00% Done",
                "Verifying: [========] 100.00%",
                "programming..",
                "done.",
                "addr 0x000100   0%",
                "Stopped.",
                "\x1b[31mmain.v:25: ERROR: syntax error\x1b[0m",
                "\x1b[33ma warning\x1b[0m",
            ]
        );
    }

    /// Verify, Upload, Run Tests, Show Diagram and Auto Format against a
    /// real engine, on the alhambra-ii blinky example (no board
    /// needed: Upload fails with `connect_board` unless one is plugged in;
    /// Simulate isn't run, since it opens GTKWave):
    /// `LOGIX_DEV_ENGINE_BINARY=<logix-engine> LOGIX_IDE_TEST_ENGINE_FOLDER=<engine folder with packages> cargo test real_engine_build -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_engine_build() {
        let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
        let engine = Engine::start(
            Path::new(&var("LOGIX_DEV_ENGINE_BINARY")),
            Path::new(&var("LOGIX_IDE_TEST_ENGINE_FOLDER")),
        )
        .await
        .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("build");
        let files = example_files("alhambra-ii", "blinky", &dir);
        let constraint = crate::build_dir::constraint_file_name(&files).map(str::to_owned);
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let events = events.clone();
            move |e: TaskEvent| events.lock().unwrap().push(e)
        };
        let mut input = TaskInput {
            board: "alhambra-ii",
            board_variant: None,
            custom_boards_file: None,
            arch: "ice40",
            source_folder: &dir,
            files: &files,
            constraint_file: constraint.as_deref(),
            lint: true,
            target: None,
            device: None,
        };
        let cancel = CancellationToken::new();
        let good = run(&engine, Task::Build, &input, sink.clone(), &cancel)
            .await
            .unwrap();
        for e in events.lock().unwrap().iter() {
            println!("{e:?}");
        }
        assert!(good.ok, "{good:?}");
        assert!(good.report.as_ref().unwrap().resources.len() > 3);

        // The language server comes from the engine's verible package.
        let ls = super::super::program_path(&engine, "verible", "verible-verilog-ls")
            .await
            .unwrap();
        assert!(ls.is_file(), "{}", ls.display());

        // A syntax error is marked at its line.
        let main = dir.join("main.v");
        let text = std::fs::read_to_string(&main).unwrap();
        std::fs::write(&main, format!("{text}\noops\n")).unwrap();
        let bad = run(&engine, Task::Build, &input, sink.clone(), &cancel)
            .await
            .unwrap();
        println!("{bad:?}");
        assert!(!bad.ok);
        assert!(bad
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("main.v") && d.line.is_some()));

        // Upload: builds, then looks for the board.
        std::fs::write(&main, text).unwrap();
        input.lint = false;
        let upload = run(&engine, Task::Upload, &input, sink.clone(), &cancel)
            .await
            .unwrap();
        println!("{upload:?}");
        assert!(upload.ok || upload.action.as_deref() == Some("connect_board"));

        // Run Tests: all testbenches, then one that fails.
        events.lock().unwrap().clear();
        let test = run(&engine, Task::Test, &input, sink.clone(), &cancel)
            .await
            .unwrap();
        for e in events.lock().unwrap().iter() {
            println!("{e:?}");
        }
        println!("{test:?}");
        assert!(test.ok, "{test:?}");
        assert_eq!(test.testbenches.len(), 1);
        assert!(test.testbenches[0].passed);
        let tb = dir.join("main_tb.v");
        let tb_text = std::fs::read_to_string(&tb).unwrap();
        std::fs::write(
            &tb,
            tb_text.replacen("`EXPECT_EQ(leds, 8'b0)", "`EXPECT_EQ(leds, 8'b1)", 1),
        )
        .unwrap();
        input.target = Some("main_tb.v");
        let failed = run(&engine, Task::Test, &input, sink.clone(), &cancel)
            .await
            .unwrap();
        println!("{failed:?}");
        assert!(!failed.ok);
        assert!(!failed.testbenches[0].passed);
        assert!(failed
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("main_tb.v") && d.line.is_some()));
        std::fs::write(&tb, tb_text).unwrap();

        // Show Diagram.
        input.target = None;
        let graph = run(&engine, Task::Graph, &input, sink.clone(), &cancel)
            .await
            .unwrap();
        assert!(graph.ok, "{graph:?}");
        assert!(graph.graph_svg.unwrap().contains("<svg"));
        input.target = Some("nope");
        let no_module = run(&engine, Task::Graph, &input, sink, &cancel)
            .await
            .unwrap();
        println!("{no_module:?}");
        assert!(!no_module.ok);

        // Auto Format: a messy file is formatted, a broken one reports
        // where the problem is.
        let format_dir = tmp.path().join("format");
        std::fs::create_dir_all(&format_dir).unwrap();
        let messy = format_dir.join("alu.v");
        std::fs::write(
            &messy,
            "module alu(input a,output b);assign b=a;\n  endmodule\n",
        )
        .unwrap();
        let ok = super::super::format_file(&engine, &format_dir, "alu.v", |l| println!("{l}"))
            .await
            .unwrap();
        assert!(ok.ok);
        assert!(std::fs::read_to_string(&messy)
            .unwrap()
            .contains("assign b = a;"));
        std::fs::write(
            format_dir.join("bad.v"),
            "module x;\ninitial begin\nend end\nendmodule\n",
        )
        .unwrap();
        let bad = super::super::format_file(&engine, &format_dir, "bad.v", |l| println!("{l}"))
            .await
            .unwrap();
        println!("{bad:?}");
        assert!(!bad.ok);
        assert!(bad
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("bad.v") && d.line == Some(3)));
    }

    /// Writes an example into `dir` and returns its project file names.
    fn example_files(board: &str, name: &str, dir: &Path) -> Vec<String> {
        crate::examples::write(board, name, dir).unwrap();
        crate::project::load(dir)
            .unwrap()
            .files
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    /// Every example builds and passes its tests on a real engine, and so
    /// does every board's Bare Minimum (slow: all 135 examples):
    /// `LOGIX_DEV_ENGINE_BINARY=<logix-engine> LOGIX_IDE_TEST_ENGINE_FOLDER=<engine folder with packages> cargo test real_examples_build -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_examples_build() {
        let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
        let engine = Engine::start(
            Path::new(&var("LOGIX_DEV_ENGINE_BINARY")),
            Path::new(&var("LOGIX_IDE_TEST_ENGINE_FOLDER")),
        )
        .await
        .unwrap();
        let catalog = super::super::catalog(&engine).await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let mut failed = Vec::new();
        let all = crate::examples::all();
        let run_one = |board: String, label: String, dir: PathBuf, files: Vec<String>| {
            let arch = catalog
                .boards
                .iter()
                .find(|b| b.id == board)
                .map(|b| b.arch.clone())
                .unwrap_or_default();
            let engine = &engine;
            let cancel = &cancel;
            async move {
                let constraint = crate::build_dir::constraint_file_name(&files).map(str::to_owned);
                let mut input = TaskInput {
                    board: &board,
                    board_variant: None,
                    custom_boards_file: None,
                    arch: &arch,
                    source_folder: &dir,
                    files: &files,
                    constraint_file: constraint.as_deref(),
                    lint: false,
                    target: None,
                    device: None,
                };
                let mut problems = Vec::new();
                let build = run(engine, Task::Build, &input, |_| {}, cancel)
                    .await
                    .unwrap();
                if !build.ok {
                    problems.push(format!("build: {:?}", build.diagnostics));
                }
                if files.iter().any(|f| project::is_testbench(f)) {
                    input.target = None;
                    let test = run(engine, Task::Test, &input, |_| {}, cancel)
                        .await
                        .unwrap();
                    if !test.ok {
                        problems.push(format!("test: {:?}", test.diagnostics));
                    }
                }
                println!(
                    "{label}: {}",
                    if problems.is_empty() { "ok" } else { "FAILED" }
                );
                (problems.is_empty(), format!("{label}: {problems:?}"))
            }
        };
        for (i, e) in all.iter().enumerate() {
            let dir = tmp.path().join(format!("e{i}"));
            let files = example_files(&e.board, &e.name, &dir);
            let label = format!("{}/{}", e.board, e.name);
            let (ok, why) = run_one(e.board.clone(), label, dir, files).await;
            if !ok {
                failed.push(why);
            }
            // The board's Bare Minimum, from its starter example.
            if crate::examples::starter_example(&all, &e.board).map(|s| &s.name) == Some(&e.name) {
                let dir = tmp.path().join(format!("b{i}"));
                std::fs::create_dir_all(&dir).unwrap();
                let starter = crate::project::load(&tmp.path().join(format!("e{i}"))).unwrap();
                let bare = crate::bare::bare_files(&starter.files).unwrap();
                for f in &bare {
                    std::fs::write(dir.join(&f.name), &f.text).unwrap();
                }
                let files = bare.into_iter().map(|f| f.name).collect();
                let label = format!("{} Bare Minimum", e.board);
                let (ok, why) = run_one(e.board.clone(), label, dir, files).await;
                if !ok {
                    failed.push(why);
                }
            }
        }
        for f in &failed {
            println!("{f}");
        }
        assert!(failed.is_empty(), "{} failed", failed.len());
    }

    /// Recaptures the fake backend's fixtures (`ui/e2e/fixtures/`) from a
    /// real engine, for the fake's blinky (in `fakeBackend.ts`):
    /// `LOGIX_DEV_ENGINE_BINARY=<logix-engine> LOGIX_IDE_TEST_ENGINE_FOLDER=<engine folder with packages> LOGIX_IDE_TEST_FIXTURES_OUT=../ui/e2e/fixtures cargo test capture_fixtures -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn capture_fixtures() {
        let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
        let out = PathBuf::from(var("LOGIX_IDE_TEST_FIXTURES_OUT"));
        let engine = Engine::start(
            Path::new(&var("LOGIX_DEV_ENGINE_BINARY")),
            Path::new(&var("LOGIX_IDE_TEST_ENGINE_FOLDER")),
        )
        .await
        .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().canonicalize().unwrap().join("build");
        std::fs::create_dir_all(&dir).unwrap();
        let blinky = "module main (\n    input sys_clk,\n    output led\n);\n  reg [23:0] counter = 0;\n  always @(posedge sys_clk) counter <= counter + 1;\n  assign led = counter[23];\nendmodule\n";
        let tb = "`timescale 1ns / 1ps\nmodule main_tb;\n  initial begin\n    $dumpvars(0, main_tb);\n    #100;\n    $finish;\n  end\nendmodule\n";
        std::fs::write(dir.join("main_tb.v"), tb).unwrap();
        std::fs::write(dir.join("pinout.pcf"), "set_io sys_clk 49\nset_io led 45\n").unwrap();
        let files: Vec<String> = ["main.v", "main_tb.v", "pinout.pcf"]
            .map(String::from)
            .into();
        // Scratch paths as they are in Logix.
        let home = std::env::var("HOME").unwrap();
        let tidy = |line: &str| {
            line.replace(&dir.to_string_lossy().into_owned(), "~/.logix/build")
                .replace(&home, "~")
        };
        let capture = |task: Task, target: Option<&'static str>, main: &'static str, name: &str| {
            std::fs::write(dir.join("main.v"), main).unwrap();
            let lines = Arc::new(Mutex::new(Vec::new()));
            let sink = {
                let lines = lines.clone();
                move |e: TaskEvent| {
                    if let TaskEvent::Line { line } = e {
                        lines.lock().unwrap().push(line)
                    }
                }
            };
            let input = TaskInput {
                board: "alhambra-ii",
                board_variant: None,
                custom_boards_file: None,
                arch: "ice40",
                source_folder: &dir,
                files: &files,
                constraint_file: Some("pinout.pcf"),
                lint: true,
                target,
                device: None,
            };
            let engine = &engine;
            let path = out.join(name);
            let tidy = &tidy;
            async move {
                let outcome = run(engine, task, &input, sink, &CancellationToken::new())
                    .await
                    .unwrap();
                let text: Vec<String> = lines.lock().unwrap().iter().map(|l| tidy(l)).collect();
                std::fs::write(path, text.join("\n") + "\n").unwrap();
                outcome
            }
        };
        let broken = blinky.replace("counter[23];", "counter[23]").leak();
        // Built from scratch, so each command line is shown.
        let _ = std::fs::remove_dir_all(dir.join("_build"));
        assert!(capture(Task::Build, None, blinky, "verify.txt").await.ok);
        assert!(
            !capture(Task::Build, None, broken, "verify-error.txt")
                .await
                .ok
        );
        assert!(capture(Task::Test, None, blinky, "test.txt").await.ok);
        let graph = capture(Task::Graph, None, blinky, "graph.txt").await;
        std::fs::write(out.join("graph.svg"), graph.graph_svg.unwrap()).unwrap();
    }

    #[test]
    fn diagram_paths_are_relative() {
        let svg = "<text>PROC $3 /Users/you/.logix/build/main.v:5.14-5.25</text>";
        assert_eq!(
            relative_paths(svg, "/Users/you/.logix/build"),
            "<text>PROC $3 main.v:5.14-5.25</text>"
        );
    }

    #[test]
    fn layout_next_to_the_testbench() {
        assert_eq!(layout_file("main_tb.v"), "main_tb.gtkw");
        assert_eq!(layout_file("uart_tb.sv"), "uart_tb.gtkw");
    }

    #[test]
    fn explains_how_a_testbench_ended() {
        assert!(ending_note("main_tb.v", "finished").is_none());
        assert!(ending_note("main_tb.v", "first_failure").is_none());
        assert!(ending_note("main_tb.v", "no_finish")
            .unwrap()
            .contains("without calling $finish"));
        assert!(ending_note("main_tb.v", "timeout")
            .unwrap()
            .starts_with("main_tb.v"));
    }
}
