pub mod book;
mod commands;
pub mod image_entry;
pub mod launch_open;
pub mod natural_sort;
pub mod protocol;
pub mod source;
pub mod state;
pub mod temp;

use launch_open::PendingOpen;
use state::AppState;
#[cfg(target_os = "macos")]
use tauri::Emitter;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let base = temp::default_base();
    temp::cleanup_stale(&base);
    let instance = temp::InstanceDir::create(&base).expect("could not create temporary folder");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(instance))
        .manage(PendingOpen::default())
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
            commands::take_pending_open
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Opened { urls } => {
            if let Some(path) = urls.iter().find_map(|url| url.to_file_path().ok()) {
                app.state::<PendingOpen>().set(path.to_string_lossy().into_owned());
                let _ = app.emit(launch_open::OPEN_EVENT, ());
            }
        }
        tauri::RunEvent::Exit => app.state::<AppState>().shutdown(),
        _ => {}
    });
}
