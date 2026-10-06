//! The comic:// scheme: `/{book_id}/{index}` returns one page of the current book.
//! `convertFileSrc` encodes `/` as `%2F`, so both forms are accepted.

use crate::book::Book;
use crate::image_entry::content_type;
use tauri::http::{header, Response};

pub fn parse_page_path(path: &str) -> Option<(u64, usize)> {
    let decoded = path.trim_start_matches('/').replace("%2F", "/").replace("%2f", "/");
    let (id, index) = decoded.split_once('/')?;
    Some((id.parse().ok()?, index.parse().ok()?))
}

fn status(code: u16) -> Response<Vec<u8>> {
    Response::builder().status(code).body(Vec::new()).unwrap()
}

pub fn page_response(book: Option<&Book>, path: &str) -> Response<Vec<u8>> {
    let Some((id, index)) = parse_page_path(path) else { return status(400) };
    let Some(book) = book.filter(|b| b.id == id) else { return status(404) };
    let Some(name) = book.page_name(index) else { return status(404) };
    match book.read_page(index) {
        Ok(bytes) => Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, content_type(name))
            .body(bytes)
            .unwrap(),
        Err(_) => status(500),
    }
}
