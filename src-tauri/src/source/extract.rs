use super::{safe_relative_path, FolderSource, PageSource, SourceError};
use crate::image_entry::{is_page_entry, normalize_entry_name};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::Path;
use tempfile::TempDir;

/// An archive extracted to a temporary folder. The folder is deleted on drop.
pub struct ExtractedSource {
    _dir: TempDir,
    inner: FolderSource,
}

impl ExtractedSource {
    pub fn from_tar_gz(archive: &Path, temp_base: &Path) -> Result<Self, SourceError> {
        let dir = new_book_dir(temp_base)?;
        extract_tar_gz(archive, dir.path())?;
        Ok(Self::wrap(dir))
    }

    fn wrap(dir: TempDir) -> Self {
        let inner = FolderSource::new(dir.path());
        Self { _dir: dir, inner }
    }
}

impl PageSource for ExtractedSource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        self.inner.list()
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        self.inner.read(path)
    }

    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError> {
        self.inner.read_prefix(path, limit)
    }
}

fn new_book_dir(temp_base: &Path) -> io::Result<TempDir> {
    fs::create_dir_all(temp_base)?;
    tempfile::Builder::new().prefix("book-").tempdir_in(temp_base)
}

/// Where `name` should be written, or `None` if it is not a page or not safe.
fn target_for(dest: &Path, name: &str) -> Option<std::path::PathBuf> {
    if !is_page_entry(&normalize_entry_name(name)) {
        return None;
    }
    safe_relative_path(name).map(|rel| dest.join(rel))
}

fn write_reader(target: &Path, reader: &mut impl Read) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    io::copy(reader, &mut File::create(target)?)?;
    Ok(())
}

fn tar_error(err: io::Error) -> SourceError {
    match err.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput | io::ErrorKind::UnexpectedEof => {
            SourceError::Corrupt(err.to_string())
        }
        _ => SourceError::Io(err),
    }
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), SourceError> {
    let file = File::open(archive)?;
    let mut tar = tar::Archive::new(GzDecoder::new(BufReader::new(file)));
    for entry in tar.entries().map_err(tar_error)? {
        let mut entry = entry.map_err(tar_error)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        if let Some(target) = target_for(dest, &name) {
            write_reader(&target, &mut entry).map_err(tar_error)?;
        }
    }
    Ok(())
}
