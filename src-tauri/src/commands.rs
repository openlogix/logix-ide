//! Tauri commands: the backend side of the UI<->backend interface.
//! Each command here has a typed wrapper in `src/lib/backend.ts`.

use std::path::Path;
use std::sync::atomic::Ordering;

use anyhow::{anyhow, Context};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};

use crate::engine;
use crate::examples;
use crate::outcome;
use crate::project::{self, ProjectFile};
use crate::setup_events::{SetupEvent, SetupStage};
use crate::{bare, build_dir, platform, settings, AppState, Catalog};

/// An error for the UI: `message` is short plain language for the user,
/// `details` is the full technical chain for the console.
#[derive(Debug, Serialize)]
pub struct UserError {
    message: String,
    details: String,
}

impl From<anyhow::Error> for UserError {
    fn from(e: anyhow::Error) -> Self {
        // Every error the user sees is in the log too, with its details.
        crate::log!("Error: {e:#}");
        UserError {
            message: e.to_string(),
            details: format!("{e:#}"),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: String,
    version: String,
    platform: String,
    platform_supported: bool,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle) -> AppInfo {
    let pkg = app.package_info();
    let current = platform::current();
    AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        platform: current.map_or("unknown", |p| p.id()).to_string(),
        platform_supported: current.is_some_and(|p| p.is_implemented()),
    }
}

/// Makes sure Logix Engine and its packages (the FPGA tools) are
/// installed and starts it, streaming progress to `on_event`. Safe to call
/// again (e.g. to retry after an error).
#[tauri::command]
pub async fn setup(
    state: State<'_, AppState>,
    on_event: Channel<SetupEvent>,
) -> Result<(), UserError> {
    let _ = on_event.send(SetupEvent::Stage {
        stage: SetupStage::Checking,
    });
    let mut engine_slot = state.engine.lock().await;
    if engine_slot.is_none() {
        let channel = on_event.clone();
        let result = engine::run_setup(move |event| {
            let _ = channel.send(event);
        })
        .await;
        match result {
            Ok(ready) => {
                crate::log!("Engine setup ready");
                *engine_slot = Some(std::sync::Arc::new(ready));
            }
            Err(e) => {
                crate::log!("Engine setup failed: {e:#}");
                return Err(e.into());
            }
        }
    }
    let _ = on_event.send(SetupEvent::Stage {
        stage: SetupStage::Ready,
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Projects and the document window
// ---------------------------------------------------------------------------

/// A project as the UI sees it. `dir` is `None` for a new, unsaved project.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDoc {
    dir: Option<String>,
    /// For an unsaved project opened from an example: the example's files,
    /// copied along on the first save.
    template_dir: Option<String>,
    name: String,
    /// `main.v` first.
    files: Vec<ProjectFile>,
    board: Option<String>,
    /// The board's variant (empty: the board's default).
    variant: project::Variant,
}

impl From<project::Project> for ProjectDoc {
    fn from(p: project::Project) -> Self {
        ProjectDoc {
            dir: Some(p.dir.to_string_lossy().into_owned()),
            template_dir: None,
            name: p.name,
            files: p.files,
            board: p.board,
            variant: p.variant,
        }
    }
}

// ---------------------------------------------------------------------------
// Boards and examples
// ---------------------------------------------------------------------------

/// Sends tool output that isn't part of a task or setup (which stream
/// through their own channel) to the console, as `tool-output` events:
/// Auto Format's.
fn console_log(app: &AppHandle) -> impl Fn(String) + Send + Sync + Clone + 'static {
    let app = app.clone();
    move |line| {
        let _ = app.emit("tool-output", line);
    }
}

/// The running engine, or a user-facing error if setup hasn't finished.
async fn ready_engine(state: &AppState) -> Result<std::sync::Arc<engine::Engine>, UserError> {
    state
        .engine
        .lock()
        .await
        .clone()
        .ok_or_else(|| anyhow!("The FPGA tools aren't ready yet.").into())
}

/// The board and example lists (fetched on first use): the engine's
/// boards, and Logix's examples for those boards.
async fn catalog(state: &AppState) -> Result<Catalog, UserError> {
    let mut slot = state.catalog.lock().await;
    if let Some(c) = slot.as_ref() {
        return Ok(c.clone());
    }
    let engine = ready_engine(state).await?;
    let engine::Catalog { boards } = engine::catalog(&engine)
        .await
        .context("Couldn't get the list of boards.")?;
    // Examples for boards the engine doesn't support can't be built.
    let all = examples::all();
    let total = all.len();
    let list: Vec<_> = all
        .into_iter()
        .filter(|e| boards.iter().any(|b| b.id == e.board))
        .collect();
    let examples = examples::ExampleList {
        hidden: total - list.len(),
        examples: list,
    };
    let c = Catalog { boards, examples };
    *slot = Some(c.clone());
    Ok(c)
}

#[tauri::command]
pub async fn get_boards(state: State<'_, AppState>) -> Result<Vec<engine::Board>, UserError> {
    Ok(catalog(&state).await?.boards)
}

/// Every board the project knows, as Logix Engine merges them (the
/// catalog's with its custom-boards.json's, marked `custom`), or the
/// problems it found in the file. Empty without the file.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBoards {
    boards: Vec<engine::Board>,
    problem: Option<engine::BoardsProblem>,
}

/// The custom boards file of the project in `folder` (its folder, or the
/// example a new project starts from): the file on disk, which the user
/// edits outside Logix; Logix Engine reads it and reports its problems at
/// line and column, named as the user knows it (`engine::BoardsFile`: its
/// full path when the project is `saved`, `custom-boards.json` when it
/// isn't). None without a folder or without the file.
fn boards_file(folder: Option<&str>, saved: bool) -> Option<engine::BoardsFile> {
    project::custom_boards_file(Path::new(folder?)).map(|f| engine::BoardsFile::new(f, saved))
}

/// A value that changes whenever the project's custom-boards.json is
/// added, changed or removed (null without it), polled by the UI to reload
/// the board list after edits made outside Logix.
#[tauri::command]
pub fn custom_boards_stamp(folder: Option<String>) -> Option<String> {
    project::custom_boards_stamp(Path::new(&folder?))
}

/// The project's own boards from its `custom-boards.json` (in `folder`),
/// as Logix Engine reads them, for the board list, or the problems the
/// engine found in it.
#[tauri::command]
pub async fn project_boards(
    state: State<'_, AppState>,
    folder: Option<String>,
    saved: bool,
) -> Result<ProjectBoards, UserError> {
    let engine = ready_engine(&state).await?;
    let file = boards_file(folder.as_deref(), saved);
    let found = engine::project_boards(&engine, file.as_ref())
        .await
        .context("Couldn't read the project's custom boards.")?;
    Ok(match found {
        Ok(boards) => ProjectBoards {
            boards,
            problem: None,
        },
        Err(problem) => ProjectBoards {
            boards: Vec::new(),
            problem: Some(problem),
        },
    })
}

/// What differs between boards `from` and `to` (Logix Engine's
/// `compare_boards`), the project's own boards included (its
/// custom-boards.json in `folder`), for the note on changing a project's
/// board.
#[tauri::command]
pub async fn compare_boards(
    state: State<'_, AppState>,
    from: String,
    to: String,
    folder: Option<String>,
    saved: bool,
) -> Result<engine::BoardComparison, UserError> {
    let engine = ready_engine(&state).await?;
    let file = boards_file(folder.as_deref(), saved);
    Ok(engine::compare_boards(&engine, &from, &to, file.as_ref())
        .await
        .context("Couldn't compare the boards.")?)
}

/// The examples Logix supports, and how many unsupported ones were hidden.
#[tauri::command]
pub async fn get_examples(state: State<'_, AppState>) -> Result<examples::ExampleList, UserError> {
    Ok(catalog(&state).await?.examples)
}

/// Opens an example as a new, unsaved project set to the example's board.
#[tauri::command]
pub async fn open_example(board: String, name: String) -> Result<ProjectDoc, UserError> {
    example_doc(board, &name, name.clone()).await
}

/// An example as a new, unsaved project named `doc_name`.
async fn example_doc(
    board: String,
    example: &str,
    doc_name: String,
) -> Result<ProjectDoc, UserError> {
    // Written out to a scratch folder, replaced by the next example: the
    // project's files are loaded from there, and its other files (e.g. a
    // GTKWave layout) are copied from there on the first save.
    let dst = platform::current_implemented()?.data_dir()?.join("example");
    let (board_id, name) = (board.clone(), example.to_string());
    let loaded = blocking(move || {
        let dir = examples::write(&board_id, &name, &dst).context("Couldn't open the example.")?;
        project::load(&dir)
    })
    .await??;
    Ok(ProjectDoc {
        dir: None,
        template_dir: Some(loaded.dir.to_string_lossy().into_owned()),
        name: doc_name,
        files: loaded.files,
        board: Some(board),
        variant: project::Variant::new(),
    })
}

// ---------------------------------------------------------------------------
// Build and upload
// ---------------------------------------------------------------------------

/// Runs Verify, Upload, Run Tests, Simulate or Show Diagram with Logix
/// Engine on the project's current (possibly unsaved) files in the build
/// folder, streaming the tools' output and the phases to `on_event`.
/// `key` identifies the project (so builds of the same project reuse the
/// engine's output); `source_dir` is the folder whose other files (e.g.
/// memory init data) are copied along.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn run_task(
    app: AppHandle,
    state: State<'_, AppState>,
    task: outcome::Task,
    testbench: Option<String>,
    key: String,
    source_dir: Option<String>,
    project_dir: Option<String>,
    files: Vec<ProjectFile>,
    board: String,
    variant: engine::Variant,
    device: Option<String>,
    on_event: Channel<outcome::TaskEvent>,
) -> Result<outcome::TaskOutcome, UserError> {
    let Ok(_running) = state.task.try_lock() else {
        return Err(anyhow!("A build or upload is already running.").into());
    };
    let engine = ready_engine(&state).await?;
    // The boards the project knows: with a custom-boards.json (the file in
    // the project folder, or the example a new project starts from), the
    // engine's list with its boards merged in (one with a catalog board's
    // ID replaces it); else the catalog's.
    let boards_file = boards_file(source_dir.as_deref(), project_dir.is_some());
    let known = match engine::project_boards(&engine, boards_file.as_ref())
        .await
        .context("Couldn't read the project's custom boards.")?
    {
        Ok(merged) if boards_file.is_some() => merged,
        Ok(_) => catalog(&state).await?.boards,
        // Problems in the file stop every task, as in the engine, whatever
        // the board.
        Err(problem) => {
            for line in problem.console_lines() {
                (on_event.clone()).send(outcome::TaskEvent::line(line)).ok();
            }
            return Ok(engine::boards_problem_outcome(&problem));
        }
    };
    let Some(info) = known.iter().find(|b| b.id == board) else {
        return Err(anyhow!(
            "Logix doesn't support the board \"{board}\" yet. Choose another board in the toolbar."
        )
        .into());
    };
    let arch = info.arch.clone();
    let pin_extension = info.pin_extension.clone();
    // The values the user chose, as they are: the engine uses the board's
    // first variant with them ({} for its default).
    let board_variant = variant.clone();
    let on_line = {
        let channel = on_event.clone();
        move |line: String| {
            let _ = channel.send(outcome::TaskEvent::line(line));
        }
    };
    let root = platform::current_implemented()?.data_dir()?.join("build");
    let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    // `testbench` is the testbench for Test/Simulate, or the module to draw
    // for Graph. Only accept project testbenches and plain Verilog module
    // names.
    if let Some(target) = &testbench {
        let ok = if task == outcome::Task::Graph {
            project::is_verilog_identifier(target)
        } else {
            project::is_testbench(target) && names.contains(target)
        };
        if !ok {
            return Err(anyhow!("\"{target}\" can't be used here.").into());
        }
    }
    let constraint =
        build_dir::constraint_file(&files, pin_extension.as_deref()).map(str::to_owned);
    let log = on_line;
    let dir = blocking(move || {
        // Keep GTKWave layouts saved since the last run; preparing clears
        // the build folder.
        if let Some(project_dir) = &project_dir {
            for name in save_back_gtkw(&root, &key, Path::new(project_dir)) {
                log(gtkw_saved_message(&name));
            }
        }
        let prepared = build_dir::prepare(
            &root,
            &build_dir::BuildInput {
                key: &key,
                source_dir: source_dir.as_deref().map(Path::new),
                files: &files,
            },
        )
        .context("Couldn't prepare the build.")?;
        Ok::<_, anyhow::Error>(prepared)
    })
    .await??;
    // GTKWave windows open before the simulation, to recognize the new one.
    let gtkwave_before = if task == outcome::Task::Simulate {
        on_main_thread(&app, || platform::running_app_pids(GTKWAVE))
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    *state.task_cancel.lock().unwrap() = Some(cancel.clone());
    if info.custom {
        crate::log!(
            "Task {task:?}: board {board} from {}",
            boards_file
                .as_ref()
                .map_or("?".into(), |f| f.path.display().to_string())
        );
    }
    let input = engine::TaskInput {
        board: &board,
        board_variant: Some(&board_variant),
        custom_boards: boards_file.as_ref(),
        arch: &arch,
        source_folder: &dir,
        files: &names,
        constraint_file: constraint.as_deref(),
        lint: lint_on_verify(),
        target: testbench.as_deref(),
        device: device.as_deref(),
    };
    let channel = on_event.clone();
    let send = move |event| {
        let _ = channel.send(event);
    };
    let result = engine::run_task(&engine, task, &input, send, &cancel)
        .await
        .context("Couldn't run Logix Engine.");
    *state.task_cancel.lock().unwrap() = None;
    let outcome = result?;
    // The engine opens GTKWave whenever the testbench ran, even failing
    // (its waveform shows where).
    if task == outcome::Task::Simulate && !outcome.testbenches.is_empty() {
        let generation = GTKWAVE_WAIT.fetch_add(1, Ordering::SeqCst) + 1;
        tauri::async_runtime::spawn(bring_gtkwave_to_front(app, gtkwave_before, generation));
    }
    Ok(outcome)
}

/// How GTKWave's process is named.
const GTKWAVE: &str = "gtkwave";

/// The latest simulation's wait for GTKWave: an older wait that ends after
/// a newer simulation started says nothing.
static GTKWAVE_WAIT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How long to wait for GTKWave's window: a cold start (the first after a
/// while) can take over 8 seconds.
const GTKWAVE_WAIT_SECONDS: u64 = 30;

/// After a simulation, GTKWave starts in the background and its
/// window would stay behind Logix. Waits for the new GTKWave (up to
/// `GTKWAVE_WAIT_SECONDS`) and brings it to the front, then tells the UI
/// with the `gtkwave-opened` event (true when found), which ends its
/// "Opening GTKWave…" status.
async fn bring_gtkwave_to_front(app: AppHandle, before: Vec<i32>, generation: u64) {
    use std::time::Duration;
    let tell = |found: bool| {
        if GTKWAVE_WAIT.load(Ordering::SeqCst) == generation {
            let _ = app.emit("gtkwave-opened", found);
        }
    };
    for _ in 0..GTKWAVE_WAIT_SECONDS * 4 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let pids = on_main_thread(&app, || platform::running_app_pids(GTKWAVE))
            .await
            .unwrap_or_default();
        let Some(pid) = pids.into_iter().filter(|p| !before.contains(p)).max() else {
            continue;
        };
        on_main_thread(&app, move || platform::bring_app_to_front(pid)).await;
        // Once more after its window has had time to appear.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let moved = on_main_thread(&app, move || platform::bring_app_to_front(pid)).await;
        crate::log!("Brought GTKWave (pid {pid}) to the front: {moved:?}");
        tell(true);
        return;
    }
    crate::log!("Didn't find a new GTKWave window to bring to the front");
    tell(false);
}

