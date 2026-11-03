//! Verify and Upload with the engine, in a prepared build folder (see
//! `build_dir`): Verify is `lint` (unless turned off) then `build`; Upload
//! is `build` then `program_board`. The tools' live output goes to the
//! console and the engine's phases to the status bar.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

use anyhow::Result;
use regex::Regex;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::client::{CallOptions, Engine, OutputKind, OutputLevel, OutputLine, ToolResult};
use super::phase_sink;
use crate::outcome::{
    dedup, BuildReport, Clock, Diagnostic, Resource, Severity, Task, TaskEvent, TaskOutcome,
};
use crate::project;

/// The design's top module, as in Apio's examples.
const TOP_MODULE: &str = "main";

/// Shown when the project has no pin file. The UI explains it in plain
/// words (`friendly` in `tasks.ts`).
pub const NO_CONSTRAINT_FILE: &str = "No constraint file (pin file) for the board.";

/// What to build.
pub struct BuildInput<'a> {
    pub board: &'a str,
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
}

/// The engine's build folder inside the prepared folder. The engine wants
/// "build" in one of its last two names; `_build` is kept between builds
/// of the same project (`build_dir`), so up-to-date builds are fast.
pub fn build_folder(source_folder: &Path) -> PathBuf {
    source_folder.join("_build").join("logix")
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

/// Runs Verify (`Task::Build`) or Upload with the engine. `on_event` gets
/// console lines and progress.
pub async fn run<F>(
    engine: &Engine,
    task: Task,
    input: &BuildInput<'_>,
    on_event: F,
    cancel: &CancellationToken,
) -> Result<TaskOutcome>
where
    F: Fn(TaskEvent) + Send + Sync + Clone + 'static,
{
    let started = Instant::now();
    let mut outcome = TaskOutcome {
        ok: false,
        diagnostics: Vec::new(),
        testbenches: Vec::new(),
        report: None,
        cancelled: false,
        graph_svg: None,
        seconds: 0.0,
        action: None,
    };
    let sources = design_files(input.files);
    let folder = input.source_folder.to_string_lossy().into_owned();
    let build_folder = build_folder(input.source_folder)
        .to_string_lossy()
        .into_owned();
    let line = {
        let on_event = on_event.clone();
        move |line: String| on_event(TaskEvent::Line { line })
    };
    let Some(constraint_file) = input.constraint_file else {
        outcome.diagnostics.push(error(NO_CONSTRAINT_FILE));
        outcome.seconds = started.elapsed().as_secs_f64();
        return Ok(outcome);
    };
    let call = |name: &'static str, args: Value| {
        let on_event = on_event.clone();
        async move {
            let console = Console::new(on_event.clone());
            let progress = phase_sink(move |message, percent, _new| {
                on_event(TaskEvent::Progress {
                    message: message.to_string(),
                    percent,
                })
            });
            let options = CallOptions {
                on_progress: Some(progress),
                on_output: Some(console.sink()),
                cancel: Some(cancel.clone()),
            };
            let result = engine.call_with(name, args, options).await;
            console.flush();
            result
        }
    };

    if task == Task::Build && input.lint {
        let result = call(
            "lint",
            json!({
                "board": input.board,
                "top_module": TOP_MODULE,
                "source_folder": folder,
                "source_files": sources,
            }),
        )
        .await?;
        match result {
            // Lint fails for warnings too, so its result never stops the
            // build; only the build decides success.
            Ok(lint) => outcome
                .diagnostics
                .extend(diagnostics(&lint["diagnostics"])),
            Err(e) if is_cancelled(&e) => return Ok(stopped(outcome, started)),
            Err(e) => line(red(&format!("Lint: {}", message(&e)))),
        }
    }

    let result = call(
        "build",
        json!({
            "board": input.board,
            "top_module": TOP_MODULE,
            "source_folder": folder,
            "source_files": sources,
            "constraint_file": constraint_file,
            "build_folder": build_folder,
        }),
    )
    .await?;
    match result {
        Ok(build) => {
            outcome.diagnostics.extend(diagnostics(&build["warnings"]));
            outcome.report = Some(report(input.arch, &build));
            outcome.ok = true;
        }
        Err(e) => {
            if is_cancelled(&e) {
                return Ok(stopped(outcome, started));
            }
            failed(&mut outcome, &e, &line);
        }
    }

    if task == Task::Upload && outcome.ok {
        let result: ToolResult =
            call("program_board", json!({ "build_folder": build_folder })).await?;
        match result {
            Ok(done) => {
                let device = &done["device"];
                let name = device["product"]
                    .as_str()
                    .or(device["manufacturer"].as_str())
                    .unwrap_or("the board");
                line(format!("Programmed {name}."));
            }
            Err(e) => {
                if is_cancelled(&e) {
                    return Ok(stopped(outcome, started));
                }
                outcome.ok = false;
                failed(&mut outcome, &e, &line);
            }
        }
    }

    outcome.diagnostics = dedup(outcome.diagnostics);
    outcome.seconds = started.elapsed().as_secs_f64();
    Ok(outcome)
}

fn is_cancelled(error: &Value) -> bool {
    error["category"] == "cancelled"
}

