//! Natural sort order for page paths: text runs compare case-insensitively,
//! digit runs compare by numeric value, one folder level at a time.

use std::cmp::Ordering;

#[derive(Clone, Copy)]
enum Token<'a> {
    Text(&'a str),
    Number(&'a str),
}

/// Compares two `/`-separated paths in natural order. The order is total:
/// it returns `Equal` only for identical strings.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let ta = path_tokens(a);
    let tb = path_tokens(b);
    primary(&ta, &tb)
        .then_with(|| leading_zeros(&ta, &tb))
        .then_with(|| a.cmp(b))
}

fn path_tokens(path: &str) -> Vec<Vec<Token<'_>>> {
    path.split('/').map(segment_tokens).collect()
}

fn segment_tokens(segment: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut run_is_digits: Option<bool> = None;
    for (i, ch) in segment.char_indices() {
        let digit = ch.is_ascii_digit();
        if let Some(prev) = run_is_digits {
            if prev != digit {
                out.push(make_token(&segment[start..i], prev));
                start = i;
            }
        }
        run_is_digits = Some(digit);
    }
    if let Some(prev) = run_is_digits {
        out.push(make_token(&segment[start..], prev));
    }
    out
}

fn make_token(s: &str, digits: bool) -> Token<'_> {
    if digits {
        Token::Number(s)
    } else {
        Token::Text(s)
    }
}

fn cmp_text(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
}

fn cmp_number(a: &str, b: &str) -> Ordering {
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

fn cmp_token(a: Token, b: Token) -> Ordering {
    match (a, b) {
        (Token::Number(x), Token::Number(y)) => cmp_number(x, y),
        (Token::Text(x), Token::Text(y)) => cmp_text(x, y),
        (Token::Number(_), Token::Text(_)) => Ordering::Less,
        (Token::Text(_), Token::Number(_)) => Ordering::Greater,
    }
}

fn primary(a: &[Vec<Token>], b: &[Vec<Token>]) -> Ordering {
    for (sa, sb) in a.iter().zip(b) {
        let order = sa
            .iter()
            .zip(sb)
            .map(|(x, y)| cmp_token(*x, *y))
            .find(|o| o.is_ne())
            .unwrap_or_else(|| sa.len().cmp(&sb.len()));
        if order.is_ne() {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

fn leading_zeros(a: &[Vec<Token>], b: &[Vec<Token>]) -> Ordering {
    for (sa, sb) in a.iter().zip(b) {
        for (x, y) in sa.iter().zip(sb) {
            if let (Token::Number(x), Token::Number(y)) = (x, y) {
                let order = x.len().cmp(&y.len());
                if order.is_ne() {
                    return order;
                }
            }
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::natural_cmp;
    use std::cmp::Ordering;

    fn sorted<'a>(input: &[&'a str]) -> Vec<&'a str> {
        let mut v = input.to_vec();
        v.sort_by(|a, b| natural_cmp(a, b));
        v
    }

    #[test]
    fn numbers_sort_by_value() {
        assert_eq!(sorted(&["page 10", "page 2", "page 1"]), ["page 1", "page 2", "page 10"]);
        assert_eq!(sorted(&["p010", "p9", "p1"]), ["p1", "p9", "p010"]);
    }

    #[test]
    fn folders_compare_one_level_at_a_time() {
        assert_eq!(sorted(&["Ch2/01", "ch10/01", "ch2/02"]), ["Ch2/01", "ch2/02", "ch10/01"]);
        assert_eq!(
            sorted(&["vol1 p2", "vol1 p10", "vol2 p1"]),
            ["vol1 p2", "vol1 p10", "vol2 p1"]
        );
    }

    #[test]
    fn text_is_case_insensitive() {
        assert_eq!(sorted(&["b", "A", "c"]), ["A", "b", "c"]);
    }

    #[test]
    fn number_before_text_at_same_position() {
        assert_eq!(sorted(&["a.jpg", "1.jpg"]), ["1.jpg", "a.jpg"]);
    }

    #[test]
    fn fewer_leading_zeros_first_then_bytes() {
        assert_eq!(sorted(&["01.png", "1.png", "001.png"]), ["1.png", "01.png", "001.png"]);
        assert_eq!(sorted(&["a.png", "A.png"]), ["A.png", "a.png"]);
    }

    #[test]
    fn punctuation_is_text() {
        assert_eq!(sorted(&["1.10", "1.5"]), ["1.5", "1.10"]);
        assert_eq!(sorted(&["-3", "-1"]), ["-1", "-3"]);
    }

    #[test]
    fn long_numbers_do_not_overflow() {
        assert_eq!(
            sorted(&["p100000000000000000000000", "p99999999999999999999999"]),
            ["p99999999999999999999999", "p100000000000000000000000"]
        );
    }

    #[test]
    fn unicode_names_sort_case_insensitively_and_deterministically() {
        assert_eq!(
            sorted(&["éclair.png", "Zebra.png", "apple.png"]),
            ["apple.png", "Zebra.png", "éclair.png"]
        );
        assert_eq!(sorted(&["élan.png", "Élan.png"]), ["Élan.png", "élan.png"]);
        assert_eq!(sorted(&["ページ10.png", "ページ2.png"]), ["ページ2.png", "ページ10.png"]);
    }

    #[test]
    fn order_is_total_and_antisymmetric() {
        let names = [
            "a", "A", "a1", "a01", "a/1", "a/01", "A/1", "1", "01", "", "ä", "a.png", "a 1",
        ];
        for x in names {
            assert_eq!(natural_cmp(x, x), Ordering::Equal, "{x}");
            for y in names {
                assert_eq!(natural_cmp(x, y), natural_cmp(y, x).reverse(), "{x} vs {y}");
                if x != y {
                    assert_ne!(natural_cmp(x, y), Ordering::Equal, "{x} vs {y}");
                }
            }
        }
    }
}
