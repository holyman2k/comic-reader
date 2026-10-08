//! A comic's identity: a hash of its page list (names and byte sizes).
//! See `docs/adr/0001-comic-identity-by-page-list-fingerprint.md`.
//! Changing the scheme orphans all stored progress.

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const FNV_BASIS_A: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_BASIS_B: u64 = 0x8422_2325_cbf2_9ce4;

/// Removes the leading directories that every name shares.
fn strip_common_dir(names: &[&str]) -> usize {
    let mut depth = 0;
    loop {
        let first = match names.first().and_then(|n| n.split('/').nth(depth)) {
            Some(seg) => seg,
            None => return depth,
        };
        let all_share = names.iter().all(|n| {
            let mut segs = n.split('/').skip(depth);
            segs.next() == Some(first) && segs.next().is_some()
        });
        if !all_share {
            return depth;
        }
        depth += 1;
    }
}

fn fnv1a(basis: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(basis, |hash, b| (hash ^ u64::from(*b)).wrapping_mul(FNV_PRIME))
}

/// Fingerprint of `pages`, each a `(name, byte size)` pair. Order does not matter.
pub fn fingerprint(pages: &[(String, u64)]) -> String {
    let names: Vec<&str> = pages.iter().map(|(n, _)| n.as_str()).collect();
    let depth = strip_common_dir(&names);
    let mut entries: Vec<(String, u64)> = pages
        .iter()
        .map(|(name, size)| (name.split('/').skip(depth).collect::<Vec<_>>().join("/"), *size))
        .collect();
    entries.sort();
    let mut data = Vec::new();
    for (name, size) in &entries {
        data.extend_from_slice(name.as_bytes());
        data.push(0);
        data.extend_from_slice(size.to_string().as_bytes());
        data.push(b'\n');
    }
    format!("{:016x}{:016x}", fnv1a(FNV_BASIS_A, &data), fnv1a(FNV_BASIS_B, &data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(list: &[(&str, u64)]) -> Vec<(String, u64)> {
        list.iter().map(|(n, s)| (n.to_string(), *s)).collect()
    }

    #[test]
    fn archive_and_unzipped_folder_match() {
        let archive = pages(&[("Comic/001.jpg", 10), ("Comic/002.jpg", 20)]);
        let folder = pages(&[("001.jpg", 10), ("002.jpg", 20)]);
        assert_eq!(fingerprint(&archive), fingerprint(&folder));
    }

    #[test]
    fn nested_common_dirs_are_removed() {
        let a = pages(&[("A/B/1.jpg", 1), ("A/B/2.jpg", 2)]);
        let b = pages(&[("1.jpg", 1), ("2.jpg", 2)]);
        assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn differing_subdirs_are_kept() {
        let a = pages(&[("A/1.jpg", 1), ("B/1.jpg", 1)]);
        let b = pages(&[("A/1.jpg", 1), ("C/1.jpg", 1)]);
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn order_does_not_matter() {
        let a = pages(&[("1.jpg", 1), ("2.jpg", 2)]);
        let b = pages(&[("2.jpg", 2), ("1.jpg", 1)]);
        assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn size_name_and_count_change_the_fingerprint() {
        let base = pages(&[("1.jpg", 1), ("2.jpg", 2)]);
        assert_ne!(fingerprint(&base), fingerprint(&pages(&[("1.jpg", 1), ("2.jpg", 3)])));
        assert_ne!(fingerprint(&base), fingerprint(&pages(&[("1.jpg", 1), ("3.jpg", 2)])));
        assert_ne!(fingerprint(&base), fingerprint(&pages(&[("1.jpg", 1)])));
    }

    #[test]
    fn single_page_keeps_its_file_name() {
        let a = pages(&[("Comic/1.jpg", 5)]);
        let b = pages(&[("1.jpg", 5)]);
        assert_eq!(fingerprint(&a), fingerprint(&b));
        assert_ne!(fingerprint(&b), fingerprint(&pages(&[("2.jpg", 5)])));
    }

    #[test]
    fn value_is_stable() {
        // Stored progress depends on this value never changing.
        assert_eq!(fingerprint(&pages(&[("1.jpg", 1), ("2.jpg", 2)])), "37383feb0fa0f1b97be03a2b2cd235e0");
    }
}
