//! The native menu bar. Menu items that act on the document (New, Open,
//! Save, ...) are forwarded to the UI as `menu` events with the item id;
//! Close and Quit are handled here, through the normal window-close flow.

use std::path::{Path, PathBuf};

use tauri::menu::{
    AboutMetadata, CheckMenuItemBuilder, ContextMenu, IsMenuItem, Menu, MenuBuilder,
    MenuItemBuilder, MenuItemKind, PredefinedMenuItem, Submenu, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

use crate::{platform, settings};

/// Menu ids forwarded to the UI (see `onMenu` in `src/lib/backend.ts`).
/// File > New Project.
pub const NEW: &str = "new";
/// File > Close Project: back to the start screen.
pub const CLOSE_PROJECT: &str = "close-project";
pub const OPEN: &str = "open";
pub const SAVE: &str = "save";
pub const SAVE_AS: &str = "save-as";
pub const NEW_FILE: &str = "new-file";
pub const EXAMPLES: &str = "examples";
pub const VERIFY: &str = "verify";
pub const UPLOAD: &str = "upload";
/// Project > Debug: a debug build, Upload, then the debug panel
/// (`commands.rs`, `run_task`; Logix Engine's docs/debugger.md).
pub const DEBUG: &str = "debug";
/// Project > Choose Debug Nets…: the net picker again, with debug.toml's
/// choice ticked, to change it (the UI's `pickNets`).
pub const DEBUG_NETS: &str = "debug-nets";
pub const RUN_TESTS: &str = "run-tests";
pub const AUTO_FORMAT: &str = "auto-format";
pub const TOGGLE_OUTPUT: &str = "toggle-output";
pub const SIMULATE: &str = "simulate";
pub const SHOW_FOLDER: &str = "show-folder";
pub const COPY_PATH: &str = "copy-path";
pub const SHOW_DIAGRAM: &str = "show-diagram";
/// Project > Board Utilization…: builds (without lint; instant when up to
/// date), then shows the report (`BuildReport.svelte`).
pub const BUILD_REPORT: &str = "build-report";
pub const DEVICES: &str = "devices";
/// Tools > Serial Monitor: the bottom pane's Serial Monitor tab, open on
/// the board's serial port (as Arduino IDE's, and its key).
pub const SERIAL_MONITOR: &str = "serial-monitor";
/// Tools > Board Explorer…: the FPGA families, FPGAs, boards and examples
/// Logix IDE knows, and how they relate, opened at the toolbar's board.
pub const BOARD_EXPLORER: &str = "board-explorer";
pub const SETTINGS: &str = "settings";
pub const GO_TO_DEFINITION: &str = "go-to-definition";
/// Edit > Go to File…: the list of the project's files, recent ones first
/// (the UI's FileList), as the tab bar's ▾ opens it.
pub const GO_TO_FILE: &str = "go-to-file";
pub const TOGGLE_COMMENT: &str = "toggle-comment";
/// Edit > Select All: the UI selects all of what has the focus (editor,
/// output pane, or a text field), not the whole window.
pub const SELECT_ALL: &str = "select-all";
/// The output pane's right-click menu.
pub const OUTPUT_SELECT_ALL: &str = "output-select-all";
pub const CLEAR_OUTPUT: &str = "clear-output";
/// Editor right-click menu, in a testbench: run or simulate that testbench.
pub const RUN_THIS_TEST: &str = "run-this-test";
/// The editor's right-click menu: the diagram of the module at the cursor.
pub const SHOW_MODULE_DIAGRAM: &str = "show-module-diagram";
pub const SIMULATE_THIS_TEST: &str = "simulate-this-test";
/// Sent to the UI (as a `menu` event) when File > Open Recent > Clear Menu
/// emptied the list, so the start screen reloads its recent projects.
pub const RECENTS_CHANGED: &str = "recents-changed";
/// Help menu items are "help:<index into HELP_LINKS>".
const HELP_PREFIX: &str = "help:";

/// Help menu: (label, web page). `None` marks a separator.
const HELP_LINKS: &[Option<(&str, &str)>] = &[
    Some(("Logix IDE Documentation", "https://www.openlogix.org/logix-ide/")),
    Some(("Tutorial", "https://www.openlogix.org/logix-ide/tutorial/")),
    Some((
        "Testbenches and Simulation",
        "https://www.openlogix.org/logix-ide/testbenches/",
    )),
    Some((
        "Using an AI Agent",
        "https://www.openlogix.org/logix-ide/ai-agents/",
    )),
    Some((
        "Troubleshooting",
        "https://www.openlogix.org/logix-ide/troubleshooting/",
    )),
    Some((
        "Keyboard Shortcuts",
        "https://www.openlogix.org/logix-ide/using-logix/#keyboard-shortcuts",
    )),
    None,
    Some(("Logix IDE Website", "https://www.openlogix.org/ide/")),
    Some(("Logix IDE on GitHub", "https://github.com/openlogix/logix-ide")),
    Some((
        "Report a Problem",
        // A new issue whose text asks for the log (see SHOW_LOGS).
        "https://github.com/openlogix/logix-ide/issues/new?body=What%20happened%2C%20and%20what%20did%20you%20expect%3F%0A%0A%0ALogix%20IDE%20log%3A%20in%20Logix%20IDE%2C%20choose%20Help%20%3E%20Show%20Log%20Folder%20and%20drag%20the%20.log%20files%20in%20that%20folder%20here.%0A",
    )),
];
pub const STOP: &str = "stop";
/// Menu items that act on the open project: greyed out (their shortcuts
/// too) on the start screen, when no project is open.
pub const NEEDS_PROJECT: &[&str] = &[
    CLOSE_PROJECT,
    NEW_FILE,
    SAVE,
    SAVE_AS,
    TOGGLE_COMMENT,
    GO_TO_DEFINITION,
    GO_TO_FILE,
    VERIFY,
    UPLOAD,
    DEBUG,
    DEBUG_NETS,
    RUN_TESTS,
    SIMULATE,
    SHOW_DIAGRAM,
    RUN_THIS_TEST,
    SIMULATE_THIS_TEST,
    SHOW_MODULE_DIAGRAM,
    BUILD_REPORT,
    STOP,
    SHOW_FOLDER,
    COPY_PATH,
    AUTO_FORMAT,
];

/// What the menu items can act on now, reported by the UI
/// (`set_menu_state`). Items that can't be used are greyed out, shortcuts
/// included, in the menu bar and the editor's right-click menu.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuState {
    /// A project is open (not the start screen).
    pub project: bool,
    /// The open tab can be formatted (Auto Format): Verilog or
    /// SystemVerilog (.v .sv .vh .svh), or the project's custom-boards.json.
    /// Pin files have no formatter.
    pub formattable: bool,
    /// The open tab has line comments: Verilog (`//`) or a pin file (`#`,
    /// or `//` for .cst): Comment/Uncomment.
    pub comment: bool,
    /// Go to Definition can answer: a Verilog tab and live checking running.
    pub definition: bool,
    /// The project has testbenches (Run Tests).
    pub testbenches: bool,
    /// The project has a testbench to simulate (Simulate; the UI picks it or
    /// asks which one: `simulateChoice.ts`).
    pub simulate: bool,
    /// The open tab is a testbench: Run Test and Simulate Test (in the
    /// editor's right-click menu only then, to keep it short).
    pub testbench_open: bool,
    /// The open tab is a design file that declares a module: Module
    /// Diagram.
    #[serde(default)]
    pub module_open: bool,
    /// One Tools submenu per dimension of the current board's variants,
    /// e.g. Tools > Memory ▸ Flash / RAM. Empty for most boards.
    #[serde(default)]
    pub variant_menus: Vec<VariantMenu>,
}