/// Runs `f` on the main thread (needed for native window APIs) and returns
/// its result, or `None` if that isn't possible.
async fn on_main_thread<T, F>(app: &AppHandle, f: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .ok()?;
    rx.await.ok()
}

// ---------------------------------------------------------------------------
// Language server (live errors while typing)
// ---------------------------------------------------------------------------

/// Starts the Verilog language server for the UI's new client. Its
/// messages go to the UI as `lsp-message` events (JSON text);
/// `lsp-closed` tells the UI it stopped. One already running (the window
/// was reloaded, so its client is gone) is replaced: the new client opens
/// the project's files again, and Verible aborts when a file it already
/// has open is opened again and the files then change
/// (`StringViewSuperRangeMap::must_emplace`).
#[tauri::command]
pub async fn lsp_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), UserError> {
    static STARTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut slot = state.lsp.lock().await;
    if slot.take().is_some() {
        crate::log!("Language server replaced (a new window)");
    }
    let number = STARTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let engine = ready_engine(&state).await?;
    let exe = engine::program_path(&engine, "verible", "verible-verilog-ls")
        .await
        .context("Couldn't find the Verilog language server.")?;
    let on_message = {
        let app = app.clone();
        move |message: String| {
            let _ = app.emit("lsp-message", message);
        }
    };
    let on_exit = {
        let app = app.clone();
        move || {
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let mut slot = state.lsp.lock().await;
                // Only this server's exit: a replaced one is already gone.
                if slot.as_ref().is_some_and(|(n, _)| *n == number) {
                    slot.take();
                    crate::log!("Language server stopped by itself");
                    let _ = app.emit("lsp-closed", ());
                }
            });
        }
    };
    let server = crate::language_server::LanguageServer::start(&exe, on_message, on_exit)
        .context("Couldn't start the Verilog language server.")?;
    crate::log!("Language server started: {}", exe.display());
    *slot = Some((number, server));
    Ok(())
}

