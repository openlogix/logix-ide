//! The waveform viewer's window (Project > Simulate; Logix Engine's
//! docs/requirements.md, Waveform viewer): Logix Engine's viewer page
//! (`simulate`'s `viewer_file`) in a window of its own, one per project,
//! movable, resizable and not modal, as the Diagram window. Each Simulate
//! shows its page there, reloaded in place (R6): the page reads the
//! testbench's `.view`, a project file, so the signals and the view stay.
//!
//! The page is the engine's, as is, kept in memory and served on the
//! `logix-wave` scheme (the build folder is cleared before each task). It
//! gets no Tauri access: the functions the page calls when it has a host
//! (`window.logixWaveSaveView`, `logixWaveSaveFile`, `logixWaveOpenSource`)
//! are defined here as POST requests to its own address, which `serve`
//! answers: a saved view becomes a `wave-view` event to the main window (a
//! project file, as an edit), an image goes through a save dialog, and a
//! source line becomes a `wave-open-source` event, only for a file of the
//! project.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

/// The scheme the pages are served on.
pub const SCHEME: &str = "logix-wave";
/// Window labels start with this.
const LABEL_PREFIX: &str = "wave-";

/// Defines the page's host functions (the engine's tool API, simulate
/// tool, The waveform viewer) as POST requests to its own address,
/// `/<label>/<what>`.
const INIT_SCRIPT: &str = "(function () {\
  const post = (what, body) => fetch(location.pathname.replace(/\\/+$/, '') + '/' + what, { method: 'POST', body: JSON.stringify(body) }).catch(() => {});\
  window.logixWaveSaveView = (view) => post('save-view', view);\
  window.logixWaveSaveFile = (name, mime, data) => post('save-file', { name, mime, data });\
  window.logixWaveOpenSource = (file, line) => post('open-source', { file, line });\
})();";

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
        .initialization_script(INIT_SCRIPT)
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

/// What a request to the scheme asks: the page, or one of its host calls.
#[derive(Debug, PartialEq)]
enum Request<'a> {
    Page(&'a str),
    Call(&'a str, &'a str),
}

fn parse(path: &str) -> Request<'_> {
    let path = path.trim_start_matches('/');
    let path = path.split(['?', '#']).next().unwrap_or("");
    match path.split_once('/') {
        Some((label, what)) if !what.is_empty() => Request::Call(label, what),
        _ => Request::Page(path.trim_end_matches('/')),
    }
}

#[derive(Deserialize)]
struct FileRequest {
    name: String,
    mime: String,
    data: String,
}

#[derive(Deserialize)]
struct SourceRequest {
    file: String,
    line: Option<u32>,
}

/// The scheme's handler: a window's page (GET), or what its page asks
/// (POST to `/<label>/save-view`, `save-file` or `open-source`).
pub fn serve(app: &AppHandle, path: &str, body: &[u8]) -> tauri::http::Response<Vec<u8>> {
    let respond = |status: u16, body: Vec<u8>| {
        tauri::http::Response::builder()
            .status(status)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(body)
            .expect("a valid response")
    };
    let (label, what) = match parse(path) {
        Request::Page(label) => {
            let html = PAGES
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|p| p.get(label).map(|page| page.html.clone()));
            return match html {
                Some(h) => respond(200, h.into_bytes()),
                None => respond(404, b"No waveform.".to_vec()),
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
        return respond(404, Vec::new());
    };
    let main = app.get_webview_window("main");
    match what {
        "save-view" => {
            // The view as the page gives it, pretty-printed for people
            // reading the project's files.
            let text = serde_json::from_slice::<serde_json::Value>(body)
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
        "open-source" => match serde_json::from_slice::<SourceRequest>(body) {
            Ok(r) if files.contains(&r.file) => {
                crate::log!("Waveforms: open {}:{:?}", r.file, r.line);
                if let Some(main) = main {
                    let _ = main.emit(
                        "wave-open-source",
                        OpenSource {
                            key,
                            file: r.file,
                            line: r.line.unwrap_or(1).max(1),
                        },
                    );
                    let _ = main.set_focus();
                }
            }
            _ => crate::log!("Waveforms: ignored a request to open a file outside the project"),
        },
        "save-file" => match serde_json::from_slice::<FileRequest>(body) {
            Ok(r) => save_file(app, label, r),
            Err(e) => crate::log!("Waveforms: ignored a file to save: {e}"),
        },
        other => crate::log!("Waveforms: ignored {other:?}"),
    }
    respond(204, Vec::new())
}

/// The bytes of a file the page saves: SVG's text, or PNG's base64.
fn file_bytes(r: &FileRequest) -> Option<Vec<u8>> {
    if r.mime == "image/png" {
        base64::engine::general_purpose::STANDARD
            .decode(&r.data)
            .ok()
    } else {
        Some(r.data.clone().into_bytes())
    }
}

/// Only a plain file name, never a path.
fn safe_name(name: &str) -> String {
    let n: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':'))
        .collect();
    if n.trim().is_empty() || n.starts_with('.') {
        "waveforms".into()
    } else {
        n
    }
}

/// Asks where to save an image of the view, then writes it.
fn save_file(app: &AppHandle, label: &str, r: FileRequest) {
    let Some(bytes) = file_bytes(&r) else {
        crate::log!("Waveforms: an image that can't be decoded");
        return;
    };
    let mut dialog = app.dialog().file().set_file_name(safe_name(&r.name));
    if let Some(w) = app.get_webview_window(label) {
        dialog = dialog.set_parent(&w);
    }
    dialog.save_file(move |picked| {
        let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
            return;
        };
        match std::fs::write(&path, &bytes) {
            Ok(()) => crate::log!("Waveforms: saved {}", path.display()),
            Err(e) => crate::log!("Waveforms: couldn't save {}: {e}", path.display()),
        }
    });
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
    fn requests_are_the_page_or_its_calls() {
        assert_eq!(parse("/wave-1"), Request::Page("wave-1"));
        assert_eq!(parse("/wave-1/"), Request::Page("wave-1"));
        assert_eq!(parse("/wave-1?theme=dark"), Request::Page("wave-1"));
        assert_eq!(
            parse("/wave-1/save-view"),
            Request::Call("wave-1", "save-view")
        );
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
    fn images_are_text_or_base64_and_named_plainly() {
        let svg = FileRequest {
            name: "a.svg".into(),
            mime: "image/svg+xml".into(),
            data: "<svg/>".into(),
        };
        assert_eq!(file_bytes(&svg).unwrap(), b"<svg/>");
        let png = FileRequest {
            name: "a.png".into(),
            mime: "image/png".into(),
            data: "iVBORw0KGgo=".into(),
        };
        assert_eq!(&file_bytes(&png).unwrap()[..4], b"\x89PNG");
        assert_eq!(safe_name("../../etc/x.png"), "waveforms");
        assert_eq!(safe_name("dir/x.png"), "dirx.png");
        assert_eq!(safe_name(".hidden"), "waveforms");
        assert_eq!(safe_name("main_tb.svg"), "main_tb.svg");
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
