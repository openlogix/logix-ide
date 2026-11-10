//! The native menu bar. Menu items that act on the document (New, Open,
//! Save, ...) are forwarded to the UI as `menu` events with the item id;
//! Close and Quit are handled here, through the normal window-close flow.

use std::path::{Path, PathBuf};

use tauri::menu::{
    AboutMetadata, CheckMenuItemBuilder, ContextMenu, Menu, MenuBuilder, MenuItemBuilder,
    SubmenuBuilder,
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
/// Tools > Board Explorer…: the FPGA families, FPGAs, boards and examples
/// Logix IDE knows, and how they relate, opened at the toolbar's board.
pub const BOARD_EXPLORER: &str = "board-explorer";
pub const SETTINGS: &str = "settings";
pub const GO_TO_DEFINITION: &str = "go-to-definition";
pub const TOGGLE_COMMENT: &str = "toggle-comment";
/// Edit > Select All: the UI selects all of what has the focus (editor,
/// output pane, or a text field), not the whole window.
pub const SELECT_ALL: &str = "select-all";
/// The output pane's right-click menu.
pub const OUTPUT_SELECT_ALL: &str = "output-select-all";
pub const CLEAR_OUTPUT: &str = "clear-output";
/// Editor right-click menu, in a testbench: run or simulate that testbench.
pub const RUN_THIS_TEST: &str = "run-this-test";
pub const SIMULATE_THIS_TEST: &str = "simulate-this-test";
/// Sent to the UI (as a `menu` event) when File > Open Recent > Clear Menu
/// emptied the list, so the start screen reloads its recent projects.
pub const RECENTS_CHANGED: &str = "recents-changed";
/// Help menu items are "help:<index into HELP_LINKS>".
const HELP_PREFIX: &str = "help:";

/// Help menu: (label, web page). `None` marks a separator.
const HELP_LINKS: &[Option<(&str, &str)>] = &[
    Some(("Logix IDE Documentation", "https://openlogix.github.io/logix-ide/")),
    Some(("Quick Start", "https://openlogix.github.io/logix-ide/quick-start/")),
    Some((
        "Testbenches and Simulation",
        "https://openlogix.github.io/logix-ide/testbenches/",
    )),
    Some((
        "Troubleshooting",
        "https://openlogix.github.io/logix-ide/troubleshooting/",
    )),
    Some((
        "Keyboard Shortcuts",
        "https://openlogix.github.io/logix-ide/using-logix/#keyboard-shortcuts",
    )),
    None,
    Some(("Logix IDE Website", "https://www.openlogix.org/ide/")),
    Some(("Logix IDE on GitHub", "https://github.com/openlogix/logix-ide")),
    Some((
        "Report a Problem",
        // A new issue whose text asks for the log (see SHOW_LOGS).
        "https://github.com/openlogix/logix-ide/issues/new?body=What%20happened%2C%20and%20what%20did%20you%20expect%3F%0A%0A%0ALogix%20log%3A%20in%20Logix%2C%20choose%20Help%20%3E%20Show%20Log%20Folder%20and%20drag%20logix.log%20here.%0A",
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
    VERIFY,
    UPLOAD,
    RUN_TESTS,
    SIMULATE,
    SHOW_DIAGRAM,
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
    /// There's a testbench to simulate: the open tab is one, or the project
    /// has exactly one (Simulate).
    pub simulate: bool,
    /// The open tab is a testbench: the editor's right-click menu adds Run
    /// This Test and Simulate (hidden otherwise, to keep it short).
    pub testbench_open: bool,
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
        _ => true,
    }
}
/// Open Recent items are "recent:<index into the recent projects>".
const RECENT_PREFIX: &str = "recent:";
const CLEAR_RECENT: &str = "clear-recent";
/// Help > Show Log Folder: the Logix IDE log files (see `logging.rs`).
const SHOW_LOGS: &str = "show-logs";
/// Help > System Info…: the UI asks for the report (`system_info`).
const SYSTEM_INFO: &str = "system-info";
const CLOSE: &str = "close";
const QUIT: &str = "quit";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let state = app
        .try_state::<crate::AppState>()
        .map(|s| s.menu_state.lock().unwrap().clone())
        .unwrap_or_default();
    let item = |id: &str, text: &str, accel: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accel)
            .enabled(is_enabled(id, &state))
            .build(app)
    };
    let quit = item(QUIT, "Quit Logix IDE", "CmdOrCtrl+Q")?;
    let settings_item = item(SETTINGS, "Settings…", "CmdOrCtrl+,")?;
    let about = AboutMetadata {
        name: Some("Logix IDE".into()),
        version: Some(app.package_info().version.to_string()),
        comments: Some(format!(
            "The Arduino experience, for FPGAs.\nPowered by Logix Engine (release {}).\n\nLogix IDE is not affiliated with or endorsed by Arduino. Arduino is a trademark of Arduino SA.",
            crate::engine::ENGINE_RELEASE_TAG
        )),
        copyright: Some("Licensed under MIT or the Apache License 2.0.".into()),
        website: Some("https://www.openlogix.org/ide/".into()),
        ..Default::default()
    };
    // macOS puts Quit in the application menu; others in File.
    let mac = platform::current().is_some_and(|p| p.has_app_menu());

    // File > Open Recent, from the recent projects in the settings.
    let recents = recent_projects();
    let mut recent = SubmenuBuilder::new(app, "Open Recent");
    for (i, label) in recent_labels(&recents).into_iter().enumerate() {
        recent = recent
            .item(&MenuItemBuilder::with_id(format!("{RECENT_PREFIX}{i}"), label).build(app)?);
    }
    if recents.is_empty() {
        recent = recent.item(
            &MenuItemBuilder::with_id("no-recent", "No Recent Projects")
                .enabled(false)
                .build(app)?,
        );
    }
    let recent = recent
        .separator()
        .item(
            &MenuItemBuilder::with_id(CLEAR_RECENT, "Clear Menu")
                .enabled(!recents.is_empty())
                .build(app)?,
        )
        .build()?;

    // "Project" in the names: these act on the whole project, New File on
    // one file in it.
    // Examples right below New Project, easy to find, as the Arduino IDE's
    // File > Examples.
    let mut file = SubmenuBuilder::new(app, "File")
        .item(&item(NEW, "New Project", "CmdOrCtrl+N")?)
        .item(&item(EXAMPLES, "Examples…", "CmdOrCtrl+Shift+E")?)
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
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item(TOGGLE_OUTPUT, "Toggle Output", "CmdOrCtrl+J")?)
        .build()?;

    // Same shortcuts as the Arduino IDE.
    let project = SubmenuBuilder::new(app, "Project")
        .item(&item(VERIFY, "Verify", "CmdOrCtrl+R")?)
        .item(&item(UPLOAD, "Upload", "CmdOrCtrl+U")?)
        .separator()
        .item(&item(RUN_TESTS, "Run Tests", "CmdOrCtrl+Shift+T")?)
        .item(&item(SIMULATE, "Simulate", "CmdOrCtrl+Alt+T")?)
        .separator()
        .item(&item(SHOW_DIAGRAM, "Show Diagram", "CmdOrCtrl+Shift+D")?)
        .item(
            &MenuItemBuilder::with_id(BUILD_REPORT, "Board Utilization…")
                .enabled(is_enabled(BUILD_REPORT, &state))
                .build(app)?,
        )
        .separator()
        .item(&item(STOP, "Stop", "CmdOrCtrl+.")?)
        .separator()
        // As in the Arduino IDE (Sketch > Show Sketch Folder).
        .item(&item(
            SHOW_FOLDER,
            "Show Project Folder",
            "CmdOrCtrl+Alt+K",
        )?)
        .item(
            &MenuItemBuilder::with_id(COPY_PATH, "Copy Project Folder Path")
                .enabled(is_enabled(COPY_PATH, &state))
                .build(app)?,
        )
        .build()?;

    // As in the Arduino IDE: Tools > Auto Format, Cmd+T; then the board's
    // options, as Arduino's board options.
    let mut tools = SubmenuBuilder::new(app, "Tools")
        .item(&item(AUTO_FORMAT, "Auto Format", "CmdOrCtrl+T")?)
        .separator();
    for dim in &state.variant_menus {
        let mut sub = SubmenuBuilder::new(app, &dim.label);
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
        tools = tools.item(&sub.build()?);
    }
    if !state.variant_menus.is_empty() {
        tools = tools.separator();
    }
    let tools = tools
        .item(&MenuItemBuilder::with_id(BOARD_EXPLORER, "Board Explorer…").build(app)?)
        .item(&MenuItemBuilder::with_id(DEVICES, "Connected Devices…").build(app)?)
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