fn stopped(mut outcome: TaskOutcome, started: Instant) -> TaskOutcome {
    outcome.ok = false;
    outcome.cancelled = true;
    outcome.seconds = started.elapsed().as_secs_f64();
    outcome
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
fn diagnostics(list: &Value) -> Vec<Diagnostic> {
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
    let clocks = build["timing"]
        .as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(name, mhz)| {
                    Some(Clock {
                        name: name.clone(),
                        fmax_mhz: mhz.as_f64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    BuildReport {
        arch: arch.to_string(),
        resources,
        clocks,
    }
}

fn red(text: &str) -> String {
    format!("\x1b[31m{text}\x1b[0m")
}

/// A progress bar line, such as openFPGALoader's `Writing: [====  ] 42.00%`.
/// The engine sends one line per redraw.
static PROGRESS_BAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<label>[^\[]*)\[[^\]]*\]\s*\d{1,3}(?:\.\d+)?\s*%").unwrap());

/// Turns live output into console lines: command lines dimmed with `$`,
/// errors red, warnings yellow. A progress bar's redraws become one line
/// (its last state, as a terminal shows it): a redraw is held until the
/// next line shows whether it was the last one. TODO(revisit): if the
/// engine marks redraws itself, drop the pattern.
#[derive(Clone)]
struct Console<F> {
    on_event: F,
    held: Arc<Mutex<Option<(String, String)>>>,
}

impl<F> Console<F>
where
    F: Fn(TaskEvent) + Send + Sync + Clone + 'static,
{
    fn new(on_event: F) -> Self {
        Console {
            on_event,
            held: Arc::new(Mutex::new(None)),
        }
    }

    fn sink(&self) -> super::client::OutputSink {
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
        let bar = (line.kind == OutputKind::Output)
            .then(|| PROGRESS_BAR.captures(&line.text))
            .flatten()
            .map(|c| format!("{}\u{0}{}", line.tool, c["label"].trim()));
        let mut held = self.held.lock().unwrap();
        match (held.take(), bar) {
            // Another redraw of the held bar: keep only the newer one.
            (Some((key, _)), Some(bar)) if key == bar => *held = Some((bar, text)),
            (previous, bar) => {
                if let Some((_, previous)) = previous {
                    self.send(previous);
                }
                match bar {
                    Some(bar) => *held = Some((bar, text)),
                    None => self.send(text),
                }
            }
        }
    }

    /// Sends a held progress bar line (at the end of a call).
    fn flush(&self) {
        if let Some((_, text)) = self.held.lock().unwrap().take() {
            self.send(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "timing": {"sys_clk$SB_IO_IN_$glb_clk": 194.36}
        });
        let r = report("ice40", &build);
        assert_eq!(r.arch, "ice40");
        assert_eq!(
            r.resources,
            [Resource {
                name: "ICESTORM_LC".into(),
                used: 27,
                available: 7680
            }]
        );
        assert_eq!(r.clocks[0].name, "sys_clk$SB_IO_IN_$glb_clk");
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
        }
    }

    #[test]
    fn console_colors_lines_and_collapses_progress_bars() {
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
        sink(out(ofl, Output, Info, "Writing: [==>     ] 10.00%"));
        sink(out(ofl, Output, Info, "Writing: [=====>  ] 60.00%"));
        sink(out(ofl, Output, Info, "Writing: [========] 100.00% Done"));
        sink(out(ofl, Output, Info, "Verifying: [==>     ] 10.00%"));
        sink(out(ofl, Output, Info, "Verifying: [========] 100.00%"));
        console.flush();
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
                "\x1b[31mmain.v:25: ERROR: syntax error\x1b[0m",
                "\x1b[33ma warning\x1b[0m",
            ]
        );
    }

    /// Verify and Upload against a real engine, on a copy of Apio's
    /// alhambra-ii blinky (no board needed: Upload fails with
    /// `connect_board` unless one is plugged in):
    /// `LOGIX_TEST_ENGINE_EXE=<logix-engine> LOGIX_TEST_ENGINE_FOLDER=<engine folder with packages> LOGIX_TEST_EXAMPLE=<alhambra-ii/blinky> cargo test real_engine_build -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_engine_build() {
        let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
        let engine = Engine::start(
            Path::new(&var("LOGIX_TEST_ENGINE_EXE")),
            Path::new(&var("LOGIX_TEST_ENGINE_FOLDER")),
        )
        .await
        .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("build");
        std::fs::create_dir_all(&dir).unwrap();
        let mut files = Vec::new();
        for entry in std::fs::read_dir(var("LOGIX_TEST_EXAMPLE")).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_file() && name != "apio.ini" {
                std::fs::copy(entry.path(), dir.join(&name)).unwrap();
                files.push(name);
            }
        }
        let constraint = crate::build_dir::constraint_file_name(&files).map(str::to_owned);
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let events = events.clone();
            move |e: TaskEvent| events.lock().unwrap().push(e)
        };
        let mut input = BuildInput {
            board: "alhambra-ii",
            arch: "ice40",
            source_folder: &dir,
            files: &files,
            constraint_file: constraint.as_deref(),
            lint: true,
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
        let upload = run(&engine, Task::Upload, &input, sink, &cancel)
            .await
            .unwrap();
        println!("{upload:?}");
        assert!(upload.ok || upload.action.as_deref() == Some("connect_board"));
    }
}
