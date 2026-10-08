//! A book: the sorted, sized pages of one source.

use crate::fingerprint::fingerprint;
use crate::image_entry::{is_page_entry, page_size};
use crate::natural_sort::natural_cmp;
use crate::source::{open_source, PageSource, SourceError};
use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

/// Header bytes read first, then once more if the size is not found (large EXIF blocks).
const PROBE_LIMITS: [usize; 2] = [64 * 1024, 1024 * 1024];
const ARCHIVE_EXTENSIONS: [&str; 6] = [".tar.gz", ".tgz", ".zip", ".cbz", ".rar", ".cbr"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageInfo {
    pub name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookInfo {
    pub book_id: u64,
    pub title: String,
    pub pages: Vec<PageInfo>,
    pub fingerprint: String,
    /// Where to resume. Set when the open command finds stored progress.
    pub resume: Option<Resume>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resume {
    pub page_index: usize,
    pub finished: bool,
}

#[derive(Debug)]
pub enum OpenError {
    Source(SourceError),
    NoImages,
}

impl OpenError {
    /// A message for the user. `name` is the file or folder name.
    pub fn message(&self, name: &str) -> String {
        match self {
            OpenError::NoImages => format!("No images found in {name}."),
            OpenError::Source(SourceError::Encrypted) => format!(
                "{name} is password protected. Password-protected archives are not supported."
            ),
            OpenError::Source(SourceError::Corrupt(_)) => {
                format!("{name} is damaged or is not a valid archive.")
            }
            OpenError::Source(SourceError::Unsupported) => {
                format!("{name} is not a supported comic file or folder.")
            }
            OpenError::Source(SourceError::Io(err)) if err.kind() == io::ErrorKind::StorageFull => {
                format!("Not enough disk space to open {name}.")
            }
            OpenError::Source(SourceError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                format!("{name} was not found.")
            }
            OpenError::Source(err) => format!("Could not open {name}: {err}."),
        }
    }
}

pub struct Book {
    pub id: u64,
    pub title: String,
    pages: Vec<PageInfo>,
    fingerprint: String,
    source_path: PathBuf,
    source: Box<dyn PageSource>,
}

impl Book {
    pub fn open(path: &Path, id: u64, temp_base: &Path) -> Result<Book, OpenError> {
        let source = open_source(path, temp_base).map_err(OpenError::Source)?;
        let mut names: Vec<String> = source
            .list()
            .map_err(OpenError::Source)?
            .into_iter()
            .filter(|name| is_page_entry(name))
            .collect();
        if names.is_empty() {
            return Err(OpenError::NoImages);
        }
        names.sort_by(|a, b| natural_cmp(a, b));
        let pages: Vec<PageInfo> = names
            .into_iter()
            .map(|name| {
                let size = probe_size(source.as_ref(), &name);
                PageInfo { width: size.map(|s| s.0), height: size.map(|s| s.1), name }
            })
            .collect();
        let sized = pages
            .iter()
            // A page whose size cannot be read must not stop the comic from opening.
            .map(|page| (page.name.clone(), source.size(&page.name).unwrap_or(0)))
            .collect::<Vec<_>>();
        Ok(Book {
            id,
            title: title_from_path(path),
            pages,
            fingerprint: fingerprint(&sized),
            source_path: path.to_path_buf(),
            source,
        })
    }

    pub fn info(&self) -> BookInfo {
        BookInfo {
            book_id: self.id,
            title: self.title.clone(),
            pages: self.pages.clone(),
            fingerprint: self.fingerprint.clone(),
            resume: None,
        }
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn page_names(&self) -> Vec<String> {
        self.pages.iter().map(|p| p.name.clone()).collect()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn page_name(&self, index: usize) -> Option<&str> {
        self.pages.get(index).map(|p| p.name.as_str())
    }

    pub fn read_page(&self, index: usize) -> Result<Vec<u8>, SourceError> {
        let name = self.page_name(index).ok_or_else(|| SourceError::NotFound(index.to_string()))?;
        self.source.read(name)
    }
}

fn probe_size(source: &dyn PageSource, name: &str) -> Option<(u32, u32)> {
    for limit in PROBE_LIMITS {
        let bytes = source.read_prefix(name, limit).ok()?;
        if let Some(size) = page_size(&bytes) {
            return Some(size);
        }
        if bytes.len() < limit {
            return None; // whole file read; a larger limit cannot help
        }
    }
    None
}

pub fn title_from_path(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    if path.is_dir() {
        return name;
    }
    for ext in ARCHIVE_EXTENSIONS {
        let cut = name.len().saturating_sub(ext.len());
        if name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(ext) && cut > 0 {
            return name[..cut].to_owned();
        }
    }
    name
}
