use super::{safe_relative_path, PageSource, SourceError};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub struct FolderSource {
    root: PathBuf,
}

impl FolderSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, SourceError> {
        let rel = safe_relative_path(path).ok_or_else(|| SourceError::NotFound(path.to_owned()))?;
        Ok(self.root.join(rel))
    }
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk(&entry.path(), &rel, out)?;
        } else if file_type.is_file() || (file_type.is_symlink() && entry.path().is_file()) {
            out.push(rel);
        }
    }
    Ok(())
}

fn not_found_or_io(path: &str, err: io::Error) -> SourceError {
    if err.kind() == io::ErrorKind::NotFound {
        SourceError::NotFound(path.to_owned())
    } else {
        SourceError::Io(err)
    }
}

impl PageSource for FolderSource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        let mut out = Vec::new();
        walk(&self.root, "", &mut out)?;
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        fs::read(self.resolve(path)?).map_err(|e| not_found_or_io(path, e))
    }

    fn size(&self, path: &str) -> Result<u64, SourceError> {
        fs::metadata(self.resolve(path)?).map(|m| m.len()).map_err(|e| not_found_or_io(path, e))
    }

    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError> {
        let file = File::open(self.resolve(path)?).map_err(|e| not_found_or_io(path, e))?;
        let mut buf = Vec::new();
        file.take(limit as u64).read_to_end(&mut buf)?;
        Ok(buf)
    }
}
