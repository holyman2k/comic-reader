//! One temporary folder per running app instance, protected by a file lock.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

const PREFIX: &str = "inst-";
const LOCK_FILE: &str = ".lock";
const CREATE_LOCK: &str = ".create.lock";

pub struct InstanceDir {
    path: PathBuf,
    lock: Option<File>,
}

impl InstanceDir {
    pub fn create(base: &Path) -> io::Result<Self> {
        fs::create_dir_all(base)?;
        let _guard = lock_base(base)?;
        let path = tempfile::Builder::new().prefix(PREFIX).tempdir_in(base)?.keep();
        let lock = File::create(path.join(LOCK_FILE))?;
        lock.try_lock().map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Self { path, lock: Some(lock) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Releases the lock and deletes the folder. Windows cannot delete an open
    /// file, so the lock file is closed first.
    pub fn remove(mut self) {
        drop(self.lock.take());
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn default_base() -> PathBuf {
    std::env::temp_dir().join("comic-reader")
}

/// Takes the exclusive base lock. `create` makes the folder, makes `.lock` and
/// locks it in three steps. Without this lock, a second instance could run
/// `cleanup_stale` between those steps, see an unlocked folder, and delete it.
/// The lock is released when the returned file is dropped.
fn lock_base(base: &Path) -> io::Result<File> {
    let file = OpenOptions::new().write(true).create(true).truncate(false).open(base.join(CREATE_LOCK))?;
    file.lock()?;
    Ok(file)
}

fn is_unlocked(dir: &Path) -> bool {
    match OpenOptions::new().write(true).open(dir.join(LOCK_FILE)) {
        Err(_) => true,
        Ok(file) => file.try_lock().is_ok(),
    }
}

/// Deletes instance folders left behind by app instances that are no longer running.
pub fn cleanup_stale(base: &Path) {
    if !base.is_dir() {
        return;
    }
    let Ok(_guard) = lock_base(base) else { return };
    let Ok(entries) = fs::read_dir(base) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_instance = entry.file_name().to_string_lossy().starts_with(PREFIX) && path.is_dir();
        if is_instance && is_unlocked(&path) {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_makes_locked_folder_under_base() {
        let base = tempfile::tempdir().unwrap();
        let inst = InstanceDir::create(base.path()).unwrap();
        assert!(inst.path().starts_with(base.path()));
        assert!(inst.path().join(LOCK_FILE).is_file());
        assert!(!is_unlocked(inst.path()));
    }

    #[test]
    fn cleanup_keeps_live_and_removes_stale() {
        let base = tempfile::tempdir().unwrap();
        let live = InstanceDir::create(base.path()).unwrap();

        let stale = base.path().join("inst-stale");
        fs::create_dir_all(stale.join("book-1")).unwrap();
        fs::write(stale.join(LOCK_FILE), b"").unwrap();
        fs::write(stale.join("book-1/p1.png"), b"x").unwrap();

        let no_lock = base.path().join("inst-nolock");
        fs::create_dir_all(&no_lock).unwrap();

        let other = base.path().join("unrelated");
        fs::create_dir_all(&other).unwrap();

        cleanup_stale(base.path());

        assert!(live.path().is_dir());
        assert!(!stale.exists());
        assert!(!no_lock.exists());
        assert!(other.is_dir());
        assert!(base.path().join(CREATE_LOCK).exists());
    }

    #[test]
    fn create_after_cleanup_succeeds_and_is_locked() {
        let base = tempfile::tempdir().unwrap();
        cleanup_stale(base.path());
        let inst = InstanceDir::create(base.path()).unwrap();
        assert!(!is_unlocked(inst.path()));
    }

    #[test]
    fn remove_deletes_folder() {
        let base = tempfile::tempdir().unwrap();
        let inst = InstanceDir::create(base.path()).unwrap();
        let path = inst.path().to_path_buf();
        fs::write(path.join("x"), b"x").unwrap();
        inst.remove();
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_of_missing_base_does_nothing() {
        let base = tempfile::tempdir().unwrap();
        let missing = base.path().join("does-not-exist");
        cleanup_stale(&missing);
        assert!(!missing.exists());
    }
}