/// Stops the language server (Settings: live checking turned off).
#[tauri::command]
pub async fn lsp_stop(state: State<'_, AppState>) -> Result<(), UserError> {
    // Dropping it stops the process.
    if state.lsp.lock().await.take().is_some() {
        crate::log!("Language server stopped (Settings)");
    }
    Ok(())
}

/// Sends one message (JSON text) to the language server.
#[tauri::command]
pub async fn lsp_send(state: State<'_, AppState>, message: String) -> Result<(), UserError> {
    let mut slot = state.lsp.lock().await;
    let (_, server) = slot
        .as_mut()
        .ok_or_else(|| anyhow!("The Verilog language server isn't running."))?;
    server
        .send(&message)
        .await
        .context("Couldn't talk to the Verilog language server.")?;
    Ok(())
}

/// The connected USB devices.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Devices {
    usb: Vec<engine::UsbDevice>,
    /// Devices that may be boards but can't be opened yet (a driver or
    /// permissions), with the fix.
    unreachable: Vec<engine::UnreachableDevice>,
}

/// The connected USB devices, for Tools > Connected Devices and hints when
/// Upload can't find the board.
#[tauri::command]
pub async fn scan_devices(
    state: State<'_, AppState>,
    folder: Option<String>,
    saved: bool,
) -> Result<Devices, UserError> {
    let engine = ready_engine(&state).await?;
    let file = boards_file(folder.as_deref(), saved);
    // A custom boards file with problems: the devices without its boards
    // (the board list shows its problems).
    let list = match engine::list_devices(&engine, file.as_ref()).await {
        Ok(list) => Ok(list),
        Err(_) if file.is_some() => engine::list_devices(&engine, None).await,
        Err(e) => Err(e),
    }
    .context("Couldn't list the connected devices.")?;
    Ok(Devices {
        usb: list.devices,
        unreachable: list.unreachable,
    })
}

