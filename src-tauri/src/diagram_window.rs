//! The Diagram window (Project > Show Diagram): Logix Engine's diagram page
//! (`diagram` tool, `diagram/diagram.html`) in a window of its own, one per
//! project, movable, resizable and not modal, so the user keeps working
//! while it's open. It's redrawn after each Verify.
//!
//! The page is the engine's, as is: self-contained, with every module,
//! drill-down, pan, zoom and hover. Logix IDE keeps it in memory and serves
//! it on the `logix-diagram` scheme (the build folder is cleared before
//! each task, so the file doesn't last). The page gets no Tauri access:
//! double-clicking a block calls `window.logixDiagramOpenSource(file, line)`,
//! which this window defines as a navigation to `logix-open-source:`; the
//! navigation is cancelled here and becomes a `diagram-open-source` event
//! to the main window, only for a file of the project.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// The scheme the pages are served on.
pub const SCHEME: &str = "logix-diagram";
/// The scheme a click on a block navigates to.
const OPEN_SOURCE: &str = "logix-open-source";
/// Window labels start with this.
const LABEL_PREFIX: &str = "diagram-";

/// Defines the engine page's hook for clicks on blocks (see the engine's
/// tool API, diagram tool, Showing the diagram).
const INIT_SCRIPT: &str = "window.logixDiagramOpenSource = function (file, line) {\
    location.href = 'logix-open-source://open/?file=' + encodeURIComponent(file) + '&line=' + encodeURIComponent(line);\
};";

/// A window's page and what it may open.
struct Page {
    html: String,
    /// The project's key, as the UI knows it.
    key: String,
    /// The project's file names: the only files a click may open.
    files: Vec<String>,
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

/// The page's address: `#module` opens that module first.
fn page_url(label: &str, module: Option<&str>) -> String {
    let base = if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{label}")
    } else {
        format!("{SCHEME}://localhost/{label}")
    };
    match module {
        Some(m) => format!("{base}#{m}"),
        None => base,
    }
}

/// What the main window gets when the user clicks a block.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSource {
    pub key: String,
    pub file: String,
    pub line: u32,
}

/// A click's file and line, if the file is one of the project's.
fn open_source_request(url: &tauri::Url, files: &[String]) -> Option<(String, u32)> {
    let mut file = None;
    let mut line = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "file" => file = Some(v.into_owned()),
            "line" => line = v.parse::<u32>().ok(),
            _ => {}
        }
    }
    let file = file?;
    files
        .contains(&file)
        .then_some((file, line.unwrap_or(1).max(1)))
}

/// Shows the project's diagram: opens its window, or updates the open one
/// in place (it stays on the module it shows). `module` is the module to
/// open a new window on; `focus` brings the window to the front.
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
            },
        );
    if let Some(window) = app.get_webview_window(&label) {
        window.set_title(title)?;
        match module {
            Some(_) => window.navigate(page_url(&label, module).parse().expect("a valid URL"))?,
            None => window.eval("location.reload()")?,
        }
        if focus {
            window.unminimize()?;
            window.set_focus()?;
        }
        return Ok(());
    }
    let handle = app.clone();
    let for_navigation = label.clone();
    let url = page_url(&label, module).parse().expect("a valid URL");
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::CustomProtocol(url))
        .title(title)
        .inner_size(1000.0, 700.0)
        .min_inner_size(480.0, 320.0)
        .initialization_script(INIT_SCRIPT)
        .on_navigation(move |url| {
            if url.scheme() != OPEN_SOURCE {
                return url.scheme() == SCHEME
                    || url.host_str() == Some(&format!("{SCHEME}.localhost"));
            }
            let request = PAGES
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|p| p.get(&for_navigation))
                .and_then(|page| {
                    open_source_request(url, &page.files).map(|(file, line)| OpenSource {
                        key: page.key.clone(),
                        file,
                        line,
                    })
                });
            match request {
                Some(r) => {
                    crate::log!("Diagram: open {}:{}", r.file, r.line);
                    if let Some(main) = handle.get_webview_window("main") {
                        let _ = main.emit("diagram-open-source", r);
                        let _ = main.set_focus();
                    }
                }
                None => crate::log!("Diagram: ignored a request to open {url}"),
            }
            false
        })
        .build()?;
    crate::commands::apply_stored_window_preferences(&window);
    Ok(())
}

/// The scheme's handler: the page of the window the address names.
pub fn serve(path: &str) -> tauri::http::Response<Vec<u8>> {
    let label = path
        .trim_start_matches('/')
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    let html = PAGES
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|p| p.get(label).map(|page| page.html.clone()));
    let builder = tauri::http::Response::builder();
    match html {
        Some(h) => builder
            .header("Content-Type", "text/html; charset=utf-8")
            .body(h.into_bytes()),
        None => builder.status(404).body(b"No diagram.".to_vec()),
    }
    .expect("a valid response")
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
    fn only_project_files_are_opened() {
        let files = vec!["main.v".to_string(), "uart.v".to_string()];
        let url =
            |q: &str| -> tauri::Url { format!("logix-open-source://open/?{q}").parse().unwrap() };
        assert_eq!(
            open_source_request(&url("file=uart.v&line=12"), &files),
            Some(("uart.v".into(), 12))
        );
        assert_eq!(
            open_source_request(&url("file=main.v"), &files),
            Some(("main.v".into(), 1))
        );
        for bad in [
            "file=..%2Fsecret.v&line=1",
            "file=%2Fetc%2Fpasswd",
            "line=3",
            "file=other.v",
        ] {
            assert_eq!(open_source_request(&url(bad), &files), None, "{bad}");
        }
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
                },
            );
        let r = serve("/diagram-test");
        assert_eq!(r.status(), 200);
        assert_eq!(r.body(), b"<html>ok</html>");
        assert_eq!(serve("/diagram-none").status(), 404);
        forget("diagram-test");
        assert_eq!(serve("/diagram-test").status(), 404);
        assert_eq!(
            page_url("diagram-1", Some("uart")),
            "logix-diagram://localhost/diagram-1#uart"
        );
    }
}
