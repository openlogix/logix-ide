//! The debugger's window (Project > Debug; Logix Engine's docs/debugger.md):
//! Logix Engine's debug panel in a window of its own, one per project,
//! movable, resizable and not modal, as the Waveforms and Diagram windows.
//!
//! Unlike those, the page isn't a file: it's live, served by Logix Engine
//! on 127.0.0.1 with a random token in its address (`open_debug_panel` with
//! `open: false`), and talks to the engine's session over that server. So
//! the window loads that address, may navigate only on that server, and
//! gets no Tauri access (no capability names it). Closing the window ends
//! the debug session (`lib.rs`, `close_debug`), which releases the driven
//! nets.
//!
//! The page's host calls (the engine's page contract, `window.logixHost`:
//! its docs/tool-api.md): the window defines `chooseNets` (the panel's
//! "Choose nets…", which opens the net picker in the main window) and
//! `saveDebugLayout` (the panel's row order, rate and chart span, which the
//! main window writes into the project's debug.toml). With no Tauri access,
//! each call is a navigation to `/__logix-host/...` on the panel's server,
//! which the window stops here and passes to the main window as an event
//! (`debug-choose-nets`, `debug-layout`); the engine answers such an
//! address with nothing, should one get through.

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

/// Window labels start with this.
pub const LABEL_PREFIX: &str = "debug-";

/// The window's label for a project: one window per project.
pub fn label(key: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    format!("{LABEL_PREFIX}{:016x}", h.finish())
}

/// Whether `url` may load in the window: only the engine's panel server,
/// the address `open_debug_panel` gave.
fn allowed(url: &Url, panel: &Url) -> bool {
    url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port() == panel.port()
        && panel.port().is_some()
}

/// The page's host, defined before the panel's own script runs.
const HOST_SCRIPT: &str = r#"window.logixHost = Object.freeze({
  version: 1,
  features: ["chooseNets", "saveDebugLayout"],
  chooseNets: () => location.assign("/__logix-host/choose-nets"),
  saveDebugLayout: (layout) =>
    location.assign("/__logix-host/debug-layout?data=" + encodeURIComponent(JSON.stringify(layout))),
});"#;

/// What the host calls' addresses start with.
const HOST_CALL_PATH: &str = "/__logix-host/";

/// The panel's layout, as `saveDebugLayout` gives it: its rows' names in
/// order (an entry's `name`, else its net), the read rate and the chart
/// span.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugLayout {
    #[serde(default)]
    pub watch: Vec<String>,
    #[serde(default)]
    pub drive: Vec<String>,
    #[serde(default, alias = "refresh_ms")]
    pub refresh_ms: Option<u32>,
    #[serde(default, alias = "window_s")]
    pub window_s: Option<u32>,
}

/// A host call the page made.
#[derive(Debug, PartialEq)]
enum HostCall {
    ChooseNets,
    SaveLayout(DebugLayout),
    /// A host call this window doesn't know, or one it couldn't read.
    Unknown,
}

/// The host call `url` is, if it is one, on the panel's server.
fn host_call(url: &Url, panel: &Url) -> Option<HostCall> {
    if !allowed(url, panel) {
        return None;
    }
    let call = url.path().strip_prefix(HOST_CALL_PATH)?;
    Some(match call {
        "choose-nets" => HostCall::ChooseNets,
        "debug-layout" => url
            .query_pairs()
            .find(|(k, _)| k == "data")
            .and_then(|(_, v)| serde_json::from_str(&v).ok())
            .map_or(HostCall::Unknown, HostCall::SaveLayout),
        _ => HostCall::Unknown,
    })
}

/// The `debug-layout` event: a project's panel layout.
#[derive(Clone, Serialize)]
struct LayoutEvent {
    key: String,
    layout: DebugLayout,
}

