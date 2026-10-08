pub mod book;
mod commands;
pub mod fingerprint;
pub mod image_entry;
pub mod launch_open;
pub mod natural_sort;
pub mod progress;
pub mod protocol;
pub mod source;
pub mod state;
pub mod temp;

use launch_open::PendingOpen;
use state::AppState;
#[cfg(target_os = "macos")]
use tauri::Emitter;
use tauri::Manager;

#[cfg(target_os = "macos")]
const MAIN_WINDOW: &str = "main";

/// Shows the main window. Makes a new empty one if the user closed it.
#[cfg(target_os = "macos")]
fn open_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == MAIN_WINDOW) else {
        return;
    };
    if let Err(e) = tauri::WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
        eprintln!("could not open window: {e}");
    }
}

fn flush_progress(app: &tauri::AppHandle) {
    if let Some(progress) = app.try_state::<progress::ProgressTracker>() {
        progress.flush();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let base = temp::default_base();
    temp::cleanup_stale(&base);
    let instance = temp::InstanceDir::create(&base).expect("could not create temporary folder");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(instance))
        .manage(PendingOpen::default())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            app.manage(progress::ProgressTracker::new(progress::default_file(&dir)));
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("comic", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            tauri::async_runtime::spawn_blocking(move || {
                let book = app.state::<AppState>().current();
                responder.respond(protocol::page_response(book.as_deref(), &path));
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_book,
            commands::report_progress,
            commands::flush_progress,
            commands::take_pending_open
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| match event {
        // Progress is flushed when a window goes, so a macOS app that stays alive keeps it.
        tauri::RunEvent::WindowEvent { event: tauri::WindowEvent::Destroyed, .. } => {
            flush_progress(app);
        }
        // On macOS the app stays open after its last window closes.
        #[cfg(target_os = "macos")]
        tauri::RunEvent::ExitRequested { code: None, api, .. } => {
            api.prevent_exit();
            flush_progress(app);
            app.state::<AppState>().close_book();
        }
        tauri::RunEvent::ExitRequested { .. } => flush_progress(app),
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Opened { urls } => {
            if let Some(path) = urls.iter().find_map(|url| url.to_file_path().ok()) {
                app.state::<PendingOpen>().set(path.to_string_lossy().into_owned());
                // A new window takes the pending path when its UI loads.
                if app.get_webview_window(MAIN_WINDOW).is_none() {
                    open_main_window(app);
                } else {
                    let _ = app.emit(launch_open::OPEN_EVENT, ());
                }
            }
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => open_main_window(app),
        tauri::RunEvent::Exit => {
            flush_progress(app);
            app.state::<AppState>().shutdown();
        }
        _ => {}
    });
}
