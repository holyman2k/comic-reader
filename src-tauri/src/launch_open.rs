//! Paths that macOS asks the app to open (Dock drop, Finder "Open With").
//! The UI may not be ready yet, so the path waits here until the UI takes it.

use std::sync::Mutex;

pub const OPEN_EVENT: &str = "open-path";

#[derive(Default)]
pub struct PendingOpen(Mutex<Option<String>>);

impl PendingOpen {
    pub fn set(&self, path: String) {
        *self.0.lock().unwrap() = Some(path);
    }

    pub fn take(&self) -> Option<String> {
        self.0.lock().unwrap().take()
    }
}
