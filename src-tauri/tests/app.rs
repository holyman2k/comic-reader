mod common;

use comic_reader_lib::book::Book;
use comic_reader_lib::protocol::{page_response, parse_page_path};
use comic_reader_lib::state::AppState;
use comic_reader_lib::temp::InstanceDir;
use common::*;

fn folder_with(files: &[(&str, Vec<u8>)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), files);
    dir
}

#[test]
fn parses_plain_and_encoded_paths() {
    assert_eq!(parse_page_path("/3/17"), Some((3, 17)));
    assert_eq!(parse_page_path("/3%2F17"), Some((3, 17)));
    assert_eq!(parse_page_path("/3%2f17"), Some((3, 17)));
    assert_eq!(parse_page_path("/x/1"), None);
    assert_eq!(parse_page_path("/3"), None);
    assert_eq!(parse_page_path("/3/-1"), None);
    assert_eq!(parse_page_path("/3/1/../../etc"), None);
}

#[test]
fn serves_pages_with_content_type() {
    let dir = folder_with(&[("b.png", png(2, 2)), ("a.jpg", b"jpegbytes".to_vec())]);
    let base = temp_base();
    let book = Book::open(dir.path(), 5, base.path()).unwrap();

    let res = page_response(Some(&book), "/5/0");
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["Content-Type"], "image/jpeg");
    assert_eq!(res.body(), b"jpegbytes");

    let res = page_response(Some(&book), "/5%2F1");
    assert_eq!(res.headers()["Content-Type"], "image/png");
}

#[test]
fn rejects_wrong_book_bad_index_and_no_book() {
    let dir = folder_with(&[("a.png", png(1, 1))]);
    let base = temp_base();
    let book = Book::open(dir.path(), 5, base.path()).unwrap();
    assert_eq!(page_response(Some(&book), "/4/0").status(), 404);
    assert_eq!(page_response(Some(&book), "/5/1").status(), 404);
    assert_eq!(page_response(None, "/5/0").status(), 404);
    assert_eq!(page_response(Some(&book), "/nonsense").status(), 400);
}

#[test]
fn page_deleted_after_open_is_server_error() {
    let dir = folder_with(&[("a.png", png(1, 1))]);
    let base = temp_base();
    let book = Book::open(dir.path(), 1, base.path()).unwrap();
    std::fs::remove_file(dir.path().join("a.png")).unwrap();
    assert_eq!(page_response(Some(&book), "/1/0").status(), 500);
}

#[test]
fn latest_open_wins_and_old_book_id_is_rejected() {
    let base = temp_base();
    let state = AppState::new(InstanceDir::create(base.path()).unwrap());
    let first = folder_with(&[("a.png", png(1, 1))]);
    let second = folder_with(&[("b.png", png(2, 2))]);

    let t1 = state.next_ticket();
    assert!(state.install(Book::open(first.path(), t1, state.temp_base()).unwrap()));
    assert_eq!(state.current().unwrap().id, t1);

    let t2 = state.next_ticket();
    let t3 = state.next_ticket();
    assert!(!state.is_latest(t2));
    // t2 finishes after t3 was requested: discarded.
    assert!(!state.install(Book::open(first.path(), t2, state.temp_base()).unwrap()));
    assert!(state.install(Book::open(second.path(), t3, state.temp_base()).unwrap()));

    let current = state.current().unwrap();
    assert_eq!(current.id, t3);
    // A late request for the first book's page must not return the new book's page.
    assert_eq!(page_response(Some(current.as_ref()), &format!("/{t1}/0")).status(), 404);
    assert_eq!(page_response(Some(current.as_ref()), &format!("/{t3}/0")).body(), &png(2, 2));
}

#[test]
fn shutdown_clears_book_and_removes_temp_folder() {
    let base = temp_base();
    let state = AppState::new(InstanceDir::create(base.path()).unwrap());
    let inst_path = state.temp_base().to_path_buf();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive = archive_dir.path().join("b.tgz");
    std::fs::write(&archive, tar_gz_bytes(&[("p1.png", png(1, 1))])).unwrap();

    let t = state.next_ticket();
    assert!(state.install(Book::open(&archive, t, state.temp_base()).unwrap()));
    state.shutdown();
    assert!(state.current().is_none());
    assert!(!inst_path.exists());
}