/// The connected USB devices that fit catalog boards, each with the boards
/// it may be, for the board list's device groups. Silent; polled by the UI
/// while it's idle.
#[tauri::command]
pub async fn connected_devices(
    state: State<'_, AppState>,
    folder: Option<String>,
    saved: bool,
) -> Result<Vec<engine::ConnectedDevice>, UserError> {
    let engine = ready_engine(&state).await?;
    let file = boards_file(folder.as_deref(), saved);
    // The project's boards first, as the engine matched them; a file with
    // problems counts as none here (the board list shows its problems).
    let usable = engine::project_boards(&engine, file.as_ref())
        .await
        .ok()
        .and_then(Result::ok)
        .is_some();
    let file = if usable { file } else { None };
    let list = engine::list_devices(&engine, file.as_ref())
        .await
        .context("Couldn't list the connected devices.")?;
    let found = engine::connected_devices(&list.devices);
    // Polled every few seconds: log only changes, not every scan.
    static LAST: std::sync::Mutex<Option<Vec<engine::ConnectedDevice>>> =
        std::sync::Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_ref() != Some(&found) {
        let names: Vec<String> = found
            .iter()
            .map(|d| {
                let boards: Vec<_> = d.boards.iter().map(|c| c.board.as_str()).collect();
                format!("{}: {}", d.name, boards.join(", "))
            })
            .collect();
        crate::log!("Connected devices: {names:?}");
        *last = Some(found.clone());
    }
    Ok(found)
}

