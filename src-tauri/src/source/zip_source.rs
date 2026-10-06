use super::{PageSource, SourceError};
use crate::image_entry::normalize_entry_name;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use zip::result::ZipError;
use zip::ZipArchive;

pub struct ZipSource {
    archive: Mutex<ZipArchive<File>>,
    names: Vec<String>,
    index: HashMap<String, usize>,
}

fn map_zip_error(err: ZipError) -> SourceError {
    match err {
        ZipError::Io(err) => SourceError::Io(err),
        ZipError::InvalidPassword => SourceError::Encrypted,
        ZipError::UnsupportedArchive(msg) if msg == ZipError::PASSWORD_REQUIRED => {
            SourceError::Encrypted
        }
        other => SourceError::Corrupt(other.to_string()),
    }
}

impl ZipSource {
    pub fn open(path: &Path) -> Result<Self, SourceError> {
        let mut archive = ZipArchive::new(File::open(path)?).map_err(map_zip_error)?;
        let mut names = Vec::new();
        let mut index = HashMap::new();
        for i in 0..archive.len() {
            // by_index fails with PASSWORD_REQUIRED for encrypted entries.
            let entry = archive.by_index(i).map_err(map_zip_error)?;
            if entry.is_dir() {
                continue;
            }
            let name = normalize_entry_name(entry.name());
            if name.is_empty() {
                continue;
            }
            index.insert(name.clone(), i);
            names.push(name);
        }
        Ok(Self { archive: Mutex::new(archive), names, index })
    }

    fn read_limited(&self, path: &str, limit: Option<usize>) -> Result<Vec<u8>, SourceError> {
        let i = *self.index.get(path).ok_or_else(|| SourceError::NotFound(path.to_owned()))?;
        let mut archive = self.archive.lock().unwrap();
        let entry = archive.by_index(i).map_err(map_zip_error)?;
        let mut buf = Vec::new();
        match limit {
            Some(n) => entry.take(n as u64).read_to_end(&mut buf)?,
            None => {
                let mut entry = entry;
                entry.read_to_end(&mut buf)?
            }
        };
        Ok(buf)
    }
}

impl PageSource for ZipSource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        Ok(self.names.clone())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        self.read_limited(path, None)
    }

    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError> {
        self.read_limited(path, Some(limit))
    }
}
