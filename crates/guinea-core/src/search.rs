//! Text matching with optional case-insensitivity, whole-word boundaries and
//! regular expressions. Matches never split an extended grapheme cluster.
//!
//! Case-insensitive literal matching uses *simple* one-to-one folding: a
//! scalar whose lowercase form is a single scalar is compared by that form,
//! otherwise it is compared as is (for example `İ` or `ß` only match
//! themselves). This keeps every match a contiguous scalar range of the
//! original text with the same scalar count as the query, so ranges stay
//! exact for selection and replace.
//!
//! Regex mode uses the `regex` crate (linear time, no backtracking, no
//! look-around or backreferences) with `^`/`$` per line (CRLF aware). Empty
//! matches are skipped, and matches that would split a grapheme cluster or
//! (with whole word) touch a word character are dropped.
use std::{cell::RefCell, ops::Range};
use unicode_segmentation::GraphemeCursor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchOptions {
    /// Compare scalars by simple lowercase folding.
    pub ignore_case: bool,
    /// Require non-word scalars (or text edges) on both sides of a match.
    pub whole_word: bool,
    /// Treat the query as a regular expression (Notepad++ "Regular expression").
    pub regex: bool,
}

/// Compiled-program limits keep a hostile pattern from using unbounded memory.
const REGEX_SIZE_LIMIT: usize = 4 * 1024 * 1024;

thread_local! {
    /// The last compiled pattern per thread: rendering asks for the same Mark
    /// pattern once per visible line, so recompiling each time would be waste.
    static LAST: RefCell<Option<(String, bool, regex::Regex)>> = const { RefCell::new(None) };
}

/// Compile `pattern` for regex search, or describe why it is invalid.
pub fn compile(pattern: &str, options: SearchOptions) -> Result<regex::Regex, String> {
    LAST.with(|last| {
        if let Some((cached, case, regex)) = &*last.borrow()
            && cached == pattern
            && *case == options.ignore_case
        {
            return Ok(regex.clone());
        }
        let regex = regex::RegexBuilder::new(pattern)
            .case_insensitive(options.ignore_case)
            .multi_line(true)
            .crlf(true)
            .size_limit(REGEX_SIZE_LIMIT)
            .dfa_size_limit(REGEX_SIZE_LIMIT)
            .build()
            .map_err(|error| match error {
                regex::Error::CompiledTooBig(_) => "Regular expression is too large".to_string(),
                error => error
                    .to_string()
                    .lines()
                    .last()
                    .unwrap_or("Invalid regular expression")
                    .trim()
                    .to_string(),
            })?;
        *last.borrow_mut() = Some((pattern.to_string(), options.ignore_case, regex.clone()));
        Ok(regex)
    })
}

/// Why `query` cannot be searched with `options`, if it cannot.
pub fn validate(query: &str, options: SearchOptions) -> Result<(), String> {
    if options.regex && !query.is_empty() {
        compile(query, options)?;
    }
    Ok(())
}

