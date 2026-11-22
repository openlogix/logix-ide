//! The host side of Logix Engine's pages in Logix IDE's own windows (the
//! Waveforms window, `wave_window.rs`, and the Diagram window,
//! `diagram_window.rs`): the page contract, `window.logixHost` version 1
//! (the engine's docs/tool-api.md, Logix Viewer), the one way the pages ask
//! their host for something (the user's decision, 2026-11-22). Each call
//! becomes a POST request to the page's own address, `/<label>/<call>`,
//! which the window's scheme handler answers with `request`: the pages get
//! no Tauri access. Plain Rust, tested; each window decides what a call
//! does for it (which file `writeDeclared` may write, which event opens a
//! source line).

use base64::Engine as _;
use serde::Deserialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// Defines `window.logixHost` (version 1) before the page's scripts run:
/// each call posts its arguments, as JSON, to `<page address>/<call>`.
pub const SCRIPT: &str = "(function () {\
  const post = (call, body) => fetch(location.pathname.replace(/\\/+$/, '') + '/' + call, { method: 'POST', body: JSON.stringify(body) }).catch(() => {});\
  window.logixHost = {\
    version: 1,\
    features: ['saveAs', 'writeDeclared', 'openSource', 'openUrl'],\
    saveAs: (name, mime, data) => post('save-as', { name, mime, data }),\
    writeDeclared: (id, text) => post('write-declared', { id, text }),\
    openSource: (file, line) => post('open-source', { file, line }),\
    openUrl: (url) => post('open-url', { url }),\
  };\
})();";

/// What a request to a window's scheme asks: its page, or a host call.
#[derive(Debug, PartialEq)]
pub enum Request<'a> {
    Page(&'a str),
    Call(&'a str, &'a str),
}

/// A request's path: `/<label>` is the page, `/<label>/<call>` a call.
pub fn request(path: &str) -> Request<'_> {
    let path = path.trim_start_matches('/');
    let path = path.split(['?', '#']).next().unwrap_or("");
    match path.split_once('/') {
        Some((label, what)) if !what.is_empty() => Request::Call(label, what),
        _ => Request::Page(path.trim_end_matches('/')),
    }
}

/// A host call, read from its name and body.
#[derive(Debug, PartialEq)]
pub enum Call {
    /// The system's Save dialog for a file the page made: an image's SVG
    /// text, or a PNG as base64.
    SaveAs {
        name: String,
        mime: String,
        data: String,
    },
    /// A file the engine declared for the page, by id, such as "view".
    WriteDeclared { id: String, text: String },
    /// A place in a source file, relative to the source folder.
    OpenSource { file: String, line: u32 },
    /// A web page, `https://` only.
    OpenUrl(String),
}

#[derive(Deserialize)]
struct SaveAsBody {
    name: String,
    mime: String,
    data: String,
}

#[derive(Deserialize)]
struct WriteDeclaredBody {
    id: String,
    text: String,
}

#[derive(Deserialize)]
struct OpenSourceBody {
    file: String,
    line: Option<u32>,
}

#[derive(Deserialize)]
struct OpenUrlBody {
    url: String,
}

/// A call's meaning, or why it's refused.
pub fn call(what: &str, body: &[u8]) -> Result<Call, String> {
    let bad = |e: serde_json::Error| format!("{what}: {e}");
    match what {
        "save-as" => {
            let b: SaveAsBody = serde_json::from_slice(body).map_err(bad)?;
            Ok(Call::SaveAs {
                name: b.name,
                mime: b.mime,
                data: b.data,
            })
        }
        "write-declared" => {
            let b: WriteDeclaredBody = serde_json::from_slice(body).map_err(bad)?;
            Ok(Call::WriteDeclared {
                id: b.id,
                text: b.text,
            })
        }
        "open-source" => {
            let b: OpenSourceBody = serde_json::from_slice(body).map_err(bad)?;
            Ok(Call::OpenSource {
                file: b.file,
                line: b.line.unwrap_or(1).max(1),
            })
        }
        "open-url" => {
            let b: OpenUrlBody = serde_json::from_slice(body).map_err(bad)?;
            Ok(Call::OpenUrl(b.url))
        }
        other => Err(format!("no host call {other:?}")),
    }
}

/// The scheme's answer: the page, or an empty one to a call.
pub fn respond(status: u16, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(body)
        .expect("a valid response")
}

