mod common;

use comic_reader_lib::book::{title_from_path, Book, OpenError};
use comic_reader_lib::source::SourceError;
use common::*;
use std::path::Path;

fn names(book: &Book) -> Vec<String> {
    book.info().pages.into_iter().map(|p| p.name).collect()
}

#[test]
fn folder_book_is_filtered_and_sorted() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(
        dir.path(),
        &[
            ("page 10.png", png(10, 10)),
            ("page 2.png", png(2, 2)),
            ("page 1.png", png(1, 1)),
            ("notes.txt", b"x".to_vec()),
            ("._page 1.png", b"appledouble".to_vec()),
            ("__MACOSX/page 3.png", png(3, 3)),
        ],
    );
    let base = temp_base();
    let book = Book::open(dir.path(), 7, base.path()).unwrap();
    assert_eq!(book.id, 7);
    assert_eq!(names(&book), ["page 1.png", "page 2.png", "page 10.png"]);
    assert_eq!(book.page_count(), 3);
    assert_eq!(book.page_name(2), Some("page 10.png"));
    assert_eq!(book.page_name(3), None);
    assert_eq!(book.read_page(1).unwrap(), png(2, 2));
}

#[test]
fn page_sizes_come_from_headers() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("a.png", png(800, 1200))]);
    let base = temp_base();
    let info = Book::open(dir.path(), 1, base.path()).unwrap().info();
    assert_eq!(info.pages[0].width, Some(800));
    assert_eq!(info.pages[0].height, Some(1200));
}

#[test]
fn empty_and_truncated_pages_have_unknown_size_but_book_opens() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("1.png", Vec::new()), ("2.png", png(5, 5)[..10].to_vec()), ("3.png", png(4, 6))]);
    let base = temp_base();
    let book = Book::open(dir.path(), 1, base.path()).unwrap();
    let info = book.info();
    assert_eq!((info.pages[0].width, info.pages[0].height), (None, None));
    assert_eq!((info.pages[1].width, info.pages[1].height), (None, None));
    assert_eq!((info.pages[2].width, info.pages[2].height), (Some(4), Some(6)));
    assert_eq!(book.read_page(0).unwrap(), Vec::<u8>::new());
}

#[test]
fn rar_book_sorts_across_folders() {
    let base = temp_base();
    let book = Book::open(&fixture("book.cbr"), 1, base.path()).unwrap();
    assert_eq!(names(&book), ["ch2/page 2.png", "ch2/page 10.png", "ch10/page 1.png"]);
    let info = book.info();
    assert_eq!((info.pages[0].width, info.pages[0].height), (Some(20), Some(30)));
    assert_eq!(info.title, "book");
}

#[test]
fn zip_book_reads_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Vol 1.CBZ");
    std::fs::write(&path, zip_bytes(&[("p2.png", png(2, 3)), ("p1.png", png(1, 2))])).unwrap();
    let base = temp_base();
    let info = Book::open(&path, 1, base.path()).unwrap().info();
    assert_eq!(info.title, "Vol 1");
    assert_eq!(info.pages[0].name, "p1.png");
    assert_eq!((info.pages[0].width, info.pages[0].height), (Some(1), Some(2)));
}

#[test]
fn source_without_images_is_no_images() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("notes.txt", b"x".to_vec())]);
    let base = temp_base();
    assert!(matches!(Book::open(dir.path(), 1, base.path()), Err(OpenError::NoImages)));
}

#[test]
fn plain_gzip_is_an_error_not_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("single.gz");
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut enc, &[b'x'; 3000]).unwrap();
    std::fs::write(&path, enc.finish().unwrap()).unwrap();
    let base = temp_base();
    assert!(Book::open(&path, 1, base.path()).is_err());
}

#[test]
fn error_messages_name_the_source() {
    let name = "Vol 1.cbz";
    assert_eq!(OpenError::NoImages.message(name), "No images found in Vol 1.cbz.");
    assert_eq!(
        OpenError::Source(SourceError::Encrypted).message(name),
        "Vol 1.cbz is password protected. Password-protected archives are not supported."
    );
    assert_eq!(
        OpenError::Source(SourceError::Corrupt("x".into())).message(name),
        "Vol 1.cbz is damaged or is not a valid archive."
    );
    assert_eq!(
        OpenError::Source(SourceError::Unsupported).message(name),
        "Vol 1.cbz is not a supported comic file or folder."
    );
    let full = std::io::Error::from(std::io::ErrorKind::StorageFull);
    assert_eq!(
        OpenError::Source(SourceError::Io(full)).message(name),
        "Not enough disk space to open Vol 1.cbz."
    );
}

#[test]
fn titles_strip_archive_extensions() {
    assert_eq!(title_from_path(Path::new("/a/Vol 1.tar.gz")), "Vol 1");
    assert_eq!(title_from_path(Path::new("/a/Vol 1.TGZ")), "Vol 1");
    assert_eq!(title_from_path(Path::new("/a/One.Piece.cbr")), "One.Piece");
    assert_eq!(title_from_path(Path::new("/a/notes.txt")), "notes.txt");
}