/// A Tools submenu for one dimension of the board's variants.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantMenu {
    /// The dimension's id, e.g. "memory".
    pub id: String,
    /// Its name, e.g. "Memory".
    pub label: String,
    pub values: Vec<VariantMenuValue>,
}

/// One value in a variant submenu.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantMenuValue {
    pub id: String,
    pub label: String,
    /// The project's current value.
    pub checked: bool,
    /// False for a value that has no configuration with the other current
    /// values (choosing it would change them, so it's greyed out).
    pub enabled: bool,
}

/// Variant submenu items are "variant:<dimension>:<value>"; the UI applies
/// them.
pub const VARIANT_PREFIX: &str = "variant:";

/// Whether the item `id` can be used in `state`.
pub fn is_enabled(id: &str, state: &MenuState) -> bool {
    if NEEDS_PROJECT.contains(&id) && !state.project {
        return false;
    }
    match id {
        AUTO_FORMAT => state.formattable,
        TOGGLE_COMMENT => state.comment,
        GO_TO_DEFINITION => state.definition,
        RUN_TESTS => state.testbenches,
        SIMULATE => state.simulate,
        RUN_THIS_TEST | SIMULATE_THIS_TEST => state.testbench_open,
        SHOW_MODULE_DIAGRAM => state.module_open,
        _ => true,
    }
}
/// Open Recent items are "recent:<index into the recent projects>".
const RECENT_PREFIX: &str = "recent:";
const CLEAR_RECENT: &str = "clear-recent";
/// Help > Show Log Folder: this session's log folder, with Logix IDE's log
/// and the logs of the programs it started (see `logging.rs`).
const SHOW_LOGS: &str = "show-logs";
/// Help > System Info…: the UI asks for the report (`system_info`).
const SYSTEM_INFO: &str = "system-info";
/// No keyboard shortcut. Simulate (Ctrl+Alt+T), Stop (Ctrl+.), Show
/// Project Folder (Ctrl+Alt+K) and Examples (Ctrl+Shift+E) had ones that
/// clash or may clash: Ubuntu opens a terminal on Ctrl+Alt+T, Ctrl+Alt is
/// AltGr on many Windows keyboards, IBus's emoji picker uses Ctrl+. (and
/// Ctrl+Shift+E in older versions). Dropped until better keys are chosen
/// (the user's decision, 2026-11-18).
const NO_KEY: &str = "";
/// Help > Check for Updates: the UI runs `check_for_updates`.
const CHECK_UPDATES: &str = "check-for-updates";
const CLOSE: &str = "close";
/// File > Open Recent, and Tools: submenus `refresh` changes in place.
const OPEN_RECENT: &str = "open-recent";
const TOOLS_MENU: &str = "tools-menu";
const QUIT: &str = "quit";

