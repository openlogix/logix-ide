mod build_dir;
mod commands;
mod diagram_window;
mod download;
mod engine;
mod examples;
mod language_server;
pub mod logging;
mod menu;
mod outcome;
mod platform;
mod project;
mod settings;
mod setup_events;
mod system_info;
mod version;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Emitter, Manager, WindowEvent};

/// State shared by all Tauri commands.
#[derive(Default)]
pub struct AppState {
    /// The running Logix Engine, set once its setup has succeeded. The lock
    /// also keeps two setup runs from overlapping (e.g. after a UI reload
    /// during development). Dropping it stops the engine.
    engine: tokio::sync::Mutex<Option<std::sync::Arc<engine::Engine>>>,
    /// Held while a build or upload runs, so only one runs at a time.
    task: tokio::sync::Mutex<()>,
    /// Stops the running build, upload, test or simulation (Stop button).
    task_cancel: std::sync::Mutex<Option<tokio_util::sync::CancellationToken>>,
    /// Held while formatting, which reuses one scratch folder.
    format: tokio::sync::Mutex<()>,
    /// The Verilog language server, while running (live errors), with its
    /// number: each start gets a new one, so a replaced server's exit isn't
    /// taken for the current one's.
    lsp: tokio::sync::Mutex<Option<(u64, language_server::LanguageServer)>>,
    /// Board and example lists, fetched once per run (boards from the
    /// engine, examples built in).
    catalog: tokio::sync::Mutex<Option<Catalog>>,
    /// Whether the open project has unsaved changes (reported by the UI).
    dirty: AtomicBool,
    /// What the menu items can act on now (reported by the UI); the others
    /// are greyed out (`menu::is_enabled`).
    menu_state: std::sync::Mutex<menu::MenuState>,
    /// Set when the UI has already dealt with unsaved changes, so the next
    /// close request goes through.
    force_close: AtomicBool,
}

#[derive(Clone)]
struct Catalog {
    boards: Vec<engine::Board>,
    families: Vec<engine::FpgaFamily>,
    examples: examples::ExampleList,
}

/// Logs crashes (panics) too, so they're in problem reports.
fn install_panic_logging() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log!("Crash: {info}");
        default(info);
    }));
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .menu(menu::build)
        .register_uri_scheme_protocol(diagram_window::SCHEME, |_ctx, request| {
            diagram_window::serve(request.uri().path())
        })
        .setup(|app| {
            logging::init(&format!(
                "Logix IDE {} ({}), Logix Engine {}",
                version::APP_VERSION,
                platform::current().map_or("unknown platform".into(), |p| format!("{p:?}")),
                version::shown(engine::ENGINE_RELEASE_TAG)
            ));
            if let Some(home) = std::env::var_os(platform::HOME_VARIABLE).filter(|v| !v.is_empty())
            {
                log!(
                    "Data folder from {}: {}",
                    platform::HOME_VARIABLE,
                    std::path::Path::new(&home).display()
                );
            }
            install_panic_logging();
            // ~/.logix/README.txt: what each program's folder is.
            if let Ok(dir) = platform::current_implemented().and_then(|p| p.data_dir()) {
                let tester =
                    std::env::var_os(platform::HOME_VARIABLE).is_some_and(|v| !v.is_empty());
                if let Err(e) = platform::write_logix_readme(&dir, tester) {
                    log!("Couldn't write ~/.logix/README.txt: {e:#}");
                }
            }
            if let Some(window) = app.get_webview_window("main") {
                commands::apply_stored_window_preferences(&window);
            }
            Ok(())
        })
        .on_menu_event(|app, event| menu::on_event(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                if diagram_window::is_diagram(window.label()) {
                    diagram_window::forget(window.label());
                } else if window.label() == "main" {
                    // Diagram windows close with the main window.
                    for w in diagram_window::windows(window.app_handle()) {
                        let _ = w.close();
                    }
                }
            }
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.dirty.load(Ordering::SeqCst) && !state.force_close.load(Ordering::SeqCst) {
                    // Let the UI ask about unsaved changes; it calls
                    // `close_window` when it's OK to close.
                    api.prevent_close();
                    let _ = window.emit("close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::system_info,
            commands::copy_text,
            commands::setup,
            commands::run_task,
            commands::cancel_task,
            commands::scan_devices,
            commands::connected_devices,
            commands::lsp_start,
            commands::lsp_send,
            commands::lsp_stop,
            commands::show_editor_menu,
            commands::set_menu_state,
            commands::show_output_menu,
            commands::log_output,
            commands::get_preferences,
            commands::set_preferences,
            commands::choose_folder_dialog,
            commands::save_gtkwave_layouts,
            commands::format_file,
            commands::get_boards,
            commands::get_fpga_families,
            commands::project_boards,
            commands::custom_boards_stamp,
            commands::get_examples,
            commands::open_example,
            commands::open_bare_minimum,
            commands::open_last_project,
            commands::close_project,
            commands::new_project,
            commands::set_last_board,
            commands::recent_boards,
            commands::last_board,
            commands::last_variant,
            commands::recent_projects,
            commands::open_project_dialog,
            commands::save_project,
            commands::open_project_at,
            commands::show_project_folder,
            commands::copy_project_path,
            commands::compare_boards,
            commands::save_project_as_dialog,
            commands::check_file_name,
            commands::ask_confirm,
            commands::ask_save_changes,
            commands::set_document_state,
            commands::close_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Logix IDE application");
}
