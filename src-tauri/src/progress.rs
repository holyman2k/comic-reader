//! Reading progress: one JSON file in the app data dir, one entry per comic.
//!
//! Progress is a convenience. Nothing here may stop a comic from opening, so
//! a corrupt file is set aside and write failures are logged by the caller.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub fingerprint: String,
    pub source_path: String,
    pub page_name: String,
    pub page_index: usize,
    pub finished: bool,
    pub last_opened: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileData {
    version: u32,
    entries: Vec<Entry>,
}

/// What the reader reached, as reported by the viewer.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub fingerprint: String,
    pub source_path: String,
    pub page_name: String,
    pub page_index: usize,
    pub finished: bool,
}

pub struct ProgressStore {
    file: PathBuf,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl ProgressStore {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }

    fn backup_path(&self) -> PathBuf {
        let mut name = self.file.file_name().unwrap_or_default().to_os_string();
        name.push(".bak");
        self.file.with_file_name(name)
    }

    /// Reads all entries. A missing file is empty. A corrupt file is moved to
    /// `<file>.bak` and treated as empty. Any other read error is returned, so
    /// a save never overwrites entries it could not read.
    fn load(&self) -> io::Result<Vec<Entry>> {
        let text = match fs::read_to_string(&self.file) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) if err.kind() == io::ErrorKind::InvalidData => {
                self.set_aside();
                return Ok(Vec::new());
            }
            Err(err) => return Err(err),
        };
        match serde_json::from_str::<FileData>(&text) {
            Ok(data) if data.version == VERSION => Ok(data.entries),
            _ => {
                self.set_aside();
                Ok(Vec::new())
            }
        }
    }

    fn set_aside(&self) {
        if let Err(err) = fs::rename(&self.file, self.backup_path()) {
            eprintln!("could not set aside corrupt progress file: {err}");
        }
    }

    /// Writes to a temporary file, then renames it over the real one.
    fn write(&self, entries: Vec<Entry>) -> io::Result<()> {
        if let Some(parent) = self.file.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&FileData { version: VERSION, entries })
            .map_err(io::Error::other)?;
        let mut tmp_name = self.file.file_name().unwrap_or_default().to_os_string();
        tmp_name.push(format!(".{}.tmp", std::process::id()));
        let tmp = self.file.with_file_name(tmp_name);
        let result = fs::write(&tmp, json).and_then(|()| fs::rename(&tmp, &self.file));
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    /// Finds the entry for a comic. A fingerprint match wins. Otherwise an
    /// entry with the same source path is reused when its saved page name is
    /// in `page_names`, and its fingerprint is updated.
    pub fn find(&self, fingerprint: &str, source_path: &str, page_names: &[String]) -> Option<Entry> {
        let mut entries = self.load().unwrap_or_else(|err| {
            eprintln!("could not read progress file: {err}");
            Vec::new()
        });
        if let Some(entry) = entries.iter().find(|e| e.fingerprint == fingerprint) {
            return Some(entry.clone());
        }
        let entry = entries.iter_mut().find(|e| {
            e.source_path == source_path && page_names.contains(&e.page_name)
        })?;
        entry.fingerprint = fingerprint.to_owned();
        // Pages were added or removed, so the saved page may have a new index.
        entry.page_index = page_names.iter().position(|n| *n == entry.page_name)?;
        let found = entry.clone();
        if let Err(err) = self.write(entries) {
            eprintln!("could not update progress fingerprint: {err}");
        }
        Some(found)
    }

    /// Stores a report. Re-reads the file and changes only this comic's entry.
    /// A comic without an entry gets one only when the reader passed page 1 or finished.
    pub fn save(&self, report: &Report) -> io::Result<()> {
        let mut entries = self.load()?;
        let existing = entries.iter().position(|e| e.fingerprint == report.fingerprint);
        if existing.is_none() && report.page_index == 0 && !report.finished {
            return Ok(());
        }
        let entry = Entry {
            fingerprint: report.fingerprint.clone(),
            source_path: report.source_path.clone(),
            page_name: report.page_name.clone(),
            page_index: report.page_index,
            finished: report.finished,
            last_opened: now(),
        };
        match existing {
            Some(i) => entries[i] = entry,
            None => entries.push(entry),
        }
        self.write(entries)
    }
}