pub fn on_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        // Both go through CloseRequested, which checks for unsaved changes.
        // Closing the only window quits the app.
        CLOSE | QUIT => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        NEW | CLOSE_PROJECT | OPEN | SAVE | SAVE_AS | NEW_FILE | EXAMPLES | VERIFY | UPLOAD
        | RUN_TESTS | AUTO_FORMAT | TOGGLE_OUTPUT | SIMULATE | SHOW_FOLDER | COPY_PATH | STOP
        | SHOW_DIAGRAM | DEVICES | SETTINGS | GO_TO_DEFINITION | TOGGLE_COMMENT | SELECT_ALL
        | OUTPUT_SELECT_ALL | CLEAR_OUTPUT | RUN_THIS_TEST | SIMULATE_THIS_TEST
        | BOARD_EXPLORER | SYSTEM_INFO | BUILD_REPORT => {
            let _ = app.emit("menu", id);
        }
        SHOW_LOGS => {
            let shown = crate::logging::dir()
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
            .item(&MenuItemBuilder::with_id(RUN_THIS_TEST, "Run This Test").build(window)?)
            .item(&MenuItemBuilder::with_id(SIMULATE_THIS_TEST, "Simulate").build(window)?)
    } else {
        menu
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

/// Rebuilds the menu bar, e.g. after the recent projects changed.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    match build(app) {
        Ok(menu) => {
            let _ = app.set_menu(menu);
        }
        Err(e) => crate::log!("Can't rebuild the menu: {e}"),
    }
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
    use super::*;

    #[test]
    fn help_links_are_web_pages() {
        for (label, url) in HELP_LINKS.iter().flatten() {
            assert!(url.starts_with("https://"), "{label}: {url}");
        }
    }

    #[test]
    fn labels_tell_same_named_projects_apart() {
        let dirs = [
            PathBuf::from("/Users/a/Documents/Logix/blinky"),
            PathBuf::from("/Users/a/work/blinky"),
            PathBuf::from("/Users/a/Documents/Logix/counter"),
        ];
        assert_eq!(
            recent_labels(&dirs),
            ["blinky — Logix", "blinky — work", "counter"]
        );
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
        for id in [SAVE, VERIFY, UPLOAD, SHOW_DIAGRAM, NEW_FILE] {
            assert!(is_enabled(id, &pins), "{id}");
        }
        for id in [
            AUTO_FORMAT,
            TOGGLE_COMMENT,
            GO_TO_DEFINITION,
            RUN_TESTS,
            SIMULATE,
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
            variant_menus: Vec::new(),
        };
        for id in [
            AUTO_FORMAT,
            TOGGLE_COMMENT,
            GO_TO_DEFINITION,
            RUN_TESTS,
            SIMULATE,
            BUILD_REPORT,
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