/// The About box's text. It names the Logix Engine release in use, so it
/// changes once setup has chosen one (`refresh`).
fn about_metadata() -> AboutMetadata<'static> {
    AboutMetadata {
        name: Some("Logix IDE".into()),
        version: Some(crate::version::APP_VERSION.to_string()),
        comments: Some(format!(
            "The Arduino experience, for FPGAs.\n{}\n\nLogix IDE is not affiliated with or endorsed by Arduino. Arduino is a trademark of Arduino SA.",
            match crate::engine::in_use().map(|u| u.engine) {
                Some(crate::engine::EngineInUse::Release(release)) => format!(
                    "Powered by Logix Engine (release {}).",
                    crate::version::shown(&release)
                ),
                Some(crate::engine::EngineInUse::Local(_)) => "Powered by a local Logix Engine.".into(),
                None => "Powered by Logix Engine.".into(),
            }
        )),
        copyright: Some("Licensed under MIT or the Apache License 2.0.".into()),
        website: Some("https://www.openlogix.org/ide/".into()),
        ..Default::default()
    }
}

/// File > Open Recent's items: the recent projects (or "No Recent
/// Projects"), then Clear Menu. None has a shortcut, so `refresh` can
/// replace them in place.
fn recent_entries<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Vec<MenuItemKind<R>>> {
    let recents = recent_projects();
    let mut out: Vec<MenuItemKind<R>> = Vec::new();
    for (i, label) in recent_labels(&recents).into_iter().enumerate() {
        out.push(
            MenuItemBuilder::with_id(format!("{RECENT_PREFIX}{i}"), label)
                .build(app)?
                .kind(),
        );
    }
    if recents.is_empty() {
        out.push(
            MenuItemBuilder::with_id("no-recent", "No Recent Projects")
                .enabled(false)
                .build(app)?
                .kind(),
        );
    }
    out.push(PredefinedMenuItem::separator(app)?.kind());
    out.push(
        MenuItemBuilder::with_id(CLEAR_RECENT, "Clear Menu")
            .enabled(!recents.is_empty())
            .build(app)?
            .kind(),
    );
    Ok(out)
}

/// A board option's submenu title in Tools: the engine's dimension label
/// with "Board" in front ("Board Memory", "Board Connection"), since in the
/// menu nothing else says whose memory it is (the user's decision,
/// 2026-11-22; the variant panel next to the board keeps the plain label).
pub fn variant_menu_title(label: &str) -> String {
    format!("Board {label}")
}

/// Tools' board options (one submenu per dimension, as Arduino's board
/// options), then a separator when there are any. No item has a shortcut,
/// so `refresh` can replace them in place.
fn variant_entries<R: Runtime>(
    app: &AppHandle<R>,
    state: &MenuState,
) -> tauri::Result<Vec<MenuItemKind<R>>> {
    let mut out: Vec<MenuItemKind<R>> = Vec::new();
    for dim in &state.variant_menus {
        let mut sub = SubmenuBuilder::new(app, variant_menu_title(&dim.label));
        for v in &dim.values {
            sub = sub.item(
                &CheckMenuItemBuilder::with_id(
                    format!("{VARIANT_PREFIX}{}:{}", dim.id, v.id),
                    &v.label,
                )
                .checked(v.checked)
                .enabled(v.enabled)
                .build(app)?,
            );
        }
        out.push(sub.build()?.kind());
    }
    if !state.variant_menus.is_empty() {
        out.push(PredefinedMenuItem::separator(app)?.kind());
    }
    Ok(out)
}

