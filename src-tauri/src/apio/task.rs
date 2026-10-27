//! Running a build or upload in a prepared project folder.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::diagnostics::{parse_line, Diagnostic};
use super::process::strip_ansi;
use super::report::BuildReport;
use super::{args, Apio};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Task {
    /// `apio build`: synthesize, place and route.
    Build,
    /// `apio upload`: build if needed, then program the board.
    Upload,
    /// `apio test [testbench]`: run one or all testbenches.
    Test,
    /// `apio sim <testbench> --detach`: simulate and open GTKWave.
    Simulate,
    /// `apio graph --no-viewer [-t module]`: draw the design as an SVG.
    Graph,
}

impl Task {
    /// `target` is the testbench for `Test`/`Simulate`, or the module to
    /// draw for `Graph`.
    fn args(self, target: Option<&str>) -> Vec<String> {
        let mut args: Vec<String> = match self {
            Task::Build => vec!["build".into()],
            Task::Upload => vec!["upload".into()],
            Task::Test => vec!["test".into()],
            Task::Simulate => vec!["sim".into()],
            Task::Graph => vec!["graph".into(), "--no-viewer".into()],
        };
        if matches!(self, Task::Test | Task::Simulate) {
            args.extend(target.map(str::to_owned));
        }
        if let (Task::Graph, Some(module)) = (self, target) {
            args.extend(["-t".into(), module.to_owned()]);
        }
        if self == Task::Simulate {
            // Return once GTKWave is up instead of waiting for it to close.
            args.push("--detach".into());
        }
        args
    }
}

/// A testbench run by `apio test`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestbenchResult {
    pub name: String,
    pub passed: bool,
}

/// `apio test` prints `Testbench <file>` before running each testbench.
static TESTBENCH_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Testbench (?P<name>\S+)\s*$").unwrap());

/// Results per testbench from the order of the headers: Apio stops at the
/// first failure, so on failure the last one started is the one that
/// failed and all before it passed.
fn testbench_results(started: Vec<String>, ok: bool) -> Vec<TestbenchResult> {
    let last = started.len().saturating_sub(1);
    started
        .into_iter()
        .enumerate()
        .map(|(i, name)| TestbenchResult {
            name,
            passed: ok || i < last,
        })
        .collect()
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
}

impl Apio {
    /// Runs `task` in `dir`, passing every output line to `on_line`.
    /// `testbench` selects one testbench for `Test` (all if `None`) and
    /// `Simulate`. `project_files` and `constraint_file` let errors be
    /// matched to the project's files.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_task<F>(
        &self,
        task: Task,
        testbench: Option<&str>,
        lint: bool,
        dir: &Path,
        project_files: Vec<String>,
        constraint_file: Option<String>,
        on_line: F,
        cancel: &CancellationToken,
    ) -> Result<TaskOutcome>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        let started = Instant::now();
        let found = Arc::new(Mutex::new(Vec::new()));
        let benches = Arc::new(Mutex::new(Vec::new()));
        let (sink, bench_sink) = (found.clone(), benches.clone());
        let handle_line = move |line: String| {
            // The console gets the colored line; parsing uses plain text.
            let plain = strip_ansi(&line);
            if let Some(d) = parse_line(&plain, &project_files, constraint_file.as_deref()) {
                sink.lock().unwrap().push(d);
            }
            if let Some(caps) = TESTBENCH_HEADER.captures(plain.trim_end()) {
                bench_sink.lock().unwrap().push(caps["name"].to_string());
            }
            on_line(line);
        };
        // Verify also lints, for warnings and clearer errors than the
        // build gives. Lint exits with a failure even for warnings, so its
        // result doesn't stop the build; only the build decides success.
        // (Unless turned off in Settings.)
        if task == Task::Build && lint {
            self.run_cancellable(&args(["lint"]), Some(dir), handle_line.clone(), cancel)
                .await?;
        }
        // Upload shows the usage tables before programming the board, like
        // Arduino's "Sketch uses ..." before uploading: `apio report` builds
        // the design (synthesis, place and route) and prints the tables, and
        // `apio upload` reuses that. If it fails (e.g. a syntax error), the
        // upload isn't attempted.
        let mut ok = true;
        let mut tables_shown = false;
        if task == Task::Upload && !cancel.is_cancelled() {
            ok = self
                .run_cancellable(&args(["report"]), Some(dir), handle_line.clone(), cancel)
                .await?
                .success();
            tables_shown = ok;
        }
        // Once stopped, the remaining steps are skipped.
        ok = ok
            && !cancel.is_cancelled()
            && self
                .run_cancellable(
                    &args(task.args(testbench)),
                    Some(dir),
                    handle_line.clone(),
                    cancel,
                )
                .await?
                .success()
            && !cancel.is_cancelled();
        let report = if ok && matches!(task, Task::Build | Task::Upload) {
            // Verify shows the usage tables after the build (best effort: a
            // failed report doesn't fail the build).
            if !tables_shown {
                let _ = self
                    .run_cancellable(&args(["report"]), Some(dir), handle_line, cancel)
                    .await;
            }
            // The same data as JSON, silently, for the status bar summary.
            self.build_report(dir).await.ok()
        } else {
            None
        };
        let graph_svg = if ok && task == Task::Graph {
            let path = dir.join("_build").join("default").join("graph.svg");
            Some(
                tokio::fs::read_to_string(&path)
                    .await
                    .with_context(|| format!("can't read {}", path.display()))?,
            )
        } else {
            None
        };
        let diagnostics = dedup(std::mem::take(&mut *found.lock().unwrap()));
        let started_benches = std::mem::take(&mut *benches.lock().unwrap());
        Ok(TaskOutcome {
            ok,
            diagnostics,
            testbenches: if task == Task::Test {
                testbench_results(started_benches, ok)
            } else {
                Vec::new()
            },
            report,
            cancelled: cancel.is_cancelled(),
            graph_svg,
            seconds: started.elapsed().as_secs_f64(),
        })
    }
}