/// Copies GTKWave layouts the user saved (from a simulation) back to the
/// project folder `project_dir`, and returns the console lines to show.
/// The UI calls this when the window regains focus, i.e. typically after
/// saving a layout in GTKWave.
#[tauri::command]
pub async fn save_gtkwave_layouts(
    key: String,
    project_dir: String,
) -> Result<Vec<String>, UserError> {
    let root = platform::current_implemented()?.data_dir()?.join("build");
    let names = blocking(move || save_back_gtkw(&root, &key, Path::new(&project_dir))).await?;
    Ok(names.iter().map(|n| gtkw_saved_message(n)).collect())
}

/// Best effort: a failure to save a layout back shouldn't stop a build.
fn save_back_gtkw(root: &Path, key: &str, project_dir: &Path) -> Vec<String> {
    build_dir::save_back_gtkw(root, key, project_dir).unwrap_or_else(|e| {
        crate::log!("Can't save GTKWave layouts: {e:#}");
        Vec::new()
    })
}

fn gtkw_saved_message(name: &str) -> String {
    format!("Saved the GTKWave layout {name} to the project folder.")
}

/// Whether Verify lints (Settings > "Show lint warnings on Verify").
fn lint_on_verify() -> bool {
    match (settings_path(), builtin_projects_dir()) {
        (Ok(path), Ok(builtin)) => settings::load(&path).preferences(&builtin).lint_on_verify,
        _ => true,
    }
}

/// Stops the running build, upload, test or simulation, if any. Returns
/// whether one was running.
#[tauri::command]
pub fn cancel_task(state: State<'_, AppState>) -> bool {
    match state.task_cancel.lock().unwrap().as_ref() {
        Some(cancel) => {
            cancel.cancel();
            true
        }
        None => false,
    }
}

/// The result of Auto Format.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatResult {
    ok: bool,
    /// The formatted text, if formatting succeeded.
    text: Option<String>,
    diagnostics: Vec<outcome::Diagnostic>,
}