/// Passes a host call to the main window.
fn pass_on(app: &AppHandle, key: &str, call: HostCall) {
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    match call {
        HostCall::ChooseNets => {
            let _ = main.emit("debug-choose-nets", key.to_string());
            let _ = main.set_focus();
        }
        HostCall::SaveLayout(layout) => {
            let _ = main.emit(
                "debug-layout",
                LayoutEvent {
                    key: key.to_string(),
                    layout,
                },
            );
        }
        HostCall::Unknown => crate::log!("Debugger: ignored a host call it doesn't know"),
    }
}

/// Shows a project's debug panel: opens its window, or loads the new
/// address in the open one. `url` is `open_debug_panel`'s.
pub fn show(app: &AppHandle, key: &str, title: &str, url: &str) -> tauri::Result<()> {
    let label = label(key);
    let panel: Url = url
        .parse()
        .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!("Bad debug panel address: {e}")))?;
    if !allowed(&panel, &panel) {
        return Err(tauri::Error::Anyhow(anyhow::anyhow!(
            "The debug panel isn't on this computer: {url}"
        )));
    }
    if let Some(window) = app.get_webview_window(&label) {
        window.set_title(title)?;
        window.navigate(panel)?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    let p = panel.clone();
    let (app2, key2) = (app.clone(), key.to_string());
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(panel))
        .title(title)
        .inner_size(820.0, 640.0)
        .min_inner_size(420.0, 320.0)
        .initialization_script(HOST_SCRIPT)
        .on_navigation(move |u| match host_call(u, &p) {
            Some(call) => {
                pass_on(&app2, &key2, call);
                false
            }
            None => allowed(u, &p),
        })
        .build()?;
    crate::commands::apply_stored_window_preferences(&window);
    Ok(())
}

/// Closes a project's debug window, if open (Upload, or a new Debug that
/// fails, ends the session it showed).
pub fn close(app: &AppHandle, key: &str) {
    if let Some(window) = app.get_webview_window(&label(key)) {
        let _ = window.destroy();
    }
}

/// The open debug windows.
pub fn windows(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(l, _)| l.starts_with(LABEL_PREFIX))
        .map(|(_, w)| w)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_panel_server_is_allowed() {
        let panel: Url = "http://127.0.0.1:4321/?t=abc&session=s1".parse().unwrap();
        let ok = |s: &str| allowed(&s.parse().unwrap(), &panel);
        assert!(ok("http://127.0.0.1:4321/?t=abc&session=s2"));
        assert!(!ok("http://127.0.0.1:4322/"));
        assert!(!ok("http://localhost:4321/"));
        assert!(!ok("https://example.com/"));
    }

    #[test]
    fn host_calls_are_read() {
        let panel: Url = "http://127.0.0.1:4321/?t=abc".parse().unwrap();
        let call = |s: &str| host_call(&s.parse().unwrap(), &panel);
        assert_eq!(
            call("http://127.0.0.1:4321/__logix-host/choose-nets"),
            Some(HostCall::ChooseNets)
        );
        let data = r#"{"watch":["b","a"],"drive":["d"],"refresh_ms":500,"window_s":30}"#;
        let mut u: Url = "http://127.0.0.1:4321/__logix-host/debug-layout"
            .parse()
            .unwrap();
        u.query_pairs_mut().append_pair("data", data);
        assert_eq!(
            host_call(&u, &panel),
            Some(HostCall::SaveLayout(DebugLayout {
                watch: vec!["b".into(), "a".into()],
                drive: vec!["d".into()],
                refresh_ms: Some(500),
                window_s: Some(30),
            }))
        );
        assert_eq!(
            call("http://127.0.0.1:4321/__logix-host/debug-layout?data=nope"),
            Some(HostCall::Unknown)
        );
        // Not a host call: the panel itself, or another server.
        assert_eq!(call("http://127.0.0.1:4321/?t=abc"), None);
        assert_eq!(call("http://127.0.0.1:9999/__logix-host/choose-nets"), None);
    }

    #[test]
    fn labels_are_per_project() {
        assert!(label("a").starts_with(LABEL_PREFIX));
        assert_ne!(label("a"), label("b"));
    }
}