/// The bytes of a file the page saves: SVG's text, or PNG's base64.
pub fn file_bytes(mime: &str, data: &str) -> Option<Vec<u8>> {
    if mime == "image/png" {
        base64::engine::general_purpose::STANDARD.decode(data).ok()
    } else {
        Some(data.as_bytes().to_vec())
    }
}

/// Only a plain file name, never a path (`fallback` when nothing is left).
pub fn safe_name(name: &str, fallback: &str) -> String {
    let n: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':'))
        .collect();
    if n.trim().is_empty() || n.starts_with('.') {
        fallback.into()
    } else {
        n
    }
}

/// `saveAs`: asks where to save the file over the page's window, then
/// writes it. `who` names the window in the log.
pub fn save_as(app: &AppHandle, label: &str, who: &str, name: &str, mime: &str, data: &str) {
    let Some(bytes) = file_bytes(mime, data) else {
        crate::log!("{who}: a file that can't be decoded");
        return;
    };
    let mut dialog = app
        .dialog()
        .file()
        .set_file_name(safe_name(name, &who.to_lowercase()));
    if let Some(w) = app.get_webview_window(label) {
        dialog = dialog.set_parent(&w);
    }
    let who = who.to_string();
    dialog.save_file(move |picked| {
        let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
            return;
        };
        match std::fs::write(&path, &bytes) {
            Ok(()) => crate::log!("{who}: saved {}", path.display()),
            Err(e) => crate::log!("{who}: couldn't save {}: {e}", path.display()),
        }
    });
}

/// `openUrl`: a web page in the system's browser, `https://` only.
pub fn open_url(who: &str, url: &str) {
    let result = crate::platform::current_implemented().and_then(|p| p.open_url(url));
    if let Err(e) = result {
        crate::log!("{who}: didn't open {url}: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_the_page_or_its_calls() {
        assert_eq!(request("/wave-1"), Request::Page("wave-1"));
        assert_eq!(request("/wave-1/"), Request::Page("wave-1"));
        assert_eq!(request("/wave-1?theme=dark"), Request::Page("wave-1"));
        assert_eq!(
            request("/diagram-1/open-source"),
            Request::Call("diagram-1", "open-source")
        );
    }

    #[test]
    fn calls_are_read_from_their_body() {
        assert_eq!(
            call("write-declared", br#"{"id":"view","text":"{}\n"}"#),
            Ok(Call::WriteDeclared {
                id: "view".into(),
                text: "{}\n".into()
            })
        );
        assert_eq!(
            call("open-source", br#"{"file":"main.v","line":12}"#),
            Ok(Call::OpenSource {
                file: "main.v".into(),
                line: 12
            })
        );
        // No line: the file's first.
        assert_eq!(
            call("open-source", br#"{"file":"main.v"}"#),
            Ok(Call::OpenSource {
                file: "main.v".into(),
                line: 1
            })
        );
        assert_eq!(
            call("open-url", br#"{"url":"https://example.com"}"#),
            Ok(Call::OpenUrl("https://example.com".into()))
        );
        assert!(matches!(
            call(
                "save-as",
                br#"{"name":"a.svg","mime":"image/svg+xml","data":"<svg/>"}"#
            ),
            Ok(Call::SaveAs { .. })
        ));
        assert!(call("save-view", b"{}").is_err());
        assert!(call("open-source", b"not json").is_err());
    }

    #[test]
    fn the_script_offers_every_call_of_version_1() {
        for name in [
            "version: 1",
            "saveAs",
            "writeDeclared",
            "openSource",
            "openUrl",
        ] {
            assert!(SCRIPT.contains(name), "{name}");
        }
        for path in [
            "'save-as'",
            "'write-declared'",
            "'open-source'",
            "'open-url'",
        ] {
            assert!(SCRIPT.contains(path), "{path}");
        }
    }

    #[test]
    fn files_are_text_or_base64_and_named_plainly() {
        assert_eq!(file_bytes("image/svg+xml", "<svg/>").unwrap(), b"<svg/>");
        assert_eq!(
            &file_bytes("image/png", "iVBORw0KGgo=").unwrap()[..4],
            b"\x89PNG"
        );
        assert_eq!(safe_name("../../etc/x.png", "waveforms"), "waveforms");
        assert_eq!(safe_name("dir/x.png", "waveforms"), "dirx.png");
        assert_eq!(safe_name(".hidden", "diagram"), "diagram");
        assert_eq!(safe_name("main_tb.svg", "waveforms"), "main_tb.svg");
    }
}
