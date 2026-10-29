//! The native menu bar. Menu items that act on the document (New, Open,
//! Save, ...) are forwarded to the UI as `menu` events with the item id;
//! Close and Quit are handled here, through the normal window-close flow.

use std::path::{Path, PathBuf};

use tauri::menu::{AboutMetadata, ContextMenu, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

use crate::{platform, settings};

/// Menu ids forwarded to the UI (see `onMenu` in `src/lib/backend.ts`).
pub const NEW: &str = "new";
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
pub const SHOW_DIAGRAM: &str = "show-diagram";
pub const DEVICES: &str = "devices";
pub const SETTINGS: &str = "settings";
pub const GO_TO_DEFINITION: &str = "go-to-definition";
pub const TOGGLE_COMMENT: &str = "toggle-comment";
/// Edit > Select All: the UI selects all of what has the focus (editor,
/// output pane, or a text field), not the whole window.
pub const SELECT_ALL: &str = "select-all";
/// The output pane's right-click menu.
pub const OUTPUT_SELECT_ALL: &str = "output-select-all";
pub const CLEAR_OUTPUT: &str = "clear-output";
/// Help menu items are "help:<index into HELP_LINKS>".
const HELP_PREFIX: &str = "help:";

/// Help menu: (label, web page). `None` marks a separator.
const HELP_LINKS: &[Option<(&str, &str)>] = &[
    Some(("Logix Documentation", "https://openlogix.github.io/logix-ide/")),
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
    // For going further: Logix uses Apio, and its docs cover the tools.
    Some((
        "Apio Documentation",
        "https://fpgawars.github.io/apio/docs/",
    )),
    None,
    Some(("Logix Website", "https://www.openlogix.org/ide/")),
    Some(("Logix on GitHub", "https://github.com/openlogix/logix-ide")),
    Some((
        "Report a Problem",
        // A new issue whose text asks for the log (see SHOW_LOGS).
        "https://github.com/openlogix/logix-ide/issues/new?body=What%20happened%2C%20and%20what%20did%20you%20expect%3F%0A%0A%0ALogix%20log%3A%20in%20Logix%2C%20choose%20Help%20%3E%20Show%20Log%20Folder%20and%20drag%20logix.log%20here.%0A",
    )),
];
pub const STOP: &str = "stop";
/// Open Recent items are "recent:<index into the recent projects>".
const RECENT_PREFIX: &str = "recent:";
const CLEAR_RECENT: &str = "clear-recent";
/// Help > Show Log Folder: the Logix log files (see `logging.rs`).
const SHOW_LOGS: &str = "show-logs";
const CLOSE: &str = "close";
const QUIT: &str = "quit";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, text: &str, accel: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accel)
            .build(app)
    };
    let quit = item(QUIT, "Quit Logix IDE", "CmdOrCtrl+Q")?;
    let settings_item = item(SETTINGS, "Settings…", "CmdOrCtrl+,")?;
    let about = AboutMetadata {
        name: Some("Logix IDE".into()),
        version: Some(app.package_info().version.to_string()),
        comments: Some(format!(
            "\"The Arduino experience, for FPGAs.\"\nPowered by Apio (release {}).\n\nLogix is not affiliated with or endorsed by Arduino. Arduino is a trademark of Arduino SA.",
            crate::apio::APIO_RELEASE_TAG
        )),
        copyright: Some("Licensed under the Apache License 2.0.".into()),
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

    let mut file = SubmenuBuilder::new(app, "File")
        .item(&item(NEW, "New", "CmdOrCtrl+N")?)
        .item(&item(OPEN, "Open…", "CmdOrCtrl+O")?)
        .item(&recent)
        .item(&item(EXAMPLES, "Examples…", "CmdOrCtrl+Shift+E")?)
        .separator()
        .item(&item(NEW_FILE, "New File…", "CmdOrCtrl+Shift+N")?)
        .separator()
        .item(&item(SAVE, "Save", "CmdOrCtrl+S")?)
        .item(&item(SAVE_AS, "Save As…", "CmdOrCtrl+Shift+S")?)
        .separator()
        .item(&item(CLOSE, "Close Window", "CmdOrCtrl+W")?);
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
        .separator()
        .item(&item(STOP, "Stop", "CmdOrCtrl+.")?)
        .separator()
        // As in the Arduino IDE (Sketch > Show Sketch Folder).
        .item(&item(
            SHOW_FOLDER,
            "Show Project Folder",
            "CmdOrCtrl+Alt+K",
        )?)
        .build()?;

    // As in the Arduino IDE: Tools > Auto Format, Cmd+T.
    let tools = SubmenuBuilder::new(app, "Tools")
        .item(&item(AUTO_FORMAT, "Auto Format", "CmdOrCtrl+T")?)
        .separator()
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
        NEW | OPEN | SAVE | SAVE_AS | NEW_FILE | EXAMPLES | VERIFY | UPLOAD | RUN_TESTS
        | AUTO_FORMAT | TOGGLE_OUTPUT | SIMULATE | SHOW_FOLDER | STOP | SHOW_DIAGRAM | DEVICES
        | SETTINGS | GO_TO_DEFINITION | TOGGLE_COMMENT | SELECT_ALL | OUTPUT_SELECT_ALL
        | CLEAR_OUTPUT => {
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
    can_go_to_definition: bool,
) -> tauri::Result<()> {
    let item = |id: &str, text: &str, accel: &str, enabled: bool| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accel)
            .enabled(enabled)
            .build(window)
    };
    let menu = MenuBuilder::new(window)
        .item(&item(
            GO_TO_DEFINITION,
            "Go to Definition",
            "F12",
            can_go_to_definition,
        )?)
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item(
            TOGGLE_COMMENT,
            "Comment/Uncomment",
            "CmdOrCtrl+/",
            true,
        )?)
        .item(&item(AUTO_FORMAT, "Auto Format", "CmdOrCtrl+T", true)?)
        .build()?;
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

fn recent_projects() -> Vec<PathBuf> {
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
}
