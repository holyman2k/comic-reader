//! Shared app state: the current book and the open-request tickets.

use crate::book::Book;
use crate::temp::InstanceDir;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    temp_base: PathBuf,
    instance: Mutex<Option<InstanceDir>>,
    current: Mutex<Option<Arc<Book>>>,
    latest: AtomicU64,
}

impl AppState {
    pub fn new(instance: InstanceDir) -> Self {
        Self {
            temp_base: instance.path().to_path_buf(),
            instance: Mutex::new(Some(instance)),
            current: Mutex::new(None),
            latest: AtomicU64::new(0),
        }
    }

    pub fn temp_base(&self) -> &Path {
        &self.temp_base
    }

    /// Starts an open request. The ticket is also the new book's id.
    pub fn next_ticket(&self) -> u64 {
        self.latest.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn is_latest(&self, ticket: u64) -> bool {
        self.latest.load(Ordering::SeqCst) == ticket
    }

    /// Makes `book` current if no newer request started. Otherwise drops it,
    /// which also deletes its temporary folder.
    pub fn install(&self, book: Book) -> bool {
        let mut current = self.current.lock().unwrap();
        if !self.is_latest(book.id) {
            return false;
        }
        *current = Some(Arc::new(book));
        true
    }

    pub fn current(&self) -> Option<Arc<Book>> {
        self.current.lock().unwrap().clone()
    }

    /// Drops the current book and cancels open requests that are still running.
    pub fn close_book(&self) {
        let mut current = self.current.lock().unwrap();
        self.next_ticket();
        current.take();
    }

    pub fn shutdown(&self) {
        self.current.lock().unwrap().take();
        if let Some(instance) = self.instance.lock().unwrap().take() {
            instance.remove();
        }
    }
}
