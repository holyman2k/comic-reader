use crate::book::{Book, BookInfo, Resume};
use crate::launch_open::PendingOpen;
use crate::progress::{ProgressTracker, Report};
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
pub async fn open_book(
    path: String,
    state: State<'_, AppState>,
    progress: State<'_, ProgressTracker>,
) -> Result<BookInfo, String> {
    progress.flush(); // the comic being read is about to be replaced
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
    let mut info = book.info();
    let found = progress.store().find(
        book.fingerprint(),
        &book.source_path().to_string_lossy(),
        &book.page_names(),
    );
    info.resume = found.map(|e| Resume { page_index: e.page_index, finished: e.finished });
    if state.install(book) {
        progress.flush(); // reports that arrived while the new comic opened
        Ok(info)
    } else {
        Err(SUPERSEDED.into())
    }
}

/// Records the reader's position in memory. It reaches the disk on `flush_progress`,
/// when another comic opens, or when the window closes or the app quits.
#[tauri::command]
pub fn report_progress(
    book_id: u64,
    page_index: usize,
    finished: bool,
    state: State<'_, AppState>,
    progress: State<'_, ProgressTracker>,
) {
    let Some(book) = state.current().filter(|b| b.id == book_id) else { return };
    let Some(page_name) = book.page_name(page_index) else { return };
    progress.report(Report {
        fingerprint: book.fingerprint().to_owned(),
        source_path: book.source_path().to_string_lossy().into_owned(),
        page_name: page_name.to_owned(),
        page_index,
        finished,
    });
}

#[tauri::command]
pub async fn flush_progress(progress: State<'_, ProgressTracker>) -> Result<(), String> {
    progress.flush();
    Ok(())
}

#[tauri::command]
pub fn take_pending_open(pending: State<'_, PendingOpen>) -> Option<String> {
    pending.take()
}
