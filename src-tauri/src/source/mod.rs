//! Page sources: a folder, a zip, or an archive extracted to a temporary folder.

mod folder;

pub use folder::FolderSource;

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SourceError {
    Io(io::Error),
    Unsupported,
    Corrupt(String),
    Encrypted,
    NotFound(String),
}

impl From<io::Error> for SourceError {
    fn from(err: io::Error) -> Self {
        SourceError::Io(err)
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Io(err) => write!(f, "I/O error: {err}"),
            SourceError::Unsupported => write!(f, "unsupported source"),
            SourceError::Corrupt(detail) => write!(f, "damaged archive: {detail}"),
            SourceError::Encrypted => write!(f, "password protected"),
            SourceError::NotFound(path) => write!(f, "{path} not found"),
        }
    }
}

/// A set of files that can be listed and read. Paths are relative and `/`-separated.
pub trait PageSource: Send + Sync {
    fn list(&self) -> Result<Vec<String>, SourceError>;
    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError>;
    /// Reads at most `limit` bytes from the start of the file.
    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Folder,
    Zip,
    Rar,
    TarGz,
}

pub fn kind_from_magic(head: &[u8]) -> Option<SourceKind> {
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Some(SourceKind::Zip)
    } else if head.starts_with(b"Rar!\x1a\x07") {
        Some(SourceKind::Rar)
    } else if head.starts_with(&[0x1f, 0x8b]) {
        Some(SourceKind::TarGz)
    } else {
        None
    }
}

pub fn kind_from_extension(name: &str) -> Option<SourceKind> {
    let name = name.to_ascii_lowercase();
    if name.ends_with(".zip") || name.ends_with(".cbz") {
        Some(SourceKind::Zip)
    } else if name.ends_with(".rar") || name.ends_with(".cbr") {
        Some(SourceKind::Rar)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(SourceKind::TarGz)
    } else {
        None
    }
}

/// Detects the source type: folder, then magic bytes, then extension.
pub fn detect(path: &Path) -> Result<SourceKind, SourceError> {
    if path.is_dir() {
        return Ok(SourceKind::Folder);
    }
    let mut head = Vec::with_capacity(8);
    File::open(path)?.take(8).read_to_end(&mut head)?;
    kind_from_magic(&head)
        .or_else(|| kind_from_extension(&path.to_string_lossy()))
        .ok_or(SourceError::Unsupported)
}

/// Converts an entry name to a relative path. Returns `None` for absolute
/// paths, drive prefixes, `..` segments, and empty names.
pub fn safe_relative_path(name: &str) -> Option<PathBuf> {
    if name.starts_with('/') || name.starts_with('\\') {
        return None;
    }
    let mut out = PathBuf::new();
    for seg in name.split(['/', '\\']) {
        match seg {
            "" | "." => continue,
            ".." => return None,
            s if s.contains(':') => return None,
            s => out.push(s),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

pub fn open_source(path: &Path, _temp_base: &Path) -> Result<Box<dyn PageSource>, SourceError> {
    match detect(path)? {
        SourceKind::Folder => Ok(Box::new(FolderSource::new(path))),
        _ => Err(SourceError::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes() {
        assert_eq!(kind_from_magic(b"PK\x03\x04rest"), Some(SourceKind::Zip));
        assert_eq!(kind_from_magic(b"PK\x05\x06"), Some(SourceKind::Zip));
        assert_eq!(kind_from_magic(b"Rar!\x1a\x07\x00"), Some(SourceKind::Rar));
        assert_eq!(kind_from_magic(b"Rar!\x1a\x07\x01\x00"), Some(SourceKind::Rar));
        assert_eq!(kind_from_magic(&[0x1f, 0x8b, 8, 0]), Some(SourceKind::TarGz));
        assert_eq!(kind_from_magic(b"hello"), None);
        assert_eq!(kind_from_magic(b""), None);
    }

    #[test]
    fn extensions() {
        assert_eq!(kind_from_extension("A.CBZ"), Some(SourceKind::Zip));
        assert_eq!(kind_from_extension("a.zip"), Some(SourceKind::Zip));
        assert_eq!(kind_from_extension("a.cbr"), Some(SourceKind::Rar));
        assert_eq!(kind_from_extension("a.RAR"), Some(SourceKind::Rar));
        assert_eq!(kind_from_extension("a.tar.gz"), Some(SourceKind::TarGz));
        assert_eq!(kind_from_extension("a.tgz"), Some(SourceKind::TarGz));
        assert_eq!(kind_from_extension("a.7z"), None);
    }

    #[test]
    fn safe_paths() {
        assert_eq!(safe_relative_path("a/b.png"), Some(PathBuf::from("a").join("b.png")));
        assert_eq!(safe_relative_path("./a\\b.png"), Some(PathBuf::from("a").join("b.png")));
        assert_eq!(safe_relative_path("../b.png"), None);
        assert_eq!(safe_relative_path("a/../../b.png"), None);
        assert_eq!(safe_relative_path("/etc/passwd"), None);
        assert_eq!(safe_relative_path("\\windows\\x.png"), None);
        assert_eq!(safe_relative_path("C:/x.png"), None);
        assert_eq!(safe_relative_path("./"), None);
    }
}
