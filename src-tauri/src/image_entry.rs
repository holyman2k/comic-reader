//! Rules for which archive or folder entries are pages, and header probing.

const IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "webp", "avif", "bmp"];

/// Converts `\` to `/` and drops empty and `.` segments. `..` is kept so that
/// callers can still reject it.
pub fn normalize_entry_name(name: &str) -> String {
    name.split(['/', '\\'])
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect::<Vec<_>>()
        .join("/")
}

fn extension(path: &str) -> Option<String> {
    let file = path.rsplit('/').next()?;
    let (stem, ext) = file.rsplit_once('.')?;
    (!stem.is_empty() || !ext.is_empty()).then(|| ext.to_ascii_lowercase())
}

fn is_excluded(path: &str) -> bool {
    path.split('/')
        .any(|seg| seg == "__MACOSX" || (seg.starts_with('.') && seg != "." && seg != ".."))
}

/// True when `path` (normalized, `/`-separated) is an image that is not excluded.
pub fn is_page_entry(path: &str) -> bool {
    let is_image = extension(path).is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.as_str()));
    is_image && !is_excluded(path)
}

pub fn content_type(path: &str) -> &'static str {
    match extension(path).as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Width and height from the image header, or `None` if it cannot be read.
pub fn page_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let size = imagesize::blob_size(bytes).ok()?;
    if size.width == 0 || size.height == 0 {
        return None;
    }
    Some((u32::try_from(size.width).ok()?, u32::try_from(size.height).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_and_dot_segments() {
        assert_eq!(normalize_entry_name("ch1\\p2.png"), "ch1/p2.png");
        assert_eq!(normalize_entry_name("./ch1//p2.png"), "ch1/p2.png");
        assert_eq!(normalize_entry_name("../x.png"), "../x.png");
    }

    #[test]
    fn accepts_image_extensions_in_any_case() {
        for name in ["a.jpg", "a.JPEG", "b/c.Png", "d.gif", "e.webp", "f.AVIF", "g.bmp"] {
            assert!(is_page_entry(name), "{name}");
        }
    }

    #[test]
    fn rejects_other_files_and_folder_dots() {
        for name in ["notes.txt", "png", "a.png/readme", "dir.jpg/file", "a.tiff", ""] {
            assert!(!is_page_entry(name), "{name}");
        }
    }

    #[test]
    fn excludes_hidden_and_macosx_entries() {
        for name in [".hidden.png", "._p1.png", ".git/x.png", "__MACOSX/p1.png", "a/__MACOSX/b.png"] {
            assert!(!is_page_entry(name), "{name}");
        }
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type("a/B.JPG"), "image/jpeg");
        assert_eq!(content_type("a.jpeg"), "image/jpeg");
        assert_eq!(content_type("a.png"), "image/png");
        assert_eq!(content_type("a.gif"), "image/gif");
        assert_eq!(content_type("a.webp"), "image/webp");
        assert_eq!(content_type("a.avif"), "image/avif");
        assert_eq!(content_type("a.bmp"), "image/bmp");
        assert_eq!(content_type("a.txt"), "application/octet-stream");
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v
    }

    #[test]
    fn reads_png_size() {
        assert_eq!(page_size(&png(800, 1200)), Some((800, 1200)));
    }

    #[test]
    fn reads_gif_size() {
        let gif = [b'G', b'I', b'F', b'8', b'9', b'a', 0x20, 0x03, 0x58, 0x02, 0, 0, 0];
        assert_eq!(page_size(&gif), Some((800, 600)));
    }

    #[test]
    fn reads_bmp_size() {
        let mut bmp = vec![b'B', b'M'];
        bmp.extend_from_slice(&[0; 12]); // file size, reserved, data offset
        bmp.extend_from_slice(&40u32.to_le_bytes()); // DIB header size
        bmp.extend_from_slice(&640i32.to_le_bytes());
        bmp.extend_from_slice(&480i32.to_le_bytes());
        bmp.extend_from_slice(&[1, 0, 24, 0]);
        bmp.extend_from_slice(&[0; 24]);
        assert_eq!(page_size(&bmp), Some((640, 480)));
    }

    #[test]
    fn reads_jpeg_size() {
        // SOI, then a baseline SOF0 segment: length 17, precision 8, height 300, width 200.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x01, 0x2C, 0x00, 0xC8, 0x03, 0x01, 0x22,
            0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xD9,
        ];
        assert_eq!(page_size(&jpeg), Some((200, 300)));
    }

    #[test]
    fn reads_webp_size() {
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&30u32.to_le_bytes());
        webp.extend_from_slice(b"WEBPVP8X");
        webp.extend_from_slice(&10u32.to_le_bytes());
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(&[0x1F, 0x03, 0x00]); // width - 1 = 799
        webp.extend_from_slice(&[0xAF, 0x04, 0x00]); // height - 1 = 1199
        assert_eq!(page_size(&webp), Some((800, 1200)));
    }

    #[test]
    fn unreadable_header_has_no_size() {
        assert_eq!(page_size(&[]), None);
        assert_eq!(page_size(b"not an image"), None);
        assert_eq!(page_size(&png(800, 1200)[..10]), None);
        assert_eq!(page_size(&png(0, 1200)), None);
    }
}
