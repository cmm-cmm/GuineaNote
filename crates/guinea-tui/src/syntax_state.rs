//! Per-tab multi-line lexer state (block comments, triple-quoted strings,
//! template literals, markup comments) for the source view.
//!
//! `states[i]` is the lexer state at the start of line `i` for the cached
//! document revision. After an edit only lines from the earliest changed line
//! are recomputed. Each frame scans at most [`FRAME_BUDGET`] scalars; lines
//! whose state is not known yet render without highlighting (never with a
//! guessed, possibly wrong context) and the next frames continue the scan.
use guinea_core::{Document, TextBuffer};
use guinea_syntax::{BasicSyntax, Language, LineState};

/// Scalars scanned per frame for state propagation.
const FRAME_BUDGET: usize = 512 * 1024;
/// Longer lines are not scanned for state; their end state is assumed Normal.
const LONG_LINE: usize = 64 * 1024;

#[derive(Default)]
pub struct SyntaxStates {
    document: u64,
    revision: u64,
    language: Option<Language>,
    states: Vec<LineState>,
}

impl SyntaxStates {
    /// Bring states up to date through `last`. Returns false when the frame
    /// budget ran out first (call again on the next frame).
    pub fn prepare(&mut self, document: &Document, language: Language, last: usize) -> bool {
        if !LineState::tracked(language) {
            self.states.clear();
            self.language = Some(language);
            return true;
        }
        if self.document != document.id || self.language != Some(language) {
            self.states.clear();
        } else if self.revision != document.revision() {
            match document.earliest_change_since(self.revision) {
                Some(usize::MAX) => {}
                Some(change) => {
                    let change = change.min(document.buffer().len_chars());
                    let line = document.buffer().char_to_line(change).unwrap_or(0);
                    self.states.truncate(line + 1);
                }
                None => self.states.clear(),
            }
        }
        self.document = document.id;
        self.revision = document.revision();
        self.language = Some(language);
        if self.states.is_empty() {
            self.states.push(LineState::Normal);
        }
        let target = last.min(document.buffer().len_lines().saturating_sub(1));
        let mut budget = FRAME_BUDGET;
        while self.states.len() <= target {
            if budget == 0 {
                return false;
            }
            let line = self.states.len() - 1;
            let state = self.states[line];
            let Ok(range) = document.line_bounds(line) else {
                break;
            };
            budget = budget.saturating_sub(range.len().max(1));
            let next = if range.len() > LONG_LINE {
                LineState::Normal
            } else {
                document
                    .buffer()
                    .slice(range)
                    .map(|text| BasicSyntax.end_state(language, &text, state))
                    .unwrap_or_default()
            };
            self.states.push(next);
        }
        true
    }

    /// State at the start of `line`, if computed for the current revision.
    pub fn state(&self, document: &Document, language: Language, line: usize) -> Option<LineState> {
        if !LineState::tracked(language) {
            return Some(LineState::Normal);
        }
        if self.document != document.id || self.revision != document.revision() {
            return None;
        }
        self.states.get(line).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_follow_edits_incrementally_and_reset_on_unknown_history() {
        let mut d = Document::from_text("a /* x\nb\nc */ d\ne\n");
        let mut s = SyntaxStates::default();
        assert!(s.prepare(&d, Language::Rust, 10));
        assert_eq!(
            s.state(&d, Language::Rust, 1),
            Some(LineState::BlockComment)
        );
        assert_eq!(s.state(&d, Language::Rust, 3), Some(LineState::Normal));
        // Closing the comment earlier changes later lines only.
        d.select(8, 8).unwrap();
        d.insert("*/").unwrap();
        assert_eq!(s.state(&d, Language::Rust, 2), None, "stale until prepared");
        assert!(s.prepare(&d, Language::Rust, 10));
        assert_eq!(
            s.state(&d, Language::Rust, 1),
            Some(LineState::BlockComment)
        );
        assert_eq!(s.state(&d, Language::Rust, 2), Some(LineState::Normal));
        d.undo().unwrap();
        assert!(s.prepare(&d, Language::Rust, 10));
        assert_eq!(
            s.state(&d, Language::Rust, 2),
            Some(LineState::BlockComment)
        );
        assert_eq!(s.state(&d, Language::Toml, 2), Some(LineState::Normal));
    }

    #[test]
    fn frame_budget_defers_far_lines_instead_of_guessing() {
        let line = "x".repeat(60 * 1024);
        let text = format!("/*\n{}", format!("{line}\n").repeat(20));
        let d = Document::from_text(&text);
        let mut s = SyntaxStates::default();
        assert!(!s.prepare(&d, Language::C, 20));
        assert_eq!(s.state(&d, Language::C, 1), Some(LineState::BlockComment));
        assert_eq!(s.state(&d, Language::C, 20), None);
        while !s.prepare(&d, Language::C, 20) {}
        assert_eq!(s.state(&d, Language::C, 20), Some(LineState::BlockComment));
    }
}