/// The text that replaces the match at `range` (bytes of `text`): the
/// literal `replacement`, or in regex mode with `$1`, `${name}` and `$$`
/// expanded from that match's capture groups.
pub fn expand_replacement(
    text: &str,
    range: Range<usize>,
    query: &str,
    options: SearchOptions,
    replacement: &str,
) -> String {
    if !options.regex {
        return replacement.to_string();
    }
    let Ok(regex) = compile(query, options) else {
        return replacement.to_string();
    };
    let mut out = String::new();
    match regex.captures_at(text, range.start) {
        Some(captures) if captures.get(0).is_some_and(|m| m.range() == range) => {
            captures.expand(replacement, &mut out);
        }
        _ => out.push_str(replacement),
    }
    out
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

/// Non-overlapping byte ranges of `needle` in `text`, in order. An invalid
/// regex yields nothing; use [`validate`] to report it.
pub fn find_matches<'a>(
    text: &'a str,
    needle: &'a str,
    options: SearchOptions,
) -> Box<dyn Iterator<Item = Range<usize>> + 'a> {
    if needle.is_empty() {
        return Box::new(std::iter::empty());
    }
    if options.regex {
        let Ok(regex) = compile(needle, options) else {
            return Box::new(std::iter::empty());
        };
        // `find_at` keeps the text before `at` as context for `^` and `\b`.
        let mut at = 0;
        return Box::new(std::iter::from_fn(move || {
            loop {
                if at > text.len() {
                    return None;
                }
                let range = regex.find_at(text, at)?.range();
                at = if range.is_empty() {
                    // Step past an empty match by one scalar (or finish).
                    text[range.end..]
                        .chars()
                        .next()
                        .map_or(text.len() + 1, |c| range.end + c.len_utf8())
                } else {
                    range.end
                };
                if !range.is_empty() && accepted(text, &range, options) {
                    return Some(range);
                }
            }
        }));
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
            regex: false,
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
                regex: false,
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
            regex: false,
        };
        let text = "cat concat cat_x (Cat) mèo cat";
        assert_eq!(all(text, "cat", options), ["cat", "Cat", "cat"]);
        assert_eq!(all("việt việtnam", "việt", options), ["việt"]);
        let sensitive = SearchOptions {
            ignore_case: false,
            whole_word: true,
            regex: false,
        };
        assert_eq!(all(text, "cat", sensitive), ["cat", "cat"]);
    }

    fn regex(ignore_case: bool, whole_word: bool) -> SearchOptions {
        SearchOptions {
            ignore_case,
            whole_word,
            regex: true,
        }
    }

    #[test]
    fn regex_mode_finds_patterns_per_line_with_crlf_and_options() {
        assert_eq!(
            all("a1 b22 c333", r"\d+", regex(false, false)),
            ["1", "22", "333"]
        );
        assert_eq!(
            all("one\r\ntwo\r\nthree", r"^t\w+$", regex(false, false)),
            ["two", "three"],
            "^/$ are per line and $ stops before CR"
        );
        assert_eq!(
            all("Việt việt", r"việt", regex(true, false)),
            ["Việt", "việt"]
        );
        assert_eq!(
            all("cat concat cat_x", r"ca\w", regex(false, true)),
            ["cat"]
        );
    }

    #[test]
    fn regex_mode_skips_empty_matches_and_grapheme_splits() {
        assert_eq!(all("baab", r"a*", regex(false, false)), ["aa"]);
        assert!(all("", r"^", regex(false, false)).is_empty());
        // "." can match the base "e" of "e + acute": that would split it.
        assert_eq!(all("e\u{301}x", r".", regex(false, false)), ["x"]);
        assert_eq!(all("👩\u{200d}💻 ok", r"\w+", regex(false, false)), ["ok"]);
    }

    #[test]
    fn invalid_regex_is_reported_and_matches_nothing() {
        let error = validate("(unclosed", regex(false, false)).unwrap_err();
        assert!(!error.is_empty());
        assert!(all("(unclosed", "(unclosed", regex(false, false)).is_empty());
        assert!(
            validate("(unclosed", SearchOptions::default()).is_ok(),
            "literal mode"
        );
        assert!(validate(&"a{1000}".repeat(200), regex(false, false)).is_err());
    }

    #[test]
    fn regex_replacement_expands_capture_groups() {
        let text = "John Smith, Jane Doe";
        let first = find_matches(text, r"(\w+) (?<last>\w+)", regex(false, false))
            .next()
            .unwrap();
        assert_eq!(
            expand_replacement(
                text,
                first,
                r"(\w+) (?<last>\w+)",
                regex(false, false),
                "${last} $1 $$"
            ),
            "Smith John $"
        );
        assert_eq!(
            expand_replacement(text, 0..4, "John", SearchOptions::default(), "$1"),
            "$1",
            "literal mode never expands"
        );
    }
}
