mod apio;
mod bare;
mod build_dir;
mod commands;
pub mod logging;
mod menu;
mod platform;
mod project;
mod settings;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Emitter, Manager, WindowEvent};

/// State shared by all Tauri commands.
#[derive(Default)]
pub struct AppState {
    /// Set once Apio setup has succeeded. The lock also keeps two setup
    /// runs from overlapping (e.g. after a UI reload during development).
    apio: tokio::sync::Mutex<Option<apio::Apio>>,
    /// Held while a build or upload runs, so only one runs at a time.
    task: tokio::sync::Mutex<()>,
    /// Stops the running build, upload, test or simulation (Stop button).
    task_cancel: std::sync::Mutex<Option<tokio_util::sync::CancellationToken>>,
    /// Held while formatting, which reuses one scratch folder.
    format: tokio::sync::Mutex<()>,
    /// The Verilog language server, while running (live errors).
    lsp: tokio::sync::Mutex<Option<apio::LanguageServer>>,
    /// Board and example lists, fetched from Apio once per run.
    catalog: tokio::sync::Mutex<Option<Catalog>>,
    /// Whether the open project has unsaved changes (reported by the UI).
    dirty: AtomicBool,
    /// Set when the UI has already dealt with unsaved changes, so the next
    /// close request goes through.
    force_close: AtomicBool,
}

#[derive(Clone)]
struct Catalog {
    boards: Vec<apio::Board>,
    examples: apio::ExampleList,
    usb_rules: Vec<apio::UsbRule>,
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
        .setup(|app| {
            let info = app.package_info();
            logging::init(&format!(
                "Logix IDE {} ({}), Apio {}",
                info.version,
                platform::current().map_or("unknown platform".into(), |p| format!("{p:?}")),
                apio::APIO_RELEASE_TAG
            ));
            install_panic_logging();
            if let Some(window) = app.get_webview_window("main") {
                commands::apply_stored_window_preferences(&window);
            }
            Ok(())
        })
        .on_menu_event(|app, event| menu::on_event(app, event.id().as_ref()))
        .on_window_event(|window, event| {
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
            commands::setup_apio,
            commands::run_task,
            commands::cancel_task,
            commands::scan_devices,
            commands::connected_boards,
            commands::lsp_start,
            commands::lsp_send,
            commands::lsp_stop,
            commands::show_editor_menu,
            commands::show_output_menu,
            commands::log_output,
            commands::get_preferences,
            commands::set_preferences,
            commands::choose_folder_dialog,
            commands::save_gtkwave_layouts,
            commands::format_file,
            commands::get_boards,
            commands::get_examples,
            commands::open_example,
            commands::open_bare_minimum,
            commands::open_last_project,
            commands::new_project,
            commands::set_last_board,
            commands::open_project_dialog,
            commands::save_project,
            commands::open_project_at,
            commands::show_project_folder,
            commands::save_project_as_dialog,
            commands::check_file_name,
            commands::ask_confirm,
            commands::ask_save_changes,
            commands::set_document_state,
            commands::close_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Logix application");
}