/// Borrows items for the builders' `items`.
fn refs<R: Runtime>(items: &[MenuItemKind<R>]) -> Vec<&dyn IsMenuItem<R>> {
    items.iter().map(|i| i as &dyn IsMenuItem<R>).collect()
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let state = app
        .try_state::<crate::AppState>()
        .map(|s| s.menu_state.lock().unwrap().clone())
        .unwrap_or_default();
    // `accel` "" for none (NO_KEY).
    let item = |id: &str, text: &str, accel: &str| {
        let b = MenuItemBuilder::with_id(id, text);
        let b = if accel.is_empty() {
            b
        } else {
            b.accelerator(accel)
        };
        b.enabled(is_enabled(id, &state)).build(app)
    };
    let quit = item(QUIT, "Quit Logix IDE", "CmdOrCtrl+Q")?;
    let settings_item = item(SETTINGS, "Settings…", "CmdOrCtrl+,")?;
    let about = about_metadata();
    // macOS puts Quit in the application menu; others in File.
    let mac = platform::current().is_some_and(|p| p.has_app_menu());

    // File > Open Recent, from the recent projects in the settings.
    let recent = SubmenuBuilder::with_id(app, OPEN_RECENT, "Open Recent")
        .items(&refs(&recent_entries(app)?))
        .build()?;

    // "Project" in the names: these act on the whole project, New File on
    // one file in it.
    // Examples right below New Project, easy to find, as the Arduino IDE's
    // File > Examples.
    let mut file = SubmenuBuilder::new(app, "File")
        .item(&item(NEW, "New Project", "CmdOrCtrl+N")?)
        .item(&item(EXAMPLES, "Examples…", NO_KEY)?)
        .item(&item(OPEN, "Open Project…", "CmdOrCtrl+O")?)
        .item(&recent)
        .item(&item(CLOSE_PROJECT, "Close Project", "CmdOrCtrl+W")?)
        .separator()
        .item(&item(NEW_FILE, "New File…", "CmdOrCtrl+Shift+N")?)
        .separator()
        .item(&item(SAVE, "Save Project", "CmdOrCtrl+S")?)
        .item(&item(SAVE_AS, "Save Project As…", "CmdOrCtrl+Shift+S")?)
        .separator()
        .item(&item(CLOSE, "Close Window", "CmdOrCtrl+Shift+W")?);
    if !mac {
        // As in the Arduino IDE on Windows and Linux: File > Preferences.
        file = file
            .separator()
            .item(&settings_item)
            .separator()
            .item(&quit);
    }
    let file = file.build()?;

    // Standard edit items; without them copy/paste shortcuts don't work in
    // the web view on macOS.
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .item(&item(SELECT_ALL, "Select All", "CmdOrCtrl+A")?)
        .separator()
        .item(&item(TOGGLE_COMMENT, "Comment/Uncomment", "CmdOrCtrl+/")?)
        .item(&item(GO_TO_DEFINITION, "Go to Definition", "F12")?)
        .item(&item(GO_TO_FILE, "Go to File…", "CmdOrCtrl+P")?)
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item(TOGGLE_OUTPUT, "Toggle Output", "CmdOrCtrl+J")?)
        .build()?;

    // Same shortcuts as the Arduino IDE.
    let project = SubmenuBuilder::new(app, "Project")
        .item(&item(VERIFY, "Verify", "CmdOrCtrl+R")?)
        .item(&item(UPLOAD, "Upload", "CmdOrCtrl+U")?)
        .item(&item(DEBUG, "Debug", NO_KEY)?)
        .item(&item(DEBUG_NETS, "Choose Debug Nets…", NO_KEY)?)
        .separator()
        .item(&item(RUN_TESTS, "Run Tests", "CmdOrCtrl+Shift+T")?)
        .item(&item(SIMULATE, "Simulate", NO_KEY)?)
        // The file commands for the open tab, under their project commands
        // (Commands, their scope and where they are: the menu bar has every
        // command; the user's report, 2026-11-22: only right-click menus
        // had them).
        .item(&item(RUN_THIS_TEST, "Run Test", NO_KEY)?)
        .item(&item(SIMULATE_THIS_TEST, "Simulate Test", NO_KEY)?)
        .separator()
        .item(&item(SHOW_DIAGRAM, "Show Diagram", "CmdOrCtrl+Shift+D")?)
        .item(&item(SHOW_MODULE_DIAGRAM, "Module Diagram", NO_KEY)?)
        .item(
            &MenuItemBuilder::with_id(BUILD_REPORT, "Board Utilization…")
                .enabled(is_enabled(BUILD_REPORT, &state))
                .build(app)?,
        )
        .separator()
        .item(&item(STOP, "Stop", NO_KEY)?)
        .separator()
        // As in the Arduino IDE (Sketch > Show Sketch Folder).
        .item(&item(SHOW_FOLDER, "Show Project Folder", NO_KEY)?)
        .item(
            &MenuItemBuilder::with_id(COPY_PATH, "Copy Project Folder Path")
                .enabled(is_enabled(COPY_PATH, &state))
                .build(app)?,
        )
        .build()?;

    // As in the Arduino IDE: Tools > Auto Format, Cmd+T; then the board's
    // options, as Arduino's board options.
    let tools = SubmenuBuilder::with_id(app, TOOLS_MENU, "Tools")
        .item(&item(AUTO_FORMAT, "Auto Format", "CmdOrCtrl+T")?)
        .separator()
        .items(&refs(&variant_entries(app, &state)?));
    let tools = tools
        .item(&MenuItemBuilder::with_id(BOARD_EXPLORER, "Board Explorer…").build(app)?)
        .item(&MenuItemBuilder::with_id(DEVICES, "Connected Devices…").build(app)?)
        .item(&item(
            SERIAL_MONITOR,
            "Serial Monitor",
            "CmdOrCtrl+Shift+M",
        )?)
        .build()?;

    let mut menu = MenuBuilder::new(app);
    if mac {
        let app_menu = SubmenuBuilder::new(app, "Logix IDE")
            .about(Some(about.clone()))
            .separator()
            .item(&settings_item)
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .item(&quit)
            .build()?;
        menu = menu.item(&app_menu);
    }
    let mut help = SubmenuBuilder::new(app, "Help");
    for (i, link) in HELP_LINKS.iter().enumerate() {
        help = match link {
            Some((label, _)) => help
                .item(&MenuItemBuilder::with_id(format!("{HELP_PREFIX}{i}"), *label).build(app)?),
            None => help.separator(),
        };
    }
    help = help
        .separator()
        .item(&MenuItemBuilder::with_id(CHECK_UPDATES, "Check for Updates").build(app)?)
        .item(&MenuItemBuilder::with_id(SYSTEM_INFO, "System Info…").build(app)?)
        .item(&MenuItemBuilder::with_id(SHOW_LOGS, "Show Log Folder").build(app)?);
    if !mac {
        help = help.separator().about(Some(about));
    }
    let help = help.build()?;

    menu.item(&file)
        .item(&edit)
        .item(&view)
        .item(&project)
        .item(&tools)
        .item(&help)
        .build()
}

