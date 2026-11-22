//! The Diagram window (Project > Show Diagram): Logix Engine's diagram page
//! (`diagram` tool, `diagram/diagram.html`) in a window of its own, one per
//! project, movable, resizable and not modal, so the user keeps working
//! while it's open. It's redrawn after each Verify.
//!
//! The page is the engine's, as is: self-contained, with every module,
//! drill-down, pan, zoom and hover. Logix IDE keeps it in memory and serves
//! it on the `logix-diagram` scheme (the build folder is cleared before
//! each task, so the file doesn't last). The page gets no Tauri access: it
//! asks through the page contract, `window.logixHost` (`page_host.rs`,
//! shared with the Waveforms window), which `serve` answers: a block's
//! source line (`openSource`) becomes a `diagram-open-source` event to the
//! main window, only for a file of the project, and Export's images
//! (`saveAs`) go through a save dialog. Until 2026-11-22 a click navigated
//! to a `logix-open-source:` address that this window cancelled, and Export
//! couldn't save.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::page_host::{self, Call, Request};

/// The scheme the pages are served on.
pub const SCHEME: &str = "logix-diagram";
/// Window labels start with this.
const LABEL_PREFIX: &str = "diagram-";

/// A window's page and what it may open.
struct Page {
    html: String,
    /// The project's key, as the UI knows it.
    key: String,
    /// The project's file names: the only files a click may open.
    files: Vec<String>,
    /// The module the diagram starts at, when it isn't the top module.
    module: Option<String>,
}

static PAGES: Mutex<Option<HashMap<String, Page>>> = Mutex::new(None);

/// The window's label for a project: one window per project.
pub fn label(key: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    format!("{LABEL_PREFIX}{:016x}", h.finish())
}

/// Whether the project's Diagram window is open.
pub fn is_open(app: &AppHandle, key: &str) -> bool {
    app.get_webview_window(&label(key)).is_some()
}

/// The page's address.
fn page_url(label: &str) -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{label}")
    } else {
        format!("{SCHEME}://localhost/{label}")
    }
}

/// The module the project's diagram starts at, when it isn't the top
/// module: Verify draws the same one again.
pub fn module(key: &str) -> Option<String> {
    module_of(&label(key))
}

fn module_of(label: &str) -> Option<String> {
    PAGES.lock().unwrap().as_ref()?.get(label)?.module.clone()
}

/// What the main window gets when the user clicks a block.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSource {
    pub key: String,
    pub file: String,
    pub line: u32,
}

/// Shows the project's diagram: opens its window, or updates the open one.
/// `module` is the module the diagram starts at (None: the top module); a
/// diagram from the same module reloads in place, staying on the module it
/// shows. `focus` brings the window to the front.
pub fn show(
    app: &AppHandle,
    key: &str,
    title: &str,
    html: String,
    files: Vec<String>,
    module: Option<&str>,
    focus: bool,
) -> tauri::Result<()> {
    let label = label(key);
    let module = module.filter(|m| *m != "main").map(str::to_string);
    let before = PAGES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(
            label.clone(),
            Page {
                html,
                key: key.to_string(),
                files,
                module: module.clone(),
            },
        );
    if let Some(window) = app.get_webview_window(&label) {
        window.set_title(title)?;
        if before.is_some_and(|b| b.module == module) {
            window.eval("location.reload()")?;
        } else {
            window.navigate(page_url(&label).parse().expect("a valid URL"))?;
        }
        if focus {
            window.unminimize()?;
            window.set_focus()?;
        }
        return Ok(());
    }
    let url = page_url(&label).parse().expect("a valid URL");
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::CustomProtocol(url))
        .title(title)
        .inner_size(1000.0, 700.0)
        .min_inner_size(480.0, 320.0)
        .initialization_script(page_host::SCRIPT)
        .on_navigation(|url| {
            url.scheme() == SCHEME || url.host_str() == Some(&format!("{SCHEME}.localhost"))
        })
        .build()?;
    crate::commands::apply_stored_window_preferences(&window);
    Ok(())
}

/// A window's page, by its label.
fn page_html(label: &str) -> Option<String> {
    PAGES
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|p| p.get(label).map(|page| page.html.clone()))
}

/// The scheme's handler: a window's page (GET), or what its page asks
/// through `window.logixHost` (POST to `/<label>/<call>`, `page_host.rs`).
pub fn serve(app: &AppHandle, path: &str, body: &[u8]) -> tauri::http::Response<Vec<u8>> {
    let (label, what) = match page_host::request(path) {
        Request::Page(label) => {
            return match page_html(label) {
                Some(h) => page_host::respond(200, h.into_bytes()),
                None => page_host::respond(404, b"No diagram.".to_vec()),
            };
        }
        Request::Call(label, what) => (label, what),
    };
    let page = PAGES
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|p| p.get(label))
        .map(|p| (p.key.clone(), p.files.clone()));
    let Some((key, files)) = page else {
        return page_host::respond(404, Vec::new());
    };
    match page_host::call(what, body) {
        Ok(Call::OpenSource { file, line }) if files.contains(&file) => {
            crate::log!("Diagram: open {file}:{line}");
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.emit("diagram-open-source", OpenSource { key, file, line });
                let _ = main.set_focus();
            }
        }
        Ok(Call::OpenSource { .. }) => {
            crate::log!("Diagram: ignored a request to open a file outside the project")
        }
        Ok(Call::SaveAs { name, mime, data }) => {
            page_host::save_as(app, label, "Diagram", &name, &mime, &data)
        }
        Ok(Call::OpenUrl(url)) => page_host::open_url("Diagram", &url),
        // The engine declares no file for the diagram page.
        Ok(Call::WriteDeclared { id, .. }) => {
            crate::log!("Diagram: ignored a write of {id:?}, which isn't declared")
        }
        Err(e) => crate::log!("Diagram: ignored a request: {e}"),
    }
    page_host::respond(204, Vec::new())
}

/// Every open Diagram window.
pub fn windows(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(l, _)| l.starts_with(LABEL_PREFIX))
        .map(|(_, w)| w)
        .collect()
}

/// Whether a window is a Diagram window.
pub fn is_diagram(label: &str) -> bool {
    label.starts_with(LABEL_PREFIX)
}

/// Forgets a closed window's page.
pub fn forget(label: &str) {
    if let Some(p) = PAGES.lock().unwrap().as_mut() {
        p.remove(label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_window_per_project() {
        assert_eq!(label("/a/proj"), label("/a/proj"));
        assert_ne!(label("/a/proj"), label("/a/other"));
        assert!(label("x").starts_with("diagram-"));
        // Tauri's labels: letters, digits, -, /, :, _.
        assert!(label("a b/ü")
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    #[test]
    fn pages_are_served_by_window() {
        PAGES
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(
                "diagram-test".into(),
                Page {
                    html: "<html>ok</html>".into(),
                    key: "k".into(),
                    files: vec![],
                    module: Some("alu".into()),
                },
            );
        assert_eq!(module_of("diagram-test"), Some("alu".into()));
        assert_eq!(
            page_html("diagram-test").as_deref(),
            Some("<html>ok</html>")
        );
        assert_eq!(page_html("diagram-none"), None);
        forget("diagram-test");
        assert_eq!(page_html("diagram-test"), None);
        // WebView2 serves custom schemes as http://<scheme>.localhost/.
        let expected = if cfg!(windows) {
            "http://logix-diagram.localhost/diagram-1"
        } else {
            "logix-diagram://localhost/diagram-1"
        };
        assert_eq!(page_url("diagram-1"), expected);
    }
}
