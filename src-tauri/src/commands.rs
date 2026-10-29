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

use crate::apio::{self, SetupEvent, SetupStage};
use crate::project::{self, ProjectFile};
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

/// Makes sure Apio and its packages are installed, streaming progress to
/// `on_event`. Safe to call again (e.g. to retry after an error).
#[tauri::command]
pub async fn setup_apio(
    state: State<'_, AppState>,
    on_event: Channel<SetupEvent>,
) -> Result<(), UserError> {
    let mut apio_slot = state.apio.lock().await;
    if apio_slot.is_none() {
        let channel = on_event.clone();
        let result = apio::run_setup(move |event| {
            let _ = channel.send(event);
        })
        .await;
        match result {
            Ok(ready) => {
                crate::log!("Apio setup ready");
                *apio_slot = Some(ready);
            }
            Err(e) => {
                crate::log!("Apio setup failed: {e:#}");
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
    /// A new project with starter content (from File > New). The UI may
    /// replace it with another board's starter while it's untouched.
    starter: bool,
}

impl From<project::Project> for ProjectDoc {
    fn from(p: project::Project) -> Self {
        ProjectDoc {
            dir: Some(p.dir.to_string_lossy().into_owned()),
            template_dir: None,
            name: p.name,
            files: p.files,
            board: p.board,
            starter: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Boards and examples
// ---------------------------------------------------------------------------

/// The ready Apio installation, or a user-facing error if setup hasn't
/// finished.
async fn ready_apio(state: &AppState) -> Result<apio::Apio, UserError> {
    state
        .apio
        .lock()
        .await
        .clone()
        .ok_or_else(|| anyhow!("The FPGA tools aren't ready yet.").into())
}

/// Sends the output of primary Apio commands that aren't part of a build,
/// upload, test or setup (which stream through their own channel) to the
/// console, as `apio-output` events. Info commands (`apio api ...`) run
/// silently instead.
fn console_log(app: &AppHandle) -> impl Fn(String) + Send + Sync + Clone + 'static {
    let app = app.clone();
    move |line| {
        let _ = app.emit("apio-output", line);
    }
}

/// The ready Apio installation, or `None` if setup hasn't finished or is
/// still running (doesn't wait for it).
fn try_ready_apio(state: &AppState) -> Option<apio::Apio> {
    state.apio.try_lock().ok()?.clone()
}

/// The board and example lists (fetched from Apio on first use).
async fn catalog(state: &AppState) -> Result<Catalog, UserError> {
    let mut slot = state.catalog.lock().await;
    if let Some(c) = slot.as_ref() {
        return Ok(c.clone());
    }
    let apio = ready_apio(state).await?;
    let boards = apio
        .boards()
        .await
        .context("Couldn't get the list of boards.")?;
    let examples = apio
        .examples()
        .await
        .context("Couldn't get the list of examples.")?;
    let usb_rules = apio.usb_rules().await;
    let c = Catalog {
        boards,
        examples,
        usb_rules,
    };
    *slot = Some(c.clone());
    Ok(c)
}

#[tauri::command]
pub async fn get_boards(state: State<'_, AppState>) -> Result<Vec<apio::Board>, UserError> {
    Ok(catalog(&state).await?.boards)
}

/// The examples Logix supports, and how many unsupported ones were hidden.
#[tauri::command]
pub async fn get_examples(state: State<'_, AppState>) -> Result<apio::ExampleList, UserError> {
    Ok(catalog(&state).await?.examples)
}

/// Opens an example as a new, unsaved project set to the example's board.
#[tauri::command]
pub async fn open_example(
    app: AppHandle,
    state: State<'_, AppState>,
    board: String,
    name: String,
) -> Result<ProjectDoc, UserError> {
    let apio = ready_apio(&state).await?;
    example_doc(&app, &apio, board, &name, name.clone()).await
}

/// An example as a new, unsaved project named `doc_name`.
async fn example_doc(
    app: &AppHandle,
    apio: &apio::Apio,
    board: String,
    example: &str,
    doc_name: String,
) -> Result<ProjectDoc, UserError> {
    let dir = apio
        .fetch_example(&board, example, console_log(app))
        .await
        .context("Couldn't open the example.")?;
    let loaded = blocking(move || project::load(&dir)).await??;
    Ok(ProjectDoc {
        dir: None,
        template_dir: Some(loaded.dir.to_string_lossy().into_owned()),
        name: doc_name,
        files: loaded.files,
        board: Some(board),
        starter: false,
    })
}

// ---------------------------------------------------------------------------
// Build and upload
// ---------------------------------------------------------------------------

/// Builds or uploads the project's current (possibly unsaved) files in a
/// temporary Apio project, streaming Apio's output lines to `on_line`.
/// `key` identifies the project (so builds of the same project reuse
/// Apio's output); `source_dir` is the folder whose other files (e.g.
/// memory init data) are copied along.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn run_task(
    app: AppHandle,
    state: State<'_, AppState>,
    task: apio::Task,
    testbench: Option<String>,
    key: String,
    source_dir: Option<String>,
    project_dir: Option<String>,
    files: Vec<ProjectFile>,
    board: String,
    on_line: Channel<String>,
) -> Result<apio::TaskOutcome, UserError> {
    let Ok(_running) = state.task.try_lock() else {
        return Err(anyhow!("A build or upload is already running.").into());
    };
    let apio = ready_apio(&state).await?;
    let root = platform::current_implemented()?.data_dir()?.join("build");
    let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    // `testbench` is the testbench for Test/Simulate, or the module to draw
    // for Graph. It's passed to Apio as an argument, so only accept project
    // testbenches and plain Verilog module names.
    if let Some(target) = &testbench {
        let ok = if task == apio::Task::Graph {
            project::is_verilog_identifier(target)
        } else {
            project::is_testbench(target) && names.contains(target)
        };
        if !ok {
            return Err(anyhow!("\"{target}\" can't be used here.").into());
        }
    }
    let constraint = build_dir::constraint_file(&files).map(str::to_owned);
    let log = on_line.clone();
    let dir = blocking(move || {
        // Keep GTKWave layouts saved since the last run; preparing clears
        // the build folder.
        if let Some(project_dir) = &project_dir {
            for name in save_back_gtkw(&root, &key, Path::new(project_dir)) {
                let _ = log.send(gtkw_saved_message(&name));
            }
        }
        let prepared = build_dir::prepare(
            &root,
            &build_dir::BuildInput {
                key: &key,
                source_dir: source_dir.as_deref().map(Path::new),
                files: &files,
                board: &board,
            },
        )
        .context("Couldn't prepare the build.")?;
        if !prepared.left_out.is_empty() {
            let _ = log.send(left_out_message(&prepared.left_out));
        }
        Ok::<_, anyhow::Error>(prepared.dir)
    })
    .await??;
    // GTKWave windows open before the simulation, to recognize the new one.
    let gtkwave_before = if task == apio::Task::Simulate {
        on_main_thread(&app, || platform::running_app_pids(GTKWAVE))
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    *state.task_cancel.lock().unwrap() = Some(cancel.clone());
    let result = apio
        .run_task(
            task,
            testbench.as_deref(),
            lint_on_verify(),
            &dir,
            names,
            constraint,
            move |line| {
                let _ = on_line.send(line);
            },
            &cancel,
        )
        .await;
    *state.task_cancel.lock().unwrap() = None;
    let outcome = result.context("Couldn't run Apio.")?;
    if task == apio::Task::Simulate && outcome.ok {
        tauri::async_runtime::spawn(bring_gtkwave_to_front(app, gtkwave_before));
    }
    Ok(outcome)
}

/// How GTKWave's process is named.
const GTKWAVE: &str = "gtkwave";

/// After `apio sim --detach`, GTKWave starts in the background and its
/// window would stay behind Logix. Waits for the new GTKWave (up to ~8 s)
/// and brings it to the front.
async fn bring_gtkwave_to_front(app: AppHandle, before: Vec<i32>) {
    use std::time::Duration;
    for _ in 0..32 {
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
        return;
    }
    crate::log!("Didn't find a new GTKWave window to bring to the front");
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

/// Starts the Verilog language server, if it isn't running. Its messages
/// go to the UI as `lsp-message` events (JSON text); `lsp-closed` tells
/// the UI it stopped.
#[tauri::command]
pub async fn lsp_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), UserError> {
    let mut slot = state.lsp.lock().await;
    if slot.is_some() {
        return Ok(());
    }
    let apio = ready_apio(&state).await?;
    let exe = apio
        .language_server_path()
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
                app.state::<AppState>().lsp.lock().await.take();
                let _ = app.emit("lsp-closed", ());
            });
        }
    };
    let server = apio::LanguageServer::start(&exe, on_message, on_exit)
        .context("Couldn't start the Verilog language server.")?;
    crate::log!("Language server started: {}", exe.display());
    *slot = Some(server);
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
    let server = slot
        .as_mut()
        .ok_or_else(|| anyhow!("The Verilog language server isn't running."))?;
    server
        .send(&message)
        .await
        .context("Couldn't talk to the Verilog language server.")?;
    Ok(())
}

/// The connected USB devices and serial ports (Tools > Connected Devices,
/// and hints when Upload can't find the board).
#[tauri::command]
pub async fn scan_devices(state: State<'_, AppState>) -> Result<apio::Devices, UserError> {
    let apio = ready_apio(&state).await?;
    Ok(apio
        .scan_devices()
        .await
        .context("Couldn't list the connected devices.")?)
}

/// The boards that match the connected USB devices, for the board list's
/// "Connected" group. Silent; polled by the UI while it's idle.
#[tauri::command]
pub async fn connected_boards(
    state: State<'_, AppState>,
) -> Result<Vec<apio::ConnectedBoard>, UserError> {
    let apio = ready_apio(&state).await?;
    let rules = catalog(&state).await?.usb_rules;
    // Polled every few seconds: log only changes, not every scan.
    let devices = apio::quietly(apio.scan_devices())
        .await
        .context("Couldn't list the connected devices.")?;
    let found = apio::match_boards(&rules, &devices);
    static LAST: std::sync::Mutex<Option<Vec<apio::ConnectedBoard>>> = std::sync::Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_ref() != Some(&found) {
        let names: Vec<_> = found.iter().map(|c| c.board.as_str()).collect();
        crate::log!("Connected boards: {names:?}");
        *last = Some(found.clone());
    }
    Ok(found)
}

/// The output pane's note about project files a build left out.
fn left_out_message(left_out: &[String]) -> String {
    let what: Vec<String> = left_out
        .iter()
        .map(|p| match p.strip_suffix('/') {
            Some(dir) => format!("{dir}/ (a project inside this one)"),
            None => p.clone(),
        })
        .collect();
    format!(
        "Not used in the build: {}. Logix only builds the files at the top of the \
         project folder.",
        what.join(", ")
    )
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
    diagnostics: Vec<apio::Diagnostic>,
}

/// Formats one file's text with `apio format` (Verible) in a scratch
/// project and returns the result; Apio's output goes to the console. The project folder isn't touched.
/// `board` is only needed because Apio requires one in `apio.ini`; any
/// board works, so the first known one is used if the project has none.
#[tauri::command]
pub async fn format_file(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    text: String,
    board: Option<String>,
) -> Result<FormatResult, UserError> {
    if !project::is_hdl_source(&name) {
        return Err(anyhow!(
            "Auto Format works on Verilog and SystemVerilog files (.v .sv .vh .svh)."
        )
        .into());
    }
    let _formatting = state.format.lock().await;
    let apio = ready_apio(&state).await?;
    let board = match board {
        Some(b) => b,
        None => catalog(&state)
            .await?
            .boards
            .first()
            .map(|b| b.id.clone())
            .ok_or_else(|| UserError::from(anyhow!("No boards are available.")))?,
    };
    let root = platform::current_implemented()?.data_dir()?.join("format");
    let file = ProjectFile {
        name: name.clone(),
        text,
    };
    let dir = blocking(move || {
        build_dir::prepare_single(&root, &file, &board).context("Couldn't prepare formatting.")
    })
    .await??;
    let outcome = apio
        .format_file(&dir, &name, console_log(&app))
        .await
        .context("Couldn't run Apio.")?;
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

/// A new, unsaved project for `board` (or else the last chosen board). If
/// the FPGA tools are ready and the board has a starter example (`blinky`
/// or `ledon`), the project starts from it, so it has the board's pin file
/// and builds right away. Otherwise it gets the generic starter `main.v`.
#[tauri::command]
pub async fn new_project(
    app: AppHandle,
    state: State<'_, AppState>,
    board: Option<String>,
) -> Result<ProjectDoc, UserError> {
    // A new project always has a board: the given one, the last one chosen,
    // or else the default.
    let board = board
        .or_else(|| {
            settings_path()
                .ok()
                .and_then(|p| settings::load(&p).last_board)
        })
        .or_else(|| Some(apio::DEFAULT_BOARD.to_string()));
    if let (Some(b), Some(apio)) = (&board, try_ready_apio(&state)) {
        if let Ok(catalog) = catalog(&state).await {
            match apio::starter_example(&catalog.examples.examples, b) {
                Some(ex) => {
                    let name = ex.name.clone();
                    if let Ok(mut doc) =
                        example_doc(&app, &apio, b.clone(), &name, "untitled".into()).await
                    {
                        doc.starter = true;
                        return Ok(doc);
                    }
                }
                // No example to take the pins from: say what's missing.
                None => return Ok(no_pins_doc(b.clone(), true)),
            }
        }
    }
    // The tools aren't ready (first run, or setup failed).
    Ok(ProjectDoc {
        dir: None,
        template_dir: None,
        name: "untitled".into(),
        files: project::starter_files(),
        board,
        starter: true,
    })
}

/// A board's Bare Minimum project (Examples): its pins and a design that
/// does nothing yet (see `bare.rs`), as a new, unsaved project.
#[tauri::command]
pub async fn open_bare_minimum(
    app: AppHandle,
    state: State<'_, AppState>,
    board: String,
) -> Result<ProjectDoc, UserError> {
    let apio = ready_apio(&state).await?;
    let catalog = catalog(&state).await?;
    let Some(ex) = apio::starter_example(&catalog.examples.examples, &board) else {
        return Ok(no_pins_doc(board, false));
    };
    let name = ex.name.clone();
    let starter = example_doc(&app, &apio, board.clone(), &name, "untitled".into()).await?;
    Ok(match bare::bare_files(&starter.files) {
        Some(files) => ProjectDoc {
            dir: None,
            template_dir: None,
            name: "untitled".into(),
            files,
            board: Some(board),
            starter: false,
        },
        None => no_pins_doc(board, false),
    })
}

/// A new project for a board without an example to take the pins from.
fn no_pins_doc(board: String, starter: bool) -> ProjectDoc {
    ProjectDoc {
        dir: None,
        template_dir: None,
        name: "untitled".into(),
        files: vec![ProjectFile {
            name: project::MAIN_FILE.into(),
            text: bare::NO_PINS_MAIN.into(),
        }],
        board: Some(board),
        starter,
    }
}

/// Remembers the board the user chose, for new projects.
#[tauri::command]
pub fn set_last_board(board: String) {
    if let Ok(path) = settings_path() {
        if let Err(e) = settings::remember_board(&path, &board) {
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
) -> Result<(), UserError> {
    let saved_dir = std::path::PathBuf::from(&dir);
    blocking(move || {
        project::save(Path::new(&dir), &files, &removed, board.as_deref())
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

/// Asks for a new project name and location, then creates the project
/// there, copying the other contents of `source_dir` (except `removed`) if
/// given. `None` if cancelled.
#[tauri::command]
pub async fn save_project_as_dialog(
    app: AppHandle,
    source_dir: Option<String>,
    suggested_name: String,
    files: Vec<ProjectFile>,
    removed: Vec<String>,
    board: Option<String>,
) -> Result<Option<ProjectDoc>, UserError> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Save Project As")
        .set_file_name(suggested_name);
    if let Some(root) = project::default_projects_root() {
        let _ = std::fs::create_dir_all(&root);
        dialog = dialog.set_directory(root);
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
        project::create(
            &target,
            source_dir.as_deref().map(Path::new),
            &files,
            &removed,
            board.as_deref(),
        )
        .context("Couldn't save the project.")?;
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

/// Asks the user to confirm something destructive. True if confirmed.
#[tauri::command]
pub async fn ask_confirm(
    app: AppHandle,
    title: String,
    message: String,
    ok_label: String,
) -> Result<bool, UserError> {
    let dialog = app
        .dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
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
    let _ = window.set_title(&format!("{marker}{name} | Logix IDE {version}"));
}

/// Shows the output pane's right-click menu (see `menu::show_output_menu`).
#[tauri::command]
pub fn show_output_menu(window: tauri::WebviewWindow) -> Result<(), UserError> {
    crate::menu::show_output_menu(&window).context("Can't show the output menu")?;
    Ok(())
}

/// Shows the editor's right-click menu (see `menu::show_editor_menu`).
#[tauri::command]
pub fn show_editor_menu(
    window: tauri::WebviewWindow,
    can_go_to_definition: bool,
) -> Result<(), UserError> {
    crate::menu::show_editor_menu(&window, can_go_to_definition)
        .context("Can't show the editor menu")?;
    Ok(())
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
            .context("Couldn't download Apio.")
            .unwrap_err();
        let user = UserError::from(err);
        assert_eq!(user.message, "Couldn't download Apio.");
        assert_eq!(user.details, "Couldn't download Apio.: dns error");
    }
}