/// The window in front, when it's one of Logix IDE's own other windows
/// (Waveforms, Diagram), not the main window.
fn other_window_in_front<R: Runtime>(app: &AppHandle<R>) -> Option<tauri::WebviewWindow<R>> {
    app.webview_windows()
        .into_values()
        .find(|w| w.label() != "main" && w.is_focused().unwrap_or(false))
}

/// Brings the main window to the front, so what a menu command shows there
/// (a dialog, the output) isn't hidden behind the Waveforms or Diagram
/// window (the user's report, 2026-11-21).
fn bring_main_forward<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

/// Whether a command shows something in the main window, so the main window
/// comes to the front first. Not Simulate and the diagram commands, which
/// bring their own windows up, and not the text commands, which act where
/// the person is.
pub fn shows_in_main(id: &str) -> bool {
    !matches!(
        id,
        SIMULATE
            | SIMULATE_THIS_TEST
            | SHOW_DIAGRAM
            | SHOW_MODULE_DIAGRAM
            | SELECT_ALL
            | TOGGLE_COMMENT
            | GO_TO_DEFINITION
            | OUTPUT_SELECT_ALL
            | SHOW_FOLDER
    )
}

pub fn on_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    // On macOS the menu bar is shared by every window, so Close Project (⌘W)
    // and Close Window (⇧⌘W) in front of a Waveforms or Diagram window close
    // that window, not the project or the app.
    if matches!(id, CLOSE | CLOSE_PROJECT) {
        if let Some(window) = other_window_in_front(app) {
            let _ = window.close();
            return;
        }
    }
    match id {
        // Both go through CloseRequested, which checks for unsaved changes.
        // Closing the only window quits the app.
        CLOSE | QUIT => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        NEW | CLOSE_PROJECT | OPEN | SAVE | SAVE_AS | NEW_FILE | EXAMPLES | VERIFY | UPLOAD
        | DEBUG | DEBUG_NETS | RUN_TESTS | AUTO_FORMAT | TOGGLE_OUTPUT | SIMULATE | SHOW_FOLDER
        | COPY_PATH | STOP | SHOW_DIAGRAM | DEVICES | SERIAL_MONITOR | SETTINGS
        | GO_TO_DEFINITION | GO_TO_FILE | TOGGLE_COMMENT | SELECT_ALL | OUTPUT_SELECT_ALL
        | CLEAR_OUTPUT | RUN_THIS_TEST | SIMULATE_THIS_TEST | SHOW_MODULE_DIAGRAM
        | BOARD_EXPLORER | SYSTEM_INFO | CHECK_UPDATES | BUILD_REPORT => {
            if shows_in_main(id) {
                bring_main_forward(app);
            }
            let _ = app.emit("menu", id);
        }
        SHOW_LOGS => {
            // This session's folder, with every program's log in it.
            let shown = crate::logging::session()
                .or_else(crate::logging::dir)
                .zip(platform::current())
                .map(|(dir, p)| p.reveal_folder(&dir));
            if let Some(Err(e)) = shown {
                crate::log!("Can't show the log folder: {e:#}");
            }
        }
        CLEAR_RECENT => {
            if let Ok(path) = settings::default_path() {
                let _ = settings::clear_recent(&path);
            }
            refresh(app);
            let _ = app.emit("menu", RECENTS_CHANGED);
        }
        _ if id.starts_with(VARIANT_PREFIX) => {
            bring_main_forward(app);
            let _ = app.emit("menu", id);
        }
        _ if id.starts_with(HELP_PREFIX) => {
            let url = id[HELP_PREFIX.len()..]
                .parse::<usize>()
                .ok()
                .and_then(|i| HELP_LINKS.get(i).copied().flatten())
                .map(|(_, url)| url);
            if let (Some(url), Some(p)) = (url, platform::current()) {
                if let Err(e) = p.open_url(url) {
                    crate::log!("Can't open {url}: {e:#}");
                }
            }
        }
        _ => {
            // Open Recent: tell the UI which project folder to open.
            let index = id
                .strip_prefix(RECENT_PREFIX)
                .and_then(|i| i.parse::<usize>().ok());
            if let Some(dir) = index.and_then(|i| recent_projects().into_iter().nth(i)) {
                let _ = app.emit("open-recent", dir.to_string_lossy().into_owned());
            }
        }
    }
}