/// Drops repeated reports of the same problem (e.g. lint and the build
/// both flagging one line), keeping the first.
fn dedup(diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
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

/// The result of formatting one file.
#[derive(Debug, Clone)]
pub struct FormatOutcome {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
}

impl Apio {
    /// Runs `apio format <file>` in `dir` (a project prepared with
    /// `build_dir::prepare_single`), passing Apio's output to `on_line`.
    /// The file is formatted in place.
    pub async fn format_file<F>(&self, dir: &Path, file: &str, on_line: F) -> Result<FormatOutcome>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        let found = Arc::new(Mutex::new(Vec::new()));
        let sink = found.clone();
        let files = vec![file.to_string()];
        let handle_line = move |line: String| {
            if let Some(d) = parse_line(&strip_ansi(&line), &files, None) {
                sink.lock().unwrap().push(d);
            }
            on_line(line);
        };
        let status = self
            .run(&args(["format", file]), Some(dir), handle_line)
            .await?;
        let diagnostics = std::mem::take(&mut *found.lock().unwrap());
        Ok(FormatOutcome {
            ok: status.success(),
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_args() {
        assert_eq!(Task::Build.args(Some("x_tb.v")), ["build"]);
        assert_eq!(Task::Test.args(None), ["test"]);
        assert_eq!(Task::Test.args(Some("main_tb.v")), ["test", "main_tb.v"]);
        assert_eq!(
            Task::Simulate.args(Some("main_tb.v")),
            ["sim", "main_tb.v", "--detach"]
        );
        assert_eq!(Task::Graph.args(None), ["graph", "--no-viewer"]);
        assert_eq!(
            Task::Graph.args(Some("counter")),
            ["graph", "--no-viewer", "-t", "counter"]
        );
    }

    #[test]
    fn dedups_repeated_problems() {
        use super::super::diagnostics::Severity;
        let d = |file: Option<&str>, line: Option<u32>, msg: &str| Diagnostic {
            severity: Severity::Error,
            file: file.map(Into::into),
            line,
            column: None,
            message: msg.into(),
        };
        let kept = dedup(vec![
            d(Some("main.v"), Some(7), "from lint"),
            d(Some("main.v"), Some(7), "from build"),
            d(Some("main.v"), Some(8), "other line"),
            d(None, None, "x"),
            d(None, None, "x"),
            d(None, None, "y"),
        ]);
        let msgs: Vec<_> = kept.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(msgs, ["from lint", "other line", "x", "y"]);
    }

    #[test]
    fn testbench_results_from_headers() {
        let names = || vec!["a_tb.v".to_string(), "b_tb.v".to_string()];
        let passed = |r: Vec<TestbenchResult>| r.iter().map(|t| t.passed).collect::<Vec<_>>();
        assert_eq!(passed(testbench_results(names(), true)), [true, true]);
        assert_eq!(passed(testbench_results(names(), false)), [true, false]);
        assert!(testbench_results(vec![], false).is_empty());
        assert!(TESTBENCH_HEADER.is_match("Testbench main_tb.v"));
        assert!(!TESTBENCH_HEADER.is_match("Testbench file names must end with '_tb.v'."));
    }
    use crate::build_dir::{self, BuildInput};
    use crate::project::{self, ProjectFile};
    use std::path::PathBuf;

    /// A stand-in for Apio: a shell script that answers the commands Logix
    /// runs like Apio would, so the whole build/test path is tested without
    /// the real tools. A build fails if main.v contains "oops".
    const FAKE_APIO: &str = r#"#!/bin/sh
cmd="$1"
bad_line() { grep -n oops main.v | head -1 | cut -d: -f1; }
case "$cmd" in
  lint)
    if grep -q oops main.v; then
      echo "%Error: main.v:$(bad_line):1: syntax error, unexpected oops"; exit 1
    fi
    echo "%Warning-UNUSED: main.v:2:3: Signal is not used: 'x'"; exit 1 ;;
  build)
    if grep -q oops main.v; then
      echo "main.v:$(bad_line): ERROR: syntax error"; exit 2
    fi
    echo "Using env default"; exit 0 ;;
  report)
    if grep -q oops main.v; then
      echo "main.v:$(bad_line): ERROR: syntax error"; exit 2
    fi
    echo "Used FPGA Resource"; exit 0 ;;
  upload) echo "Programming the board"; exit 0 ;;
  api)
    out=""
    while [ $# -gt 0 ]; do [ "$1" = "-o" ] && out="$2"; shift; done
    echo '{"build-report": {"arch": "ice40",
      "resources": {"ICESTORM_LC": {"used": 5, "available": 100}},
      "clocks": {"clk": {"fmax_mhz": 50.0}}}}' > "$out"
    exit 0 ;;
  graph)
    mkdir -p _build/default
    echo '<svg xmlns="http://www.w3.org/2000/svg"><text>fake graph</text></svg>' \
      > _build/default/graph.svg
    exit 0 ;;
  test)
    echo "Testbench main_tb.v"
    echo "*** ASSERTION FAILED at main_tb.v:3: expected 'h1, actual 'h0"
    echo "FATAL: main_tb.v:4: "
    exit 2 ;;
  *) echo "fake apio: unknown command $cmd"; exit 3 ;;
