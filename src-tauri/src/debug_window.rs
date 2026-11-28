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

use std::hash::{Hash, Hasher};

use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};

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
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(panel))
        .title(title)
        .inner_size(820.0, 640.0)
        .min_inner_size(420.0, 320.0)
        .on_navigation(move |u| allowed(u, &p))
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
    fn labels_are_per_project() {
        assert!(label("a").starts_with(LABEL_PREFIX));
        assert_ne!(label("a"), label("b"));
    }
}
