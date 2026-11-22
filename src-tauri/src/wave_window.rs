//! The waveform viewer's window (Project > Simulate; Logix Engine's
//! docs/requirements.md, Waveform viewer): Logix Engine's viewer page
//! (`simulate`'s `viewer_file`) in a window of its own, one per project,
//! movable, resizable and not modal, as the Diagram window. Each Simulate
//! shows its page there, reloaded in place (R6): the page reads the
//! testbench's `.view`, a project file, so the signals and the view stay.
//!
//! The page is the engine's, as is, kept in memory and served on the
//! `logix-wave` scheme (the build folder is cleared before each task). It
//! gets no Tauri access: it asks through the page contract,
//! `window.logixHost` (`page_host.rs`, shared with the Diagram window),
//! which `serve` answers: the view (`writeDeclared("view", …)`) becomes a
//! `wave-view` event to the main window (a project file, as an edit), an
//! image (`saveAs`) goes through a save dialog, and a source line
//! (`openSource`) becomes a `wave-open-source` event, only for a file of the
//! project.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::page_host::{self, Call, Request};

/// The scheme the pages are served on.
pub const SCHEME: &str = "logix-wave";
/// Window labels start with this.
const LABEL_PREFIX: &str = "wave-";

/// A window's page and what it may do.
struct Page {
    html: String,
    /// The project's key, as the UI knows it.
    key: String,
    /// The project's file names: the only files a click may open.
    files: Vec<String>,
    /// The testbench's view file, such as main_tb.view.
    view_file: String,
}

static PAGES: Mutex<Option<HashMap<String, Page>>> = Mutex::new(None);

/// The window's label for a project: one window per project.
pub fn label(key: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    format!("{LABEL_PREFIX}{:016x}", h.finish())
}

/// The page's address.
fn page_url(label: &str) -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{label}")
    } else {
        format!("{SCHEME}://localhost/{label}")
    }
}

/// The view file of a testbench: `main_tb.v` -> `main_tb.view`.
pub fn view_file(testbench: &str) -> String {
    let stem = std::path::Path::new(testbench)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| testbench.to_string());
    format!("{stem}.view")
}

/// What the main window gets when the page saves the view: the project
/// file's new text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedView {
    pub key: String,
    pub name: String,
    pub text: String,
}

/// What the main window gets when the user asks for a source line.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSource {
    pub key: String,
    pub file: String,
    pub line: u32,
}

/// Shows a project's simulation: opens its window, or reloads the open one
/// in place with the new page. `focus` brings the window to the front.
pub fn show(
    app: &AppHandle,
    key: &str,
    title: &str,
    html: String,
    files: Vec<String>,
    testbench: &str,
    focus: bool,
) -> tauri::Result<()> {
    let label = label(key);
    PAGES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(
            label.clone(),
            Page {
                html,
                key: key.to_string(),
                files,
                view_file: view_file(testbench),
            },
        );
    if let Some(window) = app.get_webview_window(&label) {
        window.set_title(title)?;
        window.navigate(page_url(&label).parse().expect("a valid URL"))?;
        if focus {
            window.unminimize()?;
            window.set_focus()?;
        }
        return Ok(());
    }
    let url = page_url(&label).parse().expect("a valid URL");
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::CustomProtocol(url))
        .title(title)
        .inner_size(1100.0, 700.0)
        .min_inner_size(520.0, 320.0)
        .initialization_script(page_host::SCRIPT)
        // The page's own drag and drop (reordering rows): Tauri's file-drop
        // handling would take every drag as a file drop, with a "+" cursor
        // and no drop for the page (the user's report, 2026-11-21).
        .disable_drag_drop_handler()
        .on_navigation(|url| {
            url.scheme() == SCHEME || url.host_str() == Some(&format!("{SCHEME}.localhost"))
        })
        .build()?;
    crate::commands::apply_stored_window_preferences(&window);
    Ok(())
}

