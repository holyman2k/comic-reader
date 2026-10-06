#![allow(dead_code)]

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

/// Signature + IHDR chunk. Enough for header probing; not a decodable image.
pub fn png(width: u32, height: u32) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    v
}

pub fn write_folder(root: &Path, files: &[(&str, Vec<u8>)]) {
    for (name, data) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, data).unwrap();
    }
}

pub fn zip_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in files {
        w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// Writes raw names into the tar header, so tests can use `./` and `../` names.
pub fn tar_gz_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(enc);
    for (name, data) in files {
        let mut header = tar::Header::new_gnu();
        let raw = name.as_bytes();
        header.as_old_mut().name[..raw.len()].copy_from_slice(raw);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder.append(&header, &data[..]).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn temp_base() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