/// Shows the editor's right-click menu at the mouse, like the Arduino IDE's
/// (a shorter one). Its items go to the UI as `menu` events, as in the menu
/// bar; Cut, Copy, Paste and Select All are the system's, so pasting works
/// as from the Edit menu. Go to Definition needs live checking.
pub fn show_editor_menu<R: Runtime>(
    window: &WebviewWindow<R>,
    state: &MenuState,
    module: Option<&str>,
) -> tauri::Result<()> {
    let item = |id: &str, text: &str, accel: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accel)
            .enabled(is_enabled(id, state))
            .build(window)
    };
    let menu = MenuBuilder::new(window)
        .item(&item(GO_TO_DEFINITION, "Go to Definition", "F12")?)
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item(TOGGLE_COMMENT, "Comment/Uncomment", "CmdOrCtrl+/")?)
        .item(&item(AUTO_FORMAT, "Auto Format", "CmdOrCtrl+T")?);
    // As on the tab's right-click menu: run or simulate this testbench.
    let menu = if state.project && state.testbench_open {
        menu.separator()
            .item(&MenuItemBuilder::with_id(RUN_THIS_TEST, "Run Test").build(window)?)
            .item(&MenuItemBuilder::with_id(SIMULATE_THIS_TEST, "Simulate Test").build(window)?)
    } else {
        menu
    };
    // In a design file: the diagram of the module at the cursor.
    let menu = match module.filter(|_| state.project && !state.testbench_open) {
        Some(_) => menu
            .separator()
            .item(&MenuItemBuilder::with_id(SHOW_MODULE_DIAGRAM, "Module Diagram").build(window)?),
        None => menu,
    };
    let menu = menu.build()?;
    menu.popup(window.as_ref().window())
}

/// Shows the output pane's right-click menu at the mouse: Copy (the
/// system's, for the selected text), Select All (the output only) and
/// Clear Output. The last two come back to the UI as `menu` events.
pub fn show_output_menu<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(window)
        .copy()
        .item(
            &MenuItemBuilder::with_id(OUTPUT_SELECT_ALL, "Select All")
                .accelerator("CmdOrCtrl+A")
                .build(window)?,
        )
        .separator()
        .item(&MenuItemBuilder::with_id(CLEAR_OUTPUT, "Clear Output").build(window)?)
        .build()?;
    menu.popup(window.as_ref().window())
}

/// Whether two menu states give the menu bar the same items, so going from
/// one to the other only changes which are enabled or checked.
pub fn same_items(a: &MenuState, b: &MenuState) -> bool {
    /// A submenu's ID and label, and each value's.
    type Shape<'a> = Vec<(&'a str, &'a str, Vec<(&'a str, &'a str)>)>;
    fn shape(s: &MenuState) -> Shape<'_> {
        s.variant_menus
            .iter()
            .map(|d| {
                let values = d.values.iter().map(|v| (v.id.as_str(), v.label.as_str()));
                (d.id.as_str(), d.label.as_str(), values.collect())
            })
            .collect()
    }
    shape(a) == shape(b)
}

/// Whether an item's enabled state follows the menu state.
fn follows_state(id: &str) -> bool {
    NEEDS_PROJECT.contains(&id)
        || matches!(
            id,
            AUTO_FORMAT
                | TOGGLE_COMMENT
                | GO_TO_DEFINITION
                | RUN_TESTS
                | SIMULATE
                | RUN_THIS_TEST
                | SIMULATE_THIS_TEST
                | SHOW_MODULE_DIAGRAM
        )
}

