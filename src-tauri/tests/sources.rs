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

#[cfg(not(windows))]
#[test]
fn folder_reads_names_with_colons() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("Re:Zero/p1.png", png(1, 2))]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["Re:Zero/p1.png"]);
    assert_eq!(source.read("Re:Zero/p1.png").unwrap(), png(1, 2));
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

fn write_file(dir: &std::path::Path, name: &str, data: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, data).unwrap();
    path
}

#[test]
fn zip_lists_files_and_reads_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "book.cbz",
        &zip_bytes(&[("p1.png", png(1, 2)), ("ch1/p2.png", png(3, 4))]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
    assert_eq!(source.read_prefix("p1.png", 4).unwrap(), png(1, 2)[..4].to_vec());
    assert!(matches!(source.read("missing.png"), Err(SourceError::NotFound(_))));
}

#[test]
fn zip_backslash_names_become_folders() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "win.zip",
        &zip_bytes(&[("ch1\\p2.png", png(3, 4)), (".\\p1.png", png(1, 2))]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
}

#[test]
fn zip_with_wrong_extension_opens_as_zip() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "mislabeled.cbr", &zip_bytes(&[("p1.png", png(1, 2))]));
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(source.list().unwrap(), ["p1.png"]);
}

#[test]
fn zip_encrypted_is_reported() {
    use std::io::Write;
    use zip::unstable::write::FileOptionsExt;
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .with_deprecated_encryption(b"secret")
        .unwrap();
    w.start_file("p1.png", options).unwrap();
    w.write_all(&png(1, 2)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "locked.cbz", &w.finish().unwrap().into_inner());
    let base = temp_base();
    assert!(matches!(open_source(&path, base.path()), Err(SourceError::Encrypted)));
}

#[test]
fn zip_damaged_is_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let mut data = zip_bytes(&[("p1.png", png(1, 2))]);
    data.truncate(data.len() - 30); // cut into the central directory
    let path = write_file(dir.path(), "broken.cbz", &data);
    let base = temp_base();
    assert!(matches!(open_source(&path, base.path()), Err(SourceError::Corrupt(_))));
}

#[test]
fn tar_gz_extracts_only_pages() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "book.tar.gz",
        &tar_gz_bytes(&[
            ("ch1/p2.png", png(3, 4)),
            ("p1.png", png(1, 2)),
            ("notes.txt", b"not a page".to_vec()),
        ]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
}

#[test]
fn tar_gz_from_macos_tar_skips_appledouble_and_dot_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "mac.tgz",
        &tar_gz_bytes(&[
            ("./p1.png", png(1, 2)),
            ("./._p1.png", b"appledouble".to_vec()),
            ("./ch1/p2.png", png(3, 4)),
        ]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
}

#[test]
fn tar_gz_never_writes_outside_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "evil.tgz",
        &tar_gz_bytes(&[("../evil.png", png(1, 1)), ("ok.png", png(2, 2))]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(source.list().unwrap(), ["ok.png"]);
    assert!(!base.path().join("evil.png").exists());
    // The archive's own folder is book-*/ under base, so "../evil.png" would land in base.
    for entry in std::fs::read_dir(base.path()).unwrap() {
        assert_ne!(entry.unwrap().file_name(), "evil.png");
    }
}

#[test]
fn tar_gz_folder_is_deleted_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "book.tgz", &tar_gz_bytes(&[("p1.png", png(1, 2))]));
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 1);
    drop(source);
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 0);
}

#[test]
fn damaged_tar_gz_is_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let mut data = tar_gz_bytes(&[("p1.png", vec![7u8; 4000])]);
    data.truncate(data.len() / 2);
    let path = write_file(dir.path(), "broken.tgz", &data);
    let base = temp_base();
    assert!(matches!(open_source(&path, base.path()), Err(SourceError::Corrupt(_))));
}