esac
"#;

    #[cfg(unix)]
    #[tokio::test]
    async fn runs_tasks_against_a_fake_apio() {
        use super::super::diagnostics::Severity;
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("apio");
        std::fs::write(&exe, FAKE_APIO).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let apio = Apio {
            exe,
            home: tmp.path().join("home"),
            tmp: tmp.path().join("tmp"),
        };
        let run = |task: Task, main: &str| {
            let files = vec![
                ProjectFile {
                    name: "main.v".into(),
                    text: main.into(),
                },
                ProjectFile {
                    name: "main_tb.v".into(),
                    text: "module main_tb;\nendmodule\n".into(),
                },
                ProjectFile {
                    name: "pinout.pcf".into(),
                    text: "set_io clk 1\n".into(),
                },
            ];
            let root = tmp.path().join("build");
            let dir = build_dir::prepare(
                &root,
                &BuildInput {
                    key: "fake",
                    source_dir: None,
                    files: &files,
                    board: "alhambra-ii",
                },
            )
            .unwrap()
            .dir;
            let names = files.iter().map(|f| f.name.clone()).collect();
            let apio = apio.clone();
            async move {
                let lines = Arc::new(Mutex::new(Vec::<String>::new()));
                let sink = lines.clone();
                let outcome = apio
                    .run_task(
                        task,
                        None,
                        true,
                        &dir,
                        names,
                        Some("pinout.pcf".into()),
                        move |l| sink.lock().unwrap().push(strip_ansi(&l).into_owned()),
                        &CancellationToken::new(),
                    )
                    .await
                    .unwrap();
                let lines = lines.lock().unwrap().clone();
                (outcome, lines)
            }
        };

        // Verify of a good design: lint's warning, success, and the report.
        let (good, lines) = run(Task::Build, "module main;\n  wire x;\nendmodule\n").await;
        assert!(good.ok);
        assert_eq!(good.diagnostics.len(), 1);
        assert_eq!(good.diagnostics[0].severity, Severity::Warning);
        assert_eq!(good.diagnostics[0].line, Some(2));
        let report = good.report.unwrap();
        assert_eq!(report.resources[0].used, 5);
        assert_eq!(report.clocks[0].fmax_mhz, 50.0);
        // Primary commands are shown; the info command isn't.
        let commands: Vec<_> = lines.iter().filter(|l| l.starts_with("$ ")).collect();
        assert_eq!(commands, ["$ apio lint", "$ apio build", "$ apio report"]);

        // Verify of a broken design: lint and build report the same line once.
        let (bad, _) = run(Task::Build, "module main;\noops\nendmodule\n").await;
        assert!(!bad.ok);
        assert!(bad.report.is_none());
        let errors: Vec<_> = bad
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            (errors[0].file.as_deref(), errors[0].line),
            (Some("main.v"), Some(2))
        );

        // Upload: the usage tables come before programming the board.
        let (up, lines) = run(Task::Upload, "module main;\nendmodule\n").await;
        assert!(up.ok);
        assert!(up.report.is_some());
        let commands: Vec<_> = lines.iter().filter(|l| l.starts_with("$ ")).collect();
        assert_eq!(commands, ["$ apio report", "$ apio upload"]);
        // With a syntax error, the upload isn't attempted.
        let (bad_up, lines) = run(Task::Upload, "module main;\noops\nendmodule\n").await;
        assert!(!bad_up.ok);
        assert!(!lines.contains(&"$ apio upload".to_string()));
        assert!(bad_up.diagnostics.iter().any(|d| d.line == Some(2)));

        // The diagram.
        let (graph, lines) = run(Task::Graph, "module main;\nendmodule\n").await;
        assert!(graph.ok);
        assert!(graph.graph_svg.unwrap().contains("fake graph"));
        assert!(lines.contains(&"$ apio graph --no-viewer".to_string()));

        // A failing testbench.
        let (test, _) = run(Task::Test, "module main;\nendmodule\n").await;
        assert!(!test.ok);
        assert_eq!(test.testbenches.len(), 1);
        assert!(!test.testbenches[0].passed);
        assert!(test
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("main_tb.v") && d.line == Some(3)));
    }

    /// Builds a real example with a real Apio, then a broken copy. Run with:
    /// `LOGIX_TEST_APIO_EXE=<apio> LOGIX_TEST_APIO_HOME=<apio home>
    ///  LOGIX_TEST_EXAMPLE=<fetched alhambra-ii/blinky folder>
    ///  cargo test real_apio_build -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn real_apio_build() {
        let var = |k: &str| PathBuf::from(std::env::var(k).unwrap());
        let tmp = tempfile::tempdir().unwrap();
        let apio = Apio {
            exe: var("LOGIX_TEST_APIO_EXE"),
            home: var("LOGIX_TEST_APIO_HOME"),
            tmp: tmp.path().join("tmp"),
        };
        let example = project::load(&var("LOGIX_TEST_EXAMPLE")).unwrap();
        let run = |task: Task, tb: Option<&'static str>, files: Vec<ProjectFile>| {
            let root = tmp.path().join("build");
            let dir = build_dir::prepare(
                &root,
                &BuildInput {
                    key: "test",
                    source_dir: None,
                    files: &files,
                    board: "alhambra-ii",
                },
            )
            .unwrap()
            .dir;
            let names = files.iter().map(|f| f.name.clone()).collect();
            let pcf = build_dir::constraint_file(&files).map(str::to_owned);
            let apio = apio.clone();
            async move {
                apio.run_task(
                    task,
                    tb,
                    true,
                    &dir,
                    names,
                    pcf,
                    |_| {},
                    &CancellationToken::new(),
                )
                .await
                .unwrap()
            }
        };
        let build = |files: Vec<ProjectFile>| run(Task::Build, None, files);

        let good = build(example.files.clone()).await;
        println!("good build: ok={} in {:.1}s", good.ok, good.seconds);
        assert!(good.ok, "{:?}", good.diagnostics);
        let report = good.report.expect("a build report");
        println!("report: arch={} clocks={:?}", report.arch, report.clocks);
        assert!(report
            .resources
            .iter()
            .any(|r| r.name == "ICESTORM_LC" && r.used > 0));

        // A width mismatch: lint warns, but the design still builds.
        let mut warned = example.files.clone();
        warned[0].text =
            warned[0]
                .text
                .replacen("endmodule", "  wire [7:0] wide = 4'd3;\nendmodule", 1);
        let w = build(warned).await;
        println!("lint warning build: ok={} {:?}", w.ok, w.diagnostics);
        assert!(w.ok);
        assert!(w.diagnostics.iter().any(|d| {
            d.severity == super::super::diagnostics::Severity::Warning
                && d.file.as_deref() == Some("main.v")
                && d.line.is_some()
        }));

        let mut broken = example.files.clone();
        broken[0].text = broken[0].text.replace("endmodule", "oops\nendmodule");
        let bad = build(broken).await;
        println!("broken build: ok={} {:?}", bad.ok, bad.diagnostics);
        assert!(!bad.ok);
        assert!(bad
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("main.v") && d.line.is_some()));

        let graph = run(Task::Graph, None, example.files.clone()).await;
        let svg = graph.graph_svg.expect("a diagram");
        println!("graph: ok={} svg={} bytes", graph.ok, svg.len());
        assert!(svg.contains("<svg"));

        let test = run(Task::Test, None, example.files.clone()).await;
        println!("test: ok={} {:?}", test.ok, test.testbenches);
        assert!(test.ok);
        assert_eq!(test.testbenches.len(), 1);
        assert!(test.testbenches[0].passed);

        // Make the testbench's first check fail.
        let mut failing = example.files.clone();
        let tb = failing.iter_mut().find(|f| f.name == "main_tb.v").unwrap();
        tb.text = tb
            .text
            .replacen("`EXPECT_EQ(leds, 8'b0)", "`EXPECT_EQ(leds, 8'b1)", 1);
        let failed = run(Task::Test, Some("main_tb.v"), failing).await;
        println!("failing test: ok={} {:?}", failed.ok, failed.diagnostics);
        assert!(!failed.ok);
        assert!(!failed.testbenches[0].passed);
        assert!(failed
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("main_tb.v") && d.message.starts_with("Assertion")));

        // Auto Format: a messy file gets formatted, a broken one reports
        // where the problem is.
        let format = |name: &str, text: &str| {
            let root = tmp.path().join("format");
            let file = ProjectFile {
                name: name.into(),
                text: text.into(),
            };
            let dir = build_dir::prepare_single(&root, &file, "alhambra-ii").unwrap();
            let apio = apio.clone();
            let name = name.to_string();
            async move {
                let outcome = apio.format_file(&dir, &name, |_| {}).await.unwrap();
                let text = std::fs::read_to_string(dir.join(&name)).unwrap();
                (outcome, text)
            }
        };
        let (ok, text) = format(
            "alu.v",
            "module alu(input a,output b);assign b=a;\n  endmodule\n",
        )
        .await;
        println!("format: ok={} text={text:?}", ok.ok);
        assert!(ok.ok);
        assert!(text.contains("assign b = a;"));
        let (bad, _) = format("bad.v", "module x;\ninitial begin\nend end\nendmodule\n").await;
        println!("format broken: ok={} {:?}", bad.ok, bad.diagnostics);
        assert!(!bad.ok);
        assert!(bad
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("bad.v") && d.line == Some(3)));
    }
}
