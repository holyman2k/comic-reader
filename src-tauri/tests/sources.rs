mod common;

use comic_reader_lib::source::{detect, open_source, PageSource, SourceError, SourceKind};
use common::*;

fn sorted_list(source: &dyn PageSource) -> Vec<String> {
    let mut names = source.list().unwrap();
    names.sort();
    names
}

#[test]
fn folder_lists_files_recursively_with_slashes() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("p1.png", png(1, 2)), ("ch1/p2.png", png(3, 4)), ("notes.txt", b"x".to_vec())]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "notes.txt", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
    assert_eq!(source.read_prefix("ch1/p2.png", 8).unwrap(), png(3, 4)[..8].to_vec());
}

#[test]
fn folder_reads_unicode_names() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("第1話/ページ2.png", png(5, 6)), ("Élan.png", png(7, 8))]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    let names = sorted_list(source.as_ref());
    assert_eq!(names.len(), 2);
    for name in &names {
        assert!(!source.read(name).unwrap().is_empty(), "{name}");
    }
}

#[test]
fn folder_rejects_unsafe_and_missing_paths() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("p1.png", png(1, 2))]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    assert!(matches!(source.read("../p1.png"), Err(SourceError::NotFound(_))));
    assert!(matches!(source.read("missing.png"), Err(SourceError::NotFound(_))));
}

#[test]
fn detect_prefers_magic_bytes_over_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("really-a-zip.cbr");
    std::fs::write(&path, zip_bytes(&[("p1.png", png(1, 2))])).unwrap();
    assert_eq!(detect(&path).unwrap(), SourceKind::Zip);
    assert_eq!(detect(dir.path()).unwrap(), SourceKind::Folder);
}

#[test]
fn detect_unknown_file_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, b"hello").unwrap();
    assert!(matches!(detect(&path), Err(SourceError::Unsupported)));
}

#[test]
fn detect_missing_file_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(detect(&dir.path().join("gone.cbz")), Err(SourceError::Io(_))));
}