/// The scheme's handler: a window's page (GET), or what its page asks
/// through `window.logixHost` (POST to `/<label>/<call>`, `page_host.rs`).
pub fn serve(app: &AppHandle, path: &str, body: &[u8]) -> tauri::http::Response<Vec<u8>> {
    let (label, what) = match page_host::request(path) {
        Request::Page(label) => {
            let html = PAGES
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|p| p.get(label).map(|page| page.html.clone()));
            return match html {
                Some(h) => page_host::respond(200, h.into_bytes()),
                None => page_host::respond(404, b"No waveform.".to_vec()),
            };
        }
        Request::Call(label, what) => (label, what),
    };
    let page = PAGES
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|p| p.get(label))
        .map(|p| (p.key.clone(), p.files.clone(), p.view_file.clone()));
    let Some((key, files, view_name)) = page else {
        return page_host::respond(404, Vec::new());
    };
    let main = app.get_webview_window("main");
    match page_host::call(what, body) {
        // The one file the engine declared for this page: the testbench's
        // view, pretty-printed for people reading the project's files.
        Ok(Call::WriteDeclared { id, text }) if id == "view" => {
            let text = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .filter(serde_json::Value::is_object)
                .and_then(|v| serde_json::to_string_pretty(&v).ok());
            match (text, main) {
                (Some(text), Some(main)) => {
                    let _ = main.emit(
                        "wave-view",
                        SavedView {
                            key,
                            name: view_name,
                            text: text + "\n",
                        },
                    );
                }
                _ => crate::log!("Waveforms: ignored a view that isn't a JSON object"),
            }
        }
        Ok(Call::WriteDeclared { id, .. }) => {
            crate::log!("Waveforms: ignored a write of {id:?}, which isn't declared")
        }
        Ok(Call::OpenSource { file, line }) if files.contains(&file) => {
            crate::log!("Waveforms: open {file}:{line}");
            if let Some(main) = main {
                let _ = main.emit("wave-open-source", OpenSource { key, file, line });
                let _ = main.set_focus();
            }
        }
        Ok(Call::OpenSource { .. }) => {
            crate::log!("Waveforms: ignored a request to open a file outside the project")
        }
        Ok(Call::SaveAs { name, mime, data }) => {
            page_host::save_as(app, label, "Waveforms", &name, &mime, &data)
        }
        Ok(Call::OpenUrl(url)) => page_host::open_url("Waveforms", &url),
        Err(e) => crate::log!("Waveforms: ignored a request: {e}"),
    }
    page_host::respond(204, Vec::new())
}

/// Every open waveform window.
pub fn windows(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(l, _)| l.starts_with(LABEL_PREFIX))
        .map(|(_, w)| w)
        .collect()
}

/// Whether a window is a waveform window.
pub fn is_wave(label: &str) -> bool {
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
        assert!(label("x").starts_with("wave-"));
        assert_ne!(label("x"), crate::diagram_window::label("x"));
    }

    #[test]
    fn page_addresses() {
        let expected = if cfg!(windows) {
            "http://logix-wave.localhost/wave-1"
        } else {
            "logix-wave://localhost/wave-1"
        };
        assert_eq!(page_url("wave-1"), expected);
    }

    #[test]
    fn view_files_follow_the_testbench() {
        assert_eq!(view_file("main_tb.v"), "main_tb.view");
        assert_eq!(view_file("uart_tb.sv"), "uart_tb.view");
    }

    #[test]
    fn pages_are_served_by_window() {
        PAGES
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(
                "wave-test".into(),
                Page {
                    html: "<html>ok</html>".into(),
                    key: "k".into(),
                    files: vec![],
                    view_file: "main_tb.view".into(),
                },
            );
        assert!(PAGES
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains_key("wave-test"));
        forget("wave-test");
        assert!(!PAGES
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains_key("wave-test"));
    }
}
