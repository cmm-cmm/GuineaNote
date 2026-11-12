//! Literal text matching with optional case-insensitivity and whole-word
//! boundaries. Matches never split an extended grapheme cluster.
//!
//! Case-insensitive matching uses *simple* one-to-one folding: a scalar whose
//! lowercase form is a single scalar is compared by that form, otherwise it is
//! compared as is (for example `İ` or `ß` only match themselves). This keeps
//! every match a contiguous scalar range of the original text with the same
//! scalar count as the query, so ranges stay exact for selection and replace.
use std::ops::Range;
use unicode_segmentation::GraphemeCursor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchOptions {
    /// Compare scalars by simple lowercase folding.
    pub ignore_case: bool,
    /// Require non-word scalars (or text edges) on both sides of a match.
    pub whole_word: bool,
}

fn fold(c: char) -> char {
    if c.is_ascii() {
        return c.to_ascii_lowercase();
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(single), None) => single,
        _ => c,
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn grapheme_boundary(text: &str, byte: usize) -> bool {
    GraphemeCursor::new(byte, text.len(), true)
        .is_boundary(text, 0)
        .unwrap_or(false)
}

fn accepted(text: &str, range: &Range<usize>, options: SearchOptions) -> bool {
    if !grapheme_boundary(text, range.start) || !grapheme_boundary(text, range.end) {
        return false;
    }
    !options.whole_word
        || (text[..range.start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word(c))
            && text[range.end..].chars().next().is_none_or(|c| !is_word(c)))
}

/// Non-overlapping byte ranges of `needle` in `text`, in order.
pub fn find_matches<'a>(
    text: &'a str,
    needle: &'a str,
    options: SearchOptions,
) -> Box<dyn Iterator<Item = Range<usize>> + 'a> {
    if needle.is_empty() {
        return Box::new(std::iter::empty());
    }
    if !options.ignore_case {
        return Box::new(
            text.match_indices(needle)
                .map(|(byte, matched)| byte..byte + matched.len())
                .filter(move |range| accepted(text, range, options)),
        );
    }
    let folded: Vec<char> = needle.chars().map(fold).collect();
    let mut next = 0;
    Box::new(text.char_indices().filter_map(move |(byte, c)| {
        if byte < next || fold(c) != folded[0] {
            return None;
        }
        let mut end = byte;
        let mut chars = text[byte..].chars();
        for &want in &folded {
            let got = chars.next()?;
            if fold(got) != want {
                return None;
            }
            end += got.len_utf8();
        }
        let range = byte..end;
        if accepted(text, &range, options) {
            next = end;
            Some(range)
        } else {
            None
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all<'a>(text: &'a str, needle: &str, options: SearchOptions) -> Vec<&'a str> {
        find_matches(text, needle, options)
            .map(|r| &text[r])
            .collect()
    }

    #[test]
    fn default_options_keep_literal_case_sensitive_matching() {
        let text = "Việt việt VIỆT";
        assert_eq!(all(text, "Việt", SearchOptions::default()), ["Việt"]);
        assert!(all(text, "", SearchOptions::default()).is_empty());
    }

    #[test]
    fn ignore_case_folds_unicode_scalars_one_to_one() {
        let options = SearchOptions {
            ignore_case: true,
            whole_word: false,
        };
        let text = "Việt việt VIỆT Straße STRASSE İstanbul";
        assert_eq!(all(text, "việt", options), ["Việt", "việt", "VIỆT"]);
        // Multi-scalar folds are not applied: ranges keep the query length.
        assert_eq!(all(text, "straße", options), ["Straße"]);
        assert_eq!(all(text, "İstanbul", options), ["İstanbul"]);
        assert!(all(text, "istanbul", options).is_empty());
        assert_eq!(all("aaaa", "AA", options), ["aa", "aa"]);
    }

    #[test]
    fn matches_never_split_grapheme_clusters_with_any_option() {
        for ignore_case in [false, true] {
            let options = SearchOptions {
                ignore_case,
                whole_word: false,
            };
            // "e" inside "é" (e + combining acute) and inside a ZWJ emoji.
            assert!(all("e\u{301}", "e", options).is_empty());
            assert!(all("👩\u{200d}💻", "👩", options).is_empty());
            assert_eq!(
                all("e e\u{301}", "E", options).len(),
                usize::from(ignore_case)
            );
        }
    }

    #[test]
    fn whole_word_requires_non_word_neighbours() {
        let options = SearchOptions {
            ignore_case: true,
            whole_word: true,
        };
        let text = "cat concat cat_x (Cat) mèo cat";
        assert_eq!(all(text, "cat", options), ["cat", "Cat", "cat"]);
        assert_eq!(all("việt việtnam", "việt", options), ["việt"]);
        let sensitive = SearchOptions {
            ignore_case: false,
            whole_word: true,
        };
        assert_eq!(all(text, "cat", sensitive), ["cat", "cat"]);
    }
}