/// Formats one file's text with the engine's `format` (Verible) in a
/// scratch folder and returns the result; Verible's output goes to the
/// console. The project folder isn't touched.
#[tauri::command]
pub async fn format_file(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    text: String,
) -> Result<FormatResult, UserError> {
    if !project::is_hdl_source(&name) {
        return Err(anyhow!(
            "Auto Format works on Verilog and SystemVerilog files (.v .sv .vh .svh)."
        )
        .into());
    }
    let _formatting = state.format.lock().await;
    let engine = ready_engine(&state).await?;
    let root = platform::current_implemented()?.data_dir()?.join("format");
    let file = ProjectFile {
        name: name.clone(),
        text,
    };
    let dir = blocking(move || {
        build_dir::prepare_single(&root, &file).context("Couldn't prepare formatting.")
    })
    .await??;
    let outcome = engine::format_file(&engine, &dir, &name, console_log(&app))
        .await
        .context("Couldn't run Logix Engine.")?;
    let text = if outcome.ok {
        let path = dir.join(&name);
        Some(
            tokio::fs::read_to_string(&path)
                .await
                .with_context(|| format!("can't read {}", path.display()))?,
        )
    } else {
        None
    };
    Ok(FormatResult {
        ok: outcome.ok,
        text,
        diagnostics: outcome.diagnostics,
    })
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Settings (File > Settings)
// ---------------------------------------------------------------------------

fn builtin_projects_dir() -> anyhow::Result<std::path::PathBuf> {
    project::builtin_projects_root().context("can't find the Documents folder")
}

/// The current preferences, with defaults filled in.
#[tauri::command]
pub fn get_preferences() -> Result<settings::Preferences, UserError> {
    let path = settings_path()?;
    Ok(settings::load(&path).preferences(&builtin_projects_dir()?))
}

/// Saves the preferences, applies the window ones (scale, theme), and
/// returns them as stored.
#[tauri::command]
pub fn set_preferences(
    window: tauri::WebviewWindow,
    preferences: settings::Preferences,
) -> Result<settings::Preferences, UserError> {
    let path = settings_path()?;
    let builtin = builtin_projects_dir()?;
    let mut stored = settings::load(&path);
    stored.set_preferences(&preferences, &builtin)?;
    settings::save(&path, &stored).context("Couldn't save the settings.")?;
    let prefs = stored.preferences(&builtin);
    apply_window_preferences(&window, &prefs);
    Ok(prefs)
}

/// Applies the preferences that belong to the window: interface scale and
/// theme (light/dark title bar and controls).
pub fn apply_window_preferences(window: &tauri::WebviewWindow, prefs: &settings::Preferences) {
    let _ = window.set_zoom(f64::from(prefs.ui_scale.unwrap_or(100)) / 100.0);
    let _ = window.set_theme(match prefs.theme {
        settings::Theme::System => None,
        settings::Theme::Light => Some(tauri::Theme::Light),
        settings::Theme::Dark => Some(tauri::Theme::Dark),
    });
}

/// At startup: applies the stored window preferences.
pub fn apply_stored_window_preferences(window: &tauri::WebviewWindow) {
    if let (Ok(path), Ok(builtin)) = (settings_path(), builtin_projects_dir()) {
        apply_window_preferences(window, &settings::load(&path).preferences(&builtin));
    }
}

/// Asks for a folder (Settings > Browse). `None` if cancelled.
#[tauri::command]
pub async fn choose_folder_dialog(
    app: AppHandle,
    start: Option<String>,
) -> Result<Option<String>, UserError> {
    let mut dialog = app.dialog().file().set_title("Choose the Projects Folder");
    if let Some(dir) = start.map(std::path::PathBuf::from).filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    let Some(picked) = blocking(move || dialog.blocking_pick_folder()).await? else {
        return Ok(None);
    };
    let dir = picked
        .into_path()
        .map_err(|e| anyhow!("{e}"))
        .context("Couldn't use that folder.")?;
    Ok(Some(dir.to_string_lossy().into_owned()))
}

/// Where IDE settings are kept.
fn settings_path() -> anyhow::Result<std::path::PathBuf> {
    settings::default_path()
}

/// Remembers `dir` as the project to reopen next time and the most recent
/// one (File > Open Recent). Best effort.
fn remember_project(app: &AppHandle, dir: &Path) {
    if let Ok(path) = settings_path() {
        if let Err(e) = settings::remember_project(&path, dir) {
            crate::log!("Can't save settings: {e:#}");
        }
    }
    crate::menu::refresh(app);
}

/// Reopens the project last opened or saved. `None` if there isn't one.
#[tauri::command]
pub async fn open_last_project() -> Result<Option<ProjectDoc>, UserError> {
    let Some(dir) = settings::load(&settings_path()?).last_project else {
        return Ok(None);
    };
    let name = project::project_name(&dir);
    let loaded = blocking(move || {
        project::load(&dir)
            .with_context(|| format!("Couldn't reopen the last project, \"{name}\"."))
    })
    .await??;
    Ok(Some(loaded.into()))
}

/// File > New Project > Empty Project: an unsaved project for `board` (or
/// else the last chosen board, or the default one) with the board's pins
/// and a design that does nothing yet: its Bare Minimum (see
/// `open_bare_minimum`). It builds right away.
#[tauri::command]
pub async fn new_project(board: Option<String>) -> Result<ProjectDoc, UserError> {
    // A new project always has a board: the given one, the last one chosen,
    // or else the default.
    let board = board.unwrap_or_else(last_board);
    open_bare_minimum(board).await
}

/// A board's Bare Minimum project (Examples): its pins and a design that
/// does nothing yet (see `bare.rs`), as a new, unsaved project.
#[tauri::command]
pub async fn open_bare_minimum(board: String) -> Result<ProjectDoc, UserError> {
    let all = examples::all();
    let Some(ex) = examples::starter_example(&all, &board) else {
        return Ok(no_pins_doc(board));
    };
    let name = ex.name.clone();
    let starter = example_doc(board.clone(), &name, "untitled".into()).await?;
    Ok(match bare::bare_files(&starter.files) {
        Some(files) => ProjectDoc {
            dir: None,
            template_dir: None,
            name: "untitled".into(),
            files,
            board: Some(board),
            variant: project::Variant::new(),
        },
        None => no_pins_doc(board),
    })
}

/// A new project for a board without an example to take the pins from.
fn no_pins_doc(board: String) -> ProjectDoc {
    ProjectDoc {
        dir: None,
        template_dir: None,
        name: "untitled".into(),
        files: vec![ProjectFile {
            name: project::MAIN_FILE.into(),
            text: bare::NO_PINS_MAIN.into(),
        }],
        board: Some(board),
        variant: project::Variant::new(),
    }
}

/// The recent project folders, newest first (the start screen's list).
#[tauri::command]
pub fn recent_projects() -> Vec<String> {
    crate::menu::recent_projects()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// The board for new projects: the last one chosen, or the default.
#[tauri::command]
pub fn last_board() -> String {
    settings_path()
        .ok()
        .and_then(|p| settings::load(&p).last_board)
        .unwrap_or_else(|| examples::DEFAULT_BOARD.to_string())
}

/// The variant of the last chosen board (empty: its default).
#[tauri::command]
pub fn last_variant() -> project::Variant {
    settings_path()
        .ok()
        .map(|p| settings::load(&p).last_variant)
        .unwrap_or_default()
}

/// Remembers the board and variant the user chose, for new projects.
#[tauri::command]
pub fn set_last_board(board: String, variant: project::Variant) -> Vec<String> {
    let Ok(path) = settings_path() else {
        return Vec::new();
    };
    match settings::remember_board(&path, &board, &variant) {
        Ok(recent) => recent,
        Err(e) => {
            crate::log!("Can't save settings: {e:#}");
            settings::load(&path).recent_boards
        }
    }
}

/// The boards chosen most recently, newest first (the board picker's
/// "Recently used" group).
#[tauri::command]
pub fn recent_boards() -> Vec<String> {
    settings_path()
        .ok()
        .map(|p| settings::load(&p).recent_boards)
        .unwrap_or_default()
}

/// File > Close Project: the next start shows the start screen instead of
/// reopening the closed project. Best effort.
#[tauri::command]
pub fn close_project() {
    if let Ok(path) = settings_path() {
        if let Err(e) = settings::clear_last_project(&path) {
            crate::log!("Can't save settings: {e:#}");
        }
    }
}

/// Asks the user for a project folder and loads it. `None` if cancelled.
#[tauri::command]
pub async fn open_project_dialog(app: AppHandle) -> Result<Option<ProjectDoc>, UserError> {
    let mut dialog = app.dialog().file().set_title("Open Project");
    if let Some(dir) = project::default_projects_root().filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    let Some(picked) = blocking(move || dialog.blocking_pick_folder()).await? else {
        return Ok(None);
    };
    let dir = picked
        .into_path()
        .map_err(|e| anyhow!("{e}"))
        .context("Couldn't open that folder.")?;
    let loaded = blocking(move || project::load(&dir)).await??;
    remember_project(&app, &loaded.dir);
    Ok(Some(loaded.into()))
}

/// Saves the project in its existing folder. `removed` lists files that were
/// deleted or renamed since the last save.
#[tauri::command]
pub async fn save_project(
    app: AppHandle,
    dir: String,
    files: Vec<ProjectFile>,
    removed: Vec<String>,
    board: Option<String>,
    variant: project::Variant,
) -> Result<(), UserError> {
    let saved_dir = std::path::PathBuf::from(&dir);
    blocking(move || {
        project::save(
            Path::new(&dir),
            &files,
            &removed,
            board.as_deref(),
            &variant,
        )
        .context("Couldn't save the project.")
    })
    .await??;
    remember_project(&app, &saved_dir);
    Ok(())
}

/// Opens the project in `dir` (File > Open Recent). If it can't be
/// opened (e.g. the folder was moved), it's removed from the recent list.
#[tauri::command]
pub async fn open_project_at(app: AppHandle, dir: String) -> Result<ProjectDoc, UserError> {
    let path = std::path::PathBuf::from(&dir);
    let to_load = path.clone();
    match blocking(move || project::load(&to_load)).await? {
        Ok(loaded) => {
            remember_project(&app, &loaded.dir);
            Ok(loaded.into())
        }
        Err(e) => {
            if let Ok(settings_file) = settings_path() {
                let _ = settings::forget_project(&settings_file, &path);
            }
            crate::menu::refresh(&app);
            Err(e
                .context(format!(
                    "Couldn't open \"{}\". It was removed from Open Recent.",
                    project::project_name(&path)
                ))
                .into())
        }
    }
}

/// Opens the project folder in the system's file manager (Finder).
#[tauri::command]
pub fn show_project_folder(dir: String) -> Result<(), UserError> {
    let platform = platform::current_implemented()?;
    platform
        .reveal_folder(Path::new(&dir))
        .context("Couldn't show the project folder.")?;
    Ok(())
}

/// Puts the project folder's path on the clipboard, quoted for the shell
/// when it needs it (a space, for one), to paste after `cd` in a terminal.
/// Returns the text copied.
#[tauri::command]
pub fn copy_project_path(dir: String) -> Result<String, UserError> {
    let platform = platform::current_implemented()?;
    let text = platform.shell_path(&dir);
    platform
        .copy_text(&text)
        .context("Couldn't copy the project folder's path.")?;
    Ok(text)
}

/// Asks for a new project name and location, then creates the project
/// there, copying the other contents of `source_dir` (except `removed`) if
/// given. The dialog starts next to `project_dir`, the project's own
/// folder (none for a project never saved). `None` if cancelled.
#[tauri::command]
// A Tauri command: its arguments are the UI call's named fields.
#[allow(clippy::too_many_arguments)]
pub async fn save_project_as_dialog(
    app: AppHandle,
    project_dir: Option<String>,
    source_dir: Option<String>,
    suggested_name: String,
    files: Vec<ProjectFile>,
    removed: Vec<String>,
    board: Option<String>,
    variant: project::Variant,
) -> Result<Option<ProjectDoc>, UserError> {
    let mut dialog = app.dialog().file().set_title("Save Project As");
    // Start next to the project's own folder (a project never saved, even
    // one copied from an example, starts in the projects folder), never
    // inside a project, with a name nothing there has yet.
    if let Some(root) = project::default_projects_root() {
        let _ = std::fs::create_dir_all(&root);
        let start = project::save_as_start(project_dir.as_deref().map(Path::new), &root);
        let name = project::unique_name(&start, &suggested_name);
        dialog = dialog.set_directory(start).set_file_name(name);
    } else {
        dialog = dialog.set_file_name(suggested_name);
    }
    let Some(picked) = blocking(move || dialog.blocking_save_file()).await? else {
        return Ok(None);
    };
    let target = picked
        .into_path()
        .map_err(|e| anyhow!("{e}"))
        .context("Couldn't use that location.")?;
    let target = project::normalize_new_project_path(&target);
    let created = blocking(move || {
        project::save_as(
            &target,
            source_dir.as_deref().map(Path::new),
            &files,
            &removed,
            board.as_deref(),
            &variant,
            |old| {
                platform::move_to_trash(old).context("Couldn't replace the old project.")?;
                crate::log!("Moved {} to the Trash (replaced by Save As)", old.display());
                Ok(())
            },
        )?;
        project::load(&target)
    })
    .await??;
    remember_project(&app, &created.dir);
    Ok(Some(created.into()))
}

/// Checks a name for a new or renamed file. Returns a message for the user
/// if it can't be used, or `None` if it's fine.
#[tauri::command]
pub fn check_file_name(name: String, existing: Vec<String>) -> Option<String> {
    project::check_new_file_name(&name, &existing).err()
}

/// Asks the user to confirm an action. True if confirmed. A warning (for
/// something destructive) unless `kind` is "info" (for an offer, such as
/// saving a project that has no folder yet).
#[tauri::command]
pub async fn ask_confirm(
    app: AppHandle,
    title: String,
    message: String,
    ok_label: String,
    kind: Option<String>,
) -> Result<bool, UserError> {
    let kind = match kind.as_deref() {
        Some("info") => MessageDialogKind::Info,
        _ => MessageDialogKind::Warning,
    };
    let dialog = app
        .dialog()
        .message(message)
        .title(title)
        .kind(kind)
        .buttons(MessageDialogButtons::OkCancelCustom(
            ok_label,
            "Cancel".into(),
        ));
    blocking(move || dialog.blocking_show()).await
}

/// Asks what to do with unsaved changes: "save", "discard" or "cancel".
#[tauri::command]
pub async fn ask_save_changes(app: AppHandle, name: String) -> Result<String, UserError> {
    const SAVE: &str = "Save";
    const DISCARD: &str = "Don't Save";
    const CANCEL: &str = "Cancel";
    let dialog = app
        .dialog()
        .message("Your changes will be lost if you don't save them.")
        .title(format!("Save changes to \"{name}\"?"))
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancelCustom(
            SAVE.into(),
            DISCARD.into(),
            CANCEL.into(),
        ));
    let answer = match blocking(move || dialog.blocking_show_with_result()).await? {
        MessageDialogResult::Yes => "save",
        MessageDialogResult::No => "discard",
        MessageDialogResult::Custom(label) if label == SAVE => "save",
        MessageDialogResult::Custom(label) if label == DISCARD => "discard",
        _ => "cancel",
    };
    Ok(answer.into())
}

/// Updates the window title and the unsaved-changes flag used when closing.
#[tauri::command]
pub fn set_document_state(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    name: String,
    dirty: bool,
) {
    state.dirty.store(dirty, Ordering::SeqCst);
    let marker = if dirty { "● " } else { "" };
    // As in the Arduino IDE: e.g. "blinky | Logix IDE 0.1.0", or
    // "● blinky | Logix IDE 0.1.0" with unsaved changes.
    let version = &window.app_handle().package_info().version;
    // No project (the start screen): just the app's name.
    let title = if name.is_empty() {
        "Logix IDE".to_string()
    } else {
        format!("{marker}{name} | Logix IDE {version}")
    };
    let _ = window.set_title(&title);
}

/// Shows the output pane's right-click menu (see `menu::show_output_menu`).
#[tauri::command]
pub fn show_output_menu(window: tauri::WebviewWindow) -> Result<(), UserError> {
    crate::menu::show_output_menu(&window).context("Can't show the output menu")?;
    Ok(())
}

/// Shows the editor's right-click menu (see `menu::show_editor_menu`),
/// with the items the menu state allows.
#[tauri::command]
pub fn show_editor_menu(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), UserError> {
    let menu_state = state.menu_state.lock().unwrap().clone();
    crate::menu::show_editor_menu(&window, &menu_state).context("Can't show the editor menu")?;
    Ok(())
}

/// What the menu items can act on now (see `menu::MenuState`). The menu bar
/// is rebuilt only when it changes.
#[tauri::command]
pub fn set_menu_state(app: AppHandle, state: State<'_, AppState>, menu: crate::menu::MenuState) {
    let changed = {
        let mut current = state.menu_state.lock().unwrap();
        let changed = *current != menu;
        *current = menu;
        changed
    };
    if changed {
        crate::menu::refresh(&app);
    }
}

/// Appends lines the output pane showed to the log file (see `logging.rs`).
#[tauri::command]
pub fn log_output(lines: Vec<String>) {
    crate::logging::output(&lines);
}

/// Closes the window after the UI has handled unsaved changes.
#[tauri::command]
pub fn close_window(window: tauri::WebviewWindow, state: State<'_, AppState>) {
    state.force_close.store(true, Ordering::SeqCst);
    let _ = window.close();
}

/// Runs blocking work (dialogs, file I/O) off the async runtime.
async fn blocking<T, F>(f: F) -> Result<T, UserError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| UserError::from(anyhow!("internal error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn user_error_splits_message_and_details() {
        let err = Err::<(), _>(anyhow::anyhow!("dns error"))
            .context("Couldn't download Logix Engine.")
            .unwrap_err();
        let user = UserError::from(err);
        assert_eq!(user.message, "Couldn't download Logix Engine.");
        assert_eq!(user.details, "Couldn't download Logix Engine.: dns error");
    }
}