/// Holds the latest report in memory and writes it on `flush`.
/// A flush with nothing new writes nothing.
pub struct ProgressTracker {
    store: ProgressStore,
    pending: Mutex<Option<Report>>,
}

impl ProgressTracker {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { store: ProgressStore::new(file), pending: Mutex::new(None) }
    }

    pub fn store(&self) -> &ProgressStore {
        &self.store
    }

    pub fn report(&self, report: Report) {
        *self.pending.lock().unwrap() = Some(report);
    }

    pub fn flush(&self) {
        let Some(report) = self.pending.lock().unwrap().take() else { return };
        if let Err(err) = self.store.save(&report) {
            eprintln!("could not save progress: {err}");
        }
    }
}

pub fn default_file(data_dir: &Path) -> PathBuf {
    data_dir.join("progress.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(fp: &str, path: &str, page: usize, finished: bool) -> Report {
        Report {
            fingerprint: fp.into(),
            source_path: path.into(),
            page_name: format!("{page}.jpg"),
            page_index: page,
            finished,
        }
    }

    fn names(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("{i}.jpg")).collect()
    }

    fn store() -> (tempfile::TempDir, ProgressStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = ProgressStore::new(dir.path().join("progress.json"));
        (dir, store)
    }

    #[test]
    fn saves_and_finds_by_fingerprint() {
        let (_dir, store) = store();
        store.save(&report("a", "/x.cbz", 12, false)).unwrap();
        let entry = store.find("a", "/elsewhere.cbz", &names(20)).unwrap();
        assert_eq!((entry.page_index, entry.finished), (12, false));
        assert_eq!(entry.page_name, "12.jpg");
        assert!(store.find("b", "/x2.cbz", &names(20)).is_none());
    }

    #[test]
    fn no_entry_until_page_one_is_passed() {
        let (dir, store) = store();
        store.save(&report("a", "/x.cbz", 0, false)).unwrap();
        assert!(!dir.path().join("progress.json").exists());
        store.save(&report("a", "/x.cbz", 1, false)).unwrap();
        assert!(store.find("a", "/x.cbz", &names(5)).is_some());
    }

    #[test]
    fn finishing_page_one_creates_an_entry() {
        let (_dir, store) = store();
        store.save(&report("a", "/x.cbz", 0, true)).unwrap();
        assert!(store.find("a", "/x.cbz", &names(1)).unwrap().finished);
    }

    #[test]
    fn existing_entry_accepts_page_one() {
        let (_dir, store) = store();
        store.save(&report("a", "/x.cbz", 12, false)).unwrap();
        store.save(&report("a", "/x.cbz", 0, false)).unwrap();
        assert_eq!(store.find("a", "/x.cbz", &names(20)).unwrap().page_index, 0);
    }

    #[test]
    fn save_keeps_other_entries() {
        let (_dir, store) = store();
        store.save(&report("a", "/a.cbz", 3, false)).unwrap();
        store.save(&report("b", "/b.cbz", 4, false)).unwrap();
        store.save(&report("a", "/a.cbz", 5, true)).unwrap();
        assert_eq!(store.find("a", "/a.cbz", &names(9)).unwrap().page_index, 5);
        assert_eq!(store.find("b", "/b.cbz", &names(9)).unwrap().page_index, 4);
    }

    #[test]
    fn path_fallback_reuses_entry_and_updates_fingerprint() {
        let (_dir, store) = store();
        store.save(&report("old", "/x.cbz", 12, false)).unwrap();
        let entry = store.find("new", "/x.cbz", &names(20)).unwrap();
        assert_eq!(entry.page_index, 12);
        // The entry now answers to the new fingerprint, and no longer to the old one.
        assert!(store.find("new", "/other.cbz", &[]).is_some());
        assert!(store.find("old", "/other.cbz", &[]).is_none());
    }

    #[test]
    fn path_fallback_follows_the_saved_page_to_its_new_index() {
        let (_dir, store) = store();
        store.save(&report("old", "/x.cbz", 12, false)).unwrap();
        // Two pages were inserted before "12.jpg".
        let mut pages = vec!["a.jpg".to_string(), "b.jpg".to_string()];
        pages.extend(names(20));
        let entry = store.find("new", "/x.cbz", &pages).unwrap();
        assert_eq!(entry.page_index, 14);
        assert_eq!(store.find("new", "/z.cbz", &[]).unwrap().page_index, 14);
    }

    #[test]
    fn unreadable_file_is_never_overwritten() {
        let (dir, store) = store();
        // A directory where the file should be: reading it fails, but not as missing or corrupt.
        fs::create_dir(dir.path().join("progress.json")).unwrap();
        assert!(store.save(&report("a", "/x.cbz", 3, false)).is_err());
        assert!(dir.path().join("progress.json").is_dir());
    }

    #[test]
    fn path_fallback_needs_the_saved_page_name() {
        let (_dir, store) = store();
        store.save(&report("old", "/x.cbz", 12, false)).unwrap();
        assert!(store.find("new", "/x.cbz", &names(5)).is_none());
        assert!(store.find("old", "/y.cbz", &[]).is_some());
    }

    #[test]
    fn fingerprint_match_wins_over_path_match() {
        let (_dir, store) = store();
        store.save(&report("a", "/old.cbz", 3, false)).unwrap();
        store.save(&report("b", "/new.cbz", 9, false)).unwrap();
        // Comic "a" now lives at the path where "b" was stored.
        let entry = store.find("a", "/new.cbz", &names(20)).unwrap();
        assert_eq!(entry.page_index, 3);
    }

    #[test]
    fn corrupt_file_is_set_aside() {
        let (dir, store) = store();
        let file = dir.path().join("progress.json");
        fs::write(&file, "{ not json").unwrap();
        assert!(store.find("a", "/x.cbz", &names(3)).is_none());
        assert_eq!(fs::read_to_string(dir.path().join("progress.json.bak")).unwrap(), "{ not json");
        assert!(!file.exists());
        store.save(&report("a", "/x.cbz", 2, false)).unwrap();
        assert_eq!(store.find("a", "/x.cbz", &names(3)).unwrap().page_index, 2);
    }

    #[test]
    fn unknown_version_is_set_aside() {
        let (dir, store) = store();
        fs::write(dir.path().join("progress.json"), r#"{"version":99,"entries":[]}"#).unwrap();
        assert!(store.find("a", "/x.cbz", &[]).is_none());
        assert!(dir.path().join("progress.json.bak").exists());
    }

    #[test]
    fn write_failure_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        // The data "dir" is a file, so nothing can be created below it.
        let blocker = dir.path().join("blocker");
        fs::write(&blocker, "x").unwrap();
        let store = ProgressStore::new(blocker.join("progress.json"));
        assert!(store.save(&report("a", "/x.cbz", 3, false)).is_err());
        assert!(store.find("a", "/x.cbz", &names(5)).is_none());
    }

    #[test]
    fn tracker_flush_writes_only_new_reports() {
        let (dir, store) = store();
        let file = dir.path().join("progress.json");
        let tracker = ProgressTracker { store, pending: Mutex::new(None) };
        tracker.flush();
        assert!(!file.exists());
        tracker.report(report("a", "/x.cbz", 4, false));
        tracker.flush();
        assert_eq!(tracker.store().find("a", "/x.cbz", &names(9)).unwrap().page_index, 4);
        fs::remove_file(&file).unwrap();
        tracker.flush();
        assert!(!file.exists(), "second flush must not rewrite");
    }

    #[test]
    fn tracker_keeps_the_latest_report() {
        let (_dir, store) = store();
        let tracker = ProgressTracker { store, pending: Mutex::new(None) };
        tracker.report(report("a", "/x.cbz", 4, false));
        tracker.report(report("a", "/x.cbz", 7, false));
        tracker.flush();
        assert_eq!(tracker.store().find("a", "/x.cbz", &names(9)).unwrap().page_index, 7);
    }
}