/// Updates the menu bar's items in place to `state`: which are enabled,
/// and the variant items' check marks. Replacing the whole menu bar
/// redraws the whole window on Windows, so the window flickered when the
/// toolbar's Flash/RAM choice changed (the user's report, 2026-11-14).
/// Returns false when there's no menu bar to update.
pub fn update<R: Runtime>(app: &AppHandle<R>, state: &MenuState) -> bool {
    fn walk<R: Runtime>(items: Vec<MenuItemKind<R>>, state: &MenuState) {
        for item in items {
            match item {
                MenuItemKind::Submenu(sub) => {
                    if let Ok(children) = sub.items() {
                        walk(children, state);
                    }
                }
                MenuItemKind::MenuItem(i) => {
                    let id = i.id().0.as_str();
                    if follows_state(id) {
                        let _ = i.set_enabled(is_enabled(id, state));
                    }
                }
                MenuItemKind::Check(c) => {
                    let id = c.id().0.clone();
                    let value = id.strip_prefix(VARIANT_PREFIX).and_then(|rest| {
                        let (dim, value) = rest.split_once(':')?;
                        state
                            .variant_menus
                            .iter()
                            .find(|d| d.id == dim)?
                            .values
                            .iter()
                            .find(|v| v.id == value)
                    });
                    if let Some(v) = value {
                        let _ = c.set_checked(v.checked);
                        let _ = c.set_enabled(v.enabled);
                    }
                }
                _ => {}
            }
        }
    }
    let Some(menu) = app.menu() else { return false };
    let Ok(items) = menu.items() else {
        return false;
    };
    walk(items, state);
    true
}

/// Brings the menu bar up to date after its contents changed: the recent
/// projects, the board options, or the About text (the engine release).
/// Those parts are replaced in place (`replace_parts`): replacing the whole
/// menu bar made GTK warn about every shortcut of the old one ("no
/// accelerator installed", from the menu library tearing it down; the
/// user's report, 2026-11-16) and redraws the whole window on Windows. The
/// whole menu bar is rebuilt only when it isn't laid out as expected.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let state = app
        .try_state::<crate::AppState>()
        .map(|s| s.menu_state.lock().unwrap().clone())
        .unwrap_or_default();
    if let Some(menu) = app.menu() {
        match replace_parts(app, &menu, &state) {
            Ok(()) => {
                update(app, &state);
                return;
            }
            Err(e) => crate::log!("Can't update the menu in place, rebuilding it: {e}"),
        }
    }
    match build(app) {
        Ok(menu) => {
            let _ = app.set_menu(menu);
        }
        Err(e) => crate::log!("Can't rebuild the menu: {e}"),
    }
}

/// Replaces the parts of the menu bar that change, none of which has a
/// shortcut: File > Open Recent's items, Tools' board options (between
/// Auto Format's separator and Board Explorer), and the About item.
fn replace_parts<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Menu<R>,
    state: &MenuState,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let top = menu.items()?;
    let recent = find_submenu(&top, OPEN_RECENT).context("no Open Recent menu")?;
    let tools = find_submenu(&top, TOOLS_MENU).context("no Tools menu")?;
    let tool_items = tools.items()?;
    let explorer = tool_items
        .iter()
        .position(|i| i.id().0 == BOARD_EXPLORER)
        .context("no Board Explorer item")?;
    // Auto Format and its separator stay.
    let first = 2;
    anyhow::ensure!(explorer >= first, "Tools isn't laid out as expected");
    // The About item: a predefined one, in the app menu (macOS) or Help.
    let about = top
        .iter()
        .filter_map(|i| match i {
            MenuItemKind::Submenu(s) => Some(s.clone()),
            _ => None,
        })
        .find_map(|sub| {
            let items = sub.items().ok()?;
            let at = items.iter().position(|i| match i {
                MenuItemKind::Predefined(p) => p.text().is_ok_and(|t| t.starts_with("About")),
                _ => false,
            })?;
            Some((sub, at))
        });

    for _ in 0..recent.items()?.len() {
        recent.remove_at(0)?;
    }
    recent.append_items(&refs(&recent_entries(app)?))?;
    for _ in first..explorer {
        tools.remove_at(first)?;
    }
    tools.insert_items(&refs(&variant_entries(app, state)?), first)?;
    if let Some((sub, at)) = about {
        sub.remove_at(at)?;
        sub.insert(
            &PredefinedMenuItem::about(app, None::<&str>, Some(about_metadata()))?,
            at,
        )?;
    }
    Ok(())
}

/// The submenu with this ID, anywhere in the menu bar.
fn find_submenu<R: Runtime>(items: &[MenuItemKind<R>], id: &str) -> Option<Submenu<R>> {
    items.iter().find_map(|i| match i {
        MenuItemKind::Submenu(sub) if sub.id().0 == id => Some(sub.clone()),
        MenuItemKind::Submenu(sub) => find_submenu(&sub.items().ok()?, id),
        _ => None,
    })
}

pub fn recent_projects() -> Vec<PathBuf> {
    settings::default_path()
        .map(|p| settings::load(&p).recent_projects)
        .unwrap_or_default()
}

