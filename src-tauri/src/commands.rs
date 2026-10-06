use crate::book::{Book, BookInfo};
use crate::launch_open::PendingOpen;
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

pub const SUPERSEDED: &str = "superseded";

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn open_book(path: String, state: State<'_, AppState>) -> Result<BookInfo, String> {
    let ticket = state.next_ticket();
    let path = PathBuf::from(path);
    let base = state.temp_base().to_path_buf();
    let source_path = path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || Book::open(&source_path, ticket, &base))
        .await
        .map_err(|e| format!("Internal error: {e}"))?;
    if !state.is_latest(ticket) {
        return Err(SUPERSEDED.into());
    }
    let book = result.map_err(|e| e.message(&display_name(&path)))?;
    let info = book.info();
    if state.install(book) {
        Ok(info)
    } else {
        Err(SUPERSEDED.into())
    }
}

#[tauri::command]
pub fn take_pending_open(pending: State<'_, PendingOpen>) -> Option<String> {
    pending.take()
}