/// Menu labels for recent projects: the folder name, plus the parent
/// folder's name when two projects have the same name.
fn recent_labels(dirs: &[PathBuf]) -> Vec<String> {
    let name = |p: &Path| crate::project::project_name(p);
    dirs.iter()
        .map(|dir| {
            let n = name(dir);
            let shared = dirs.iter().filter(|d| name(d) == n).count() > 1;
            match dir.parent().filter(|_| shared) {
                Some(parent) => format!("{n} — {}", name(parent)),
                None => n,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {

    #[test]
    fn commands_that_show_in_the_main_window_bring_it_forward() {
        for id in [
            SYSTEM_INFO,
            SETTINGS,
            VERIFY,
            EXAMPLES,
            DEVICES,
            CHECK_UPDATES,
        ] {
            assert!(shows_in_main(id), "{id}");
        }
        // Their own windows come up instead, or they act where the person is.
        for id in [
            SIMULATE,
            SHOW_DIAGRAM,
            SHOW_MODULE_DIAGRAM,
            SELECT_ALL,
            TOGGLE_COMMENT,
        ] {
            assert!(!shows_in_main(id), "{id}");
        }
    }

    use super::*;

    #[test]
    fn a_new_check_mark_keeps_the_items() {
        let memory = |flash: bool, ram_enabled: bool| MenuState {
            project: true,
            variant_menus: vec![VariantMenu {
                id: "memory".into(),
                label: "Memory".into(),
                values: vec![
                    VariantMenuValue {
                        id: "flash".into(),
                        label: "Flash".into(),
                        checked: flash,
                        enabled: true,
                    },
                    VariantMenuValue {
                        id: "ram".into(),
                        label: "RAM".into(),
                        checked: !flash,
                        enabled: ram_enabled,
                    },
                ],
            }],
            ..Default::default()
        };
        // Flash to RAM, or a greyed item: updated in place.
        assert!(same_items(&memory(true, true), &memory(false, true)));
        assert!(same_items(&memory(true, true), &memory(true, false)));
        // Another board's variants (or none): rebuilt.
        assert!(!same_items(&memory(true, true), &MenuState::default()));
        assert!(same_items(
            &MenuState::default(),
            &MenuState {
                project: true,
                ..Default::default()
            }
        ));
        // Only items that follow the state are touched.
        assert!(follows_state(SIMULATE) && follows_state(SAVE));
        assert!(!follows_state(OPEN) && !follows_state(CLEAR_RECENT));
    }

    #[test]
    fn help_links_are_web_pages() {
        for (label, url) in HELP_LINKS.iter().flatten() {
            assert!(url.starts_with("https://"), "{label}: {url}");
        }
    }

    #[test]
    fn labels_tell_same_named_projects_apart() {
        let dirs = [
            PathBuf::from("/Users/a/Documents/OpenLogix/IDE/blinky"),
            PathBuf::from("/Users/a/work/blinky"),
            PathBuf::from("/Users/a/Documents/OpenLogix/IDE/counter"),
        ];
        assert_eq!(
            recent_labels(&dirs),
            ["blinky — IDE", "blinky — work", "counter"]
        );
    }

    #[test]
    fn board_options_say_whose_they_are() {
        assert_eq!(variant_menu_title("Memory"), "Board Memory");
        assert_eq!(variant_menu_title("Connection"), "Board Connection");
    }

    #[test]
    fn items_follow_the_menu_state() {
        let none = MenuState::default();
        assert!(!is_enabled(SAVE, &none));
        assert!(is_enabled(OPEN, &none));
        assert!(is_enabled(SETTINGS, &none));
        assert!(
            is_enabled(BOARD_EXPLORER, &none),
            "with no project or board too"
        );
        // A pin file open in a project without testbenches.
        let pins = MenuState {
            project: true,
            ..Default::default()
        };
        for id in [
            SAVE,
            VERIFY,
            UPLOAD,
            DEBUG,
            DEBUG_NETS,
            SHOW_DIAGRAM,
            NEW_FILE,
        ] {
            assert!(is_enabled(id, &pins), "{id}");
        }
        for id in [
            AUTO_FORMAT,
            TOGGLE_COMMENT,
            GO_TO_DEFINITION,
            RUN_TESTS,
            SIMULATE,
            RUN_THIS_TEST,
            SIMULATE_THIS_TEST,
            SHOW_MODULE_DIAGRAM,
        ] {
            assert!(!is_enabled(id, &pins), "{id}");
        }
        // A pin file has line comments, but no formatter.
        let pin_file = MenuState {
            project: true,
            comment: true,
            ..Default::default()
        };
        assert!(is_enabled(TOGGLE_COMMENT, &pin_file));
        assert!(!is_enabled(AUTO_FORMAT, &pin_file));
        let all = MenuState {
            project: true,
            formattable: true,
            comment: true,
            definition: true,
            testbenches: true,
            simulate: true,
            testbench_open: true,
            module_open: true,
            variant_menus: Vec::new(),
        };
        for id in [
            AUTO_FORMAT,
            TOGGLE_COMMENT,
            GO_TO_DEFINITION,
            RUN_TESTS,
            SIMULATE,
            BUILD_REPORT,
            RUN_THIS_TEST,
            SIMULATE_THIS_TEST,
            SHOW_MODULE_DIAGRAM,
        ] {
            assert!(is_enabled(id, &all), "{id}");
        }
        // Nothing works without a project, whatever else is set.
        let stale = MenuState {
            project: false,
            ..all
        };
        assert!(!is_enabled(AUTO_FORMAT, &stale));
    }
}
