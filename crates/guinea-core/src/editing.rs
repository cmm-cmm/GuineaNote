//! Editing commands built on `Document` primitives: word motion, block
//! indentation, line duplication/movement and whole-line clipboard ranges.
//! Every command is one Undo step, keeps grapheme boundaries and only touches
//! the bytes it is asked to change (newline sequences are preserved).
use crate::{Document, EditError, TextBuffer, graphemes};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Word motion inspects at most this many scalars around the caret, so a
/// multi-MiB line never turns Ctrl+Left/Right into a full-line scan.
const WORD_WINDOW: usize = 4096;

fn blank(segment: &str) -> bool {
    segment.chars().all(char::is_whitespace)
}

impl Document {
    fn line_of(&self, idx: usize) -> usize {
        self.buffer().char_to_line(idx).expect("valid index")
    }

    fn boundary_at_or_before(&self, idx: usize) -> usize {
        if self.is_grapheme_boundary(idx) {
            idx
        } else {
            graphemes::previous(self.buffer(), idx).unwrap_or(idx)
        }
    }

    fn boundary_at_or_after(&self, idx: usize) -> usize {
        if self.is_grapheme_boundary(idx) {
            idx
        } else {
            graphemes::next(self.buffer(), idx).unwrap_or(idx)
        }
    }

    /// End of the next word to the right (skipping whitespace first), or the
    /// start of the next line when the caret is at a line end.
    pub(crate) fn word_right(&self, idx: usize) -> usize {
        let line = self.line_of(idx);
        let bounds = self.line_bounds(line).expect("valid line");
        if idx >= bounds.end {
            return if line + 1 < self.buffer().len_lines() {
                self.buffer().line_to_char(line + 1).expect("valid line")
            } else {
                idx
            };
        }
        let end = bounds.end.min(idx.saturating_add(WORD_WINDOW));
        let text = self.buffer().slice(idx..end).expect("valid range");
        // Skip whitespace, then take one word (or punctuation) segment.
        let mut chars = 0;
        for segment in text.split_word_bounds() {
            chars += segment.chars().count();
            if !blank(segment) {
                break;
            }
        }
        self.boundary_at_or_after((idx + chars).min(bounds.end))
    }

    /// Start of the previous word to the left (skipping whitespace first), or
    /// the end of the previous line when the caret is at a line start.
    pub(crate) fn word_left(&self, idx: usize) -> usize {
        let line = self.line_of(idx);
        let bounds = self.line_bounds(line).expect("valid line");
        if idx <= bounds.start {
            return if line > 0 {
                self.line_bounds(line - 1).expect("valid line").end
            } else {
                idx
            };
        }
        let start = bounds.start.max(idx.saturating_sub(WORD_WINDOW));
        let text = self.buffer().slice(start..idx).expect("valid range");
        let mut chars = 0;
        for segment in text.split_word_bounds().rev() {
            chars += segment.chars().count();
            if !blank(segment) {
                break;
            }
        }
        self.boundary_at_or_before(idx - chars.min(idx - bounds.start))
    }

    /// Delete from the previous word start to the caret (or the selection).
    pub fn delete_word_left(&mut self) -> Result<(), EditError> {
        let range = if self.selection().is_empty() {
            self.word_left(self.cursor())..self.cursor()
        } else {
            self.selection().range()
        };
        self.replace_range(range, "")
    }

    /// Delete from the caret to the next word end (or the selection).
    pub fn delete_word_right(&mut self) -> Result<(), EditError> {
        let range = if self.selection().is_empty() {
            self.cursor()..self.word_right(self.cursor())
        } else {
            self.selection().range()
        };
        self.replace_range(range, "")
    }

    /// The line including its newline sequence, used by cut/copy when nothing
    /// is selected. The last line has no newline to include.
    pub fn line_range_with_newline(&self, line: usize) -> Range<usize> {
        let start = self.buffer().line_to_char(line).expect("valid line");
        let end = if line + 1 < self.buffer().len_lines() {
            self.buffer().line_to_char(line + 1).expect("valid line")
        } else {
            self.buffer().len_chars()
        };
        start..end
    }

    /// Text copied for the caret line when nothing is selected. A last line
    /// without a newline gains the document's newline so pasting stays a line.
    pub fn current_line_text(&self) -> String {
        let line = self.line_of(self.cursor());
        let range = self.line_range_with_newline(line);
        let mut text = self.buffer().slice(range).expect("valid line");
        if !text.ends_with('\n') && !text.ends_with('\r') {
            text.push_str(self.line_ending().newline());
        }
        text
    }

    /// Remove the caret line including its newline (Notepad++/VS Code cut
    /// without a selection). The last line also removes the preceding newline.
    pub fn cut_line(&mut self) -> Result<String, EditError> {
        let text = self.current_line_text();
        let line = self.line_of(self.cursor());
        let mut range = self.line_range_with_newline(line);
        if line + 1 >= self.buffer().len_lines() && line > 0 {
            range.start = self.line_bounds(line - 1)?.end;
        }
        self.replace_range(range, "")?;
        Ok(text)
    }

    /// Indentation unit used by Tab: `tab` spaces when space-indented lines
    /// outnumber tab-indented lines in the first 2,000 lines, otherwise `\t`.
    pub fn indent_unit(&self, tab: usize) -> String {
        let (mut tabs, mut spaces) = (0usize, 0usize);
        for line in 0..self.buffer().len_lines().min(2000) {
            let Ok(start) = self.buffer().line_to_char(line) else {
                break;
            };
            let end = (start + 1).min(self.buffer().len_chars());
            match self.buffer().slice(start..end).ok().as_deref() {
                Some("\t") => tabs += 1,
                Some(" ") => spaces += 1,
                _ => {}
            }
        }
        if spaces > tabs {
            " ".repeat(tab.clamp(1, 16))
        } else {
            "\t".into()
        }
    }

    /// Leading whitespace of the caret line, limited to text before the caret.
    pub fn caret_indentation(&self) -> String {
        let line = self.line_of(self.cursor());
        let start = self.buffer().line_to_char(line).expect("valid line");
        let end = self.cursor().min(start + 1024);
        self.buffer()
            .slice(start..end)
            .expect("valid range")
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }

    /// Lines touched by the selection; a selection ending at column 0 of a
    /// later line does not include that line.
    pub fn selected_lines(&self) -> Range<usize> {
        let range = self.selection().range();
        let first = self.line_of(range.start);
        let mut last = self.line_of(range.end);
        if last > first && self.buffer().line_to_char(last).ok() == Some(range.end) {
            last -= 1;
        }
        first..last + 1
    }

    /// Whether Tab should indent lines instead of replacing the selection.
    pub fn selection_spans_lines(&self) -> bool {
        let range = self.selection().range();
        !range.is_empty() && self.line_of(range.start) != self.line_of(range.end)
    }

    /// Indent (or with `outdent`, remove one level from) every selected line
    /// as one Undo step. Empty lines are not indented. The selection is
    /// extended to whole lines so repeated Tab/Shift+Tab keeps working.
    pub fn indent_lines(&mut self, unit: &str, tab: usize, outdent: bool) -> Result<(), EditError> {
        let lines = self.selected_lines();
        let reversed = self.selection().head < self.selection().anchor;
        let mut edits = Vec::new();
        let mut delta = 0isize;
        let mut first_delta = 0isize;
        for line in lines.clone() {
            let bounds = self.line_bounds(line)?;
            if outdent {
                let head = self
                    .buffer()
                    .slice(bounds.start..(bounds.start + tab.max(1)).min(bounds.end))?;
                let remove = if head.starts_with('\t') {
                    1
                } else {
                    head.chars().take_while(|c| *c == ' ').count()
                };
                if remove > 0 {
                    edits.push(crate::TextEdit {
                        start: bounds.start,
                        end: bounds.start + remove,
                        text: String::new(),
                    });
                    delta -= remove as isize;
                    if line == lines.start {
                        first_delta = -(remove as isize);
                    }
                }
            } else if !bounds.is_empty() || lines.len() == 1 {
                edits.push(crate::TextEdit {
                    start: bounds.start,
                    end: bounds.start,
                    text: unit.into(),
                });
                let count = unit.chars().count() as isize;
                delta += count;
                if line == lines.start {
                    first_delta = count;
                }
            }
        }
        let start = self.buffer().line_to_char(lines.start)?;
        let last_end = self.line_bounds(lines.end - 1)?.end;
        let was_empty = self.selection().is_empty();
        let caret = self.cursor();
        self.apply_edits(&edits)?;
        if was_empty {
            let moved = caret
                .saturating_add_signed(first_delta)
                .max(start)
                .min(self.buffer().len_chars());
            let moved = self.boundary_at_or_after(moved);
            return self.select(moved, moved);
        }
        let end = last_end.saturating_add_signed(delta);
        if reversed {
            self.select(end, start)
        } else {
            self.select(start, end)
        }
    }

    /// Duplicate the selection after itself, or the caret line below it.
    /// The caret moves onto the copy so repeated presses keep duplicating.
    pub fn duplicate(&mut self) -> Result<(), EditError> {
        let selection = self.selection().range();
        if !selection.is_empty() {
            let text = self.selected_text();
            let count = text.chars().count();
            self.select(selection.end, selection.end)?;
            self.insert(&text)?;
            return self.select(selection.end, selection.end + count);
        }
        let line = self.line_of(self.cursor());
        let bounds = self.line_bounds(line)?;
        let column = self.cursor() - bounds.start;
        let text = self.buffer().slice(bounds.clone())?;
        let separator = self.line_ending().newline();
        self.select(bounds.end, bounds.end)?;
        self.insert(&format!("{separator}{text}"))?;
        let target = bounds.end + separator.chars().count() + column;
        let target = self.boundary_at_or_after(target);
        self.select(target, target)
    }

    /// Move the selected lines (or the caret line) one line up or down as one
    /// Undo step, preserving every newline sequence and the selection.
    pub fn move_lines(&mut self, down: bool) -> Result<(), EditError> {
        let lines = self.selected_lines();
        let total = self.buffer().len_lines();
        if (!down && lines.start == 0) || (down && lines.end >= total) {
            return Ok(());
        }
        let block_start = self.buffer().line_to_char(lines.start)?;
        let block_end = self.line_bounds(lines.end - 1)?.end;
        let block = self.buffer().slice(block_start..block_end)?;
        let selection = self.selection();
        let (range, replacement, shift) = if down {
            let other = self.line_bounds(lines.end)?;
            let separator = self.buffer().slice(block_end..other.start)?;
            let neighbour = self.buffer().slice(other.clone())?;
            let shift = neighbour.chars().count() + separator.chars().count();
            (
                block_start..other.end,
                format!("{neighbour}{separator}{block}"),
                shift as isize,
            )
        } else {
            let other = self.line_bounds(lines.start - 1)?;
            let separator = self.buffer().slice(other.end..block_start)?;
            let neighbour = self.buffer().slice(other.clone())?;
            let shift = neighbour.chars().count() + separator.chars().count();
            (
                other.start..block_end,
                format!("{block}{separator}{neighbour}"),
                -(shift as isize),
            )
        };
        self.replace_range(range, &replacement)?;
        self.select(
            selection.anchor.saturating_add_signed(shift),
            selection.head.saturating_add_signed(shift),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{Document, Movement, TextBuffer};

    fn at(text: &str, cursor: usize) -> Document {
        let mut d = Document::from_text(text);
        d.select(cursor, cursor).unwrap();
        d
    }

    #[test]
    fn word_motion_skips_spaces_keeps_clusters_and_crosses_lines() {
        let text = "Việt  e\u{301}nam, 👩‍💻x\r\nnext";
        let mut d = at(text, 0);
        let mut stops = vec![];
        for _ in 0..8 {
            d.move_cursor(Movement::WordRight, false);
            stops.push(d.cursor());
        }
        let chars: Vec<char> = text.chars().collect();
        for &stop in &stops {
            assert!(d.is_grapheme_boundary(stop), "{stop}");
        }
        // "Việt" ends at 4; "e\u{301}nam" ends after the accent cluster.
        assert_eq!(stops[0], 4);
        assert_eq!(
            chars[..stops[1]].iter().collect::<String>(),
            "Việt  e\u{301}nam"
        );
        assert!(stops.contains(&d.line_bounds(0).unwrap().end));
        assert!(stops.contains(&d.buffer().line_to_char(1).unwrap()));
        let end = d.cursor();
        d.move_cursor(Movement::WordLeft, false);
        assert!(d.cursor() < end && d.is_grapheme_boundary(d.cursor()));
        let mut d = at(text, d.buffer().line_to_char(1).unwrap());
        d.move_cursor(Movement::WordLeft, false);
        assert_eq!(d.cursor(), d.line_bounds(0).unwrap().end, "crosses CRLF");
    }

    #[test]
    fn word_deletes_are_single_undo_steps() {
        let mut d = at("alpha beta  gamma", 17);
        d.delete_word_left().unwrap();
        assert_eq!(d.text(), "alpha beta  ");
        d.delete_word_left().unwrap();
        assert_eq!(d.text(), "alpha ");
        d.select(0, 0).unwrap();
        d.delete_word_right().unwrap();
        assert_eq!(d.text(), " ");
        d.undo().unwrap();
        d.undo().unwrap();
        d.undo().unwrap();
        assert_eq!(d.text(), "alpha beta  gamma");
    }

    #[test]
    fn typing_coalesces_until_caret_moves_or_another_edit() {
        let mut d = Document::default();
        for c in "Xin chào\nthế giới".chars() {
            d.type_text(&c.to_string()).unwrap();
        }
        assert_eq!(d.history_status().undo, 1, "a typed burst is one step");
        d.type_backspace().unwrap();
        d.type_backspace().unwrap();
        assert_eq!(d.history_status().undo, 2, "backspaces form their own step");
        d.move_cursor(Movement::Start, false);
        d.type_text("A").unwrap();
        d.type_text("B").unwrap();
        assert_eq!(d.text(), "ABXin chào\nthế gi");
        assert_eq!(d.history_status().undo, 3);
        d.undo().unwrap();
        assert_eq!(d.text(), "Xin chào\nthế gi");
        d.undo().unwrap();
        assert_eq!(d.text(), "Xin chào\nthế giới");
        d.undo().unwrap();
        assert_eq!(d.text(), "");
        assert!(!d.is_dirty());
        d.redo().unwrap();
        assert_eq!(d.text(), "Xin chào\nthế giới");
        // Delete-forward merges too; a save boundary starts a new step.
        let mut d = at("abcdef", 0);
        d.type_delete().unwrap();
        d.type_delete().unwrap();
        d.mark_saved();
        d.type_delete().unwrap();
        assert_eq!(d.text(), "def");
        d.undo().unwrap();
        assert_eq!(d.text(), "cdef");
        assert!(!d.is_dirty(), "undo back to the saved step is clean");
        d.undo().unwrap();
        assert_eq!(d.text(), "abcdef");
    }

    #[test]
    fn whole_line_cut_and_copy_keep_newline_sequences() {
        let mut d = at("one\r\ntwo\nthree", 6);
        assert_eq!(d.current_line_text(), "two\n");
        assert_eq!(d.cut_line().unwrap(), "two\n");
        assert_eq!(d.text(), "one\r\nthree");
        let last = d.buffer().len_chars();
        d.select(last, last).unwrap();
        assert_eq!(d.cut_line().unwrap(), "three\n");
        assert_eq!(d.text(), "one");
        d.undo().unwrap();
        d.undo().unwrap();
        assert_eq!(d.text(), "one\r\ntwo\nthree");
    }

    #[test]
    fn block_indent_detects_unit_and_outdent_reverses_it() {
        let mut d = Document::from_text("fn a() {\n    b();\n\n    c();\n}\n");
        assert_eq!(d.indent_unit(4), "    ");
        d.select(0, d.buffer().len_chars() - 1).unwrap();
        assert!(d.selection_spans_lines());
        d.indent_lines("    ", 4, false).unwrap();
        assert_eq!(
            d.text(),
            "    fn a() {\n        b();\n\n        c();\n    }\n"
        );
        assert_eq!(d.history_status().undo, 1);
        d.indent_lines("    ", 4, true).unwrap();
        assert_eq!(d.text(), "fn a() {\n    b();\n\n    c();\n}\n");
        let mut tabs = at("\tx\n\ty\nz", 3);
        assert_eq!(tabs.indent_unit(4), "\t");
        tabs.indent_lines("\t", 4, true).unwrap();
        assert_eq!(tabs.text(), "\tx\ny\nz");
        assert_eq!(tabs.cursor(), 3, "caret stays in the outdented line");
    }

    #[test]
    fn duplicate_and_move_lines_preserve_newlines_and_selection() {
        let mut d = at("a\r\nbé\nc", 4);
        d.duplicate().unwrap();
        // Mixed CRLF/LF files insert LF (the document newline policy).
        assert_eq!(d.text(), "a\r\nbé\nbé\nc");
        d.undo().unwrap();
        d.select(3, 5).unwrap();
        d.duplicate().unwrap();
        assert_eq!(d.text(), "a\r\nbébé\nc");
        d.undo().unwrap();
        let mut d = at("one\r\ntwo\nthree", 6);
        d.move_lines(false).unwrap();
        assert_eq!(d.text(), "two\r\none\nthree");
        assert_eq!(d.cursor(), 1);
        d.move_lines(false).unwrap();
        assert_eq!(d.text(), "two\r\none\nthree", "first line stays");
        d.move_lines(true).unwrap();
        assert_eq!(d.text(), "one\r\ntwo\nthree");
        assert_eq!(d.cursor(), 6);
        d.move_lines(true).unwrap();
        assert_eq!(d.text(), "one\r\nthree\ntwo");
        d.move_lines(true).unwrap();
        assert_eq!(d.text(), "one\r\nthree\ntwo", "last line stays");
        for _ in 0..3 {
            d.undo().unwrap();
        }
        assert_eq!(d.text(), "one\r\ntwo\nthree");
    }
}

/// Line and text transformations (Notepad++ Edit menu). Each is one Undo step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOperation {
    SortAscending,
    SortDescending,
    RemoveDuplicates,
    RemoveEmpty,
    TrimTrailing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseConversion {
    Upper,
    Lower,
    Title,
}

impl Document {
    /// Scalar range of the lines a line operation applies to: the selected
    /// lines, or the whole document without a multi-line selection. The range
    /// excludes the last line's newline so the document end is preserved.
    fn operation_lines(&self) -> Result<Range<usize>, EditError> {
        let lines = if self.selection_spans_lines() {
            self.selected_lines()
        } else {
            0..self.buffer().len_lines()
        };
        let start = self.buffer().line_to_char(lines.start)?;
        let end = self.line_bounds(lines.end - 1)?.end;
        Ok(start..end)
    }

    /// Apply a line operation; existing newline sequences are reused in order
    /// (a block of CRLF lines stays CRLF). Returns false when nothing changed.
    pub fn apply_line_operation(&mut self, operation: LineOperation) -> Result<bool, EditError> {
        let range = self.operation_lines()?;
        let text = self.buffer().slice(range.clone())?;
        let mut lines = Vec::new();
        let mut separators = Vec::new();
        let mut rest = text.as_str();
        while let Some(at) = rest.find('\n') {
            let (line, separator) = if rest[..at].ends_with('\r') {
                (&rest[..at - 1], "\r\n")
            } else {
                (&rest[..at], "\n")
            };
            lines.push(line.to_string());
            separators.push(separator);
            rest = &rest[at + 1..];
        }
        lines.push(rest.to_string());
        match operation {
            LineOperation::SortAscending => lines.sort(),
            LineOperation::SortDescending => lines.sort_by(|a, b| b.cmp(a)),
            LineOperation::RemoveDuplicates => {
                let mut seen = std::collections::HashSet::new();
                lines.retain(|line| seen.insert(line.clone()));
            }
            LineOperation::RemoveEmpty => lines.retain(|line| !line.trim().is_empty()),
            LineOperation::TrimTrailing => {
                for line in &mut lines {
                    line.truncate(line.trim_end_matches([' ', '\t']).len());
                }
            }
        }
        let mut output = String::with_capacity(text.len());
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                output.push_str(separators.get(index - 1).copied().unwrap_or("\n"));
            }
            output.push_str(line);
        }
        if output == text {
            return Ok(false);
        }
        self.replace_range(range.clone(), &output)?;
        let end = range.start + output.chars().count();
        self.select(range.start, end)?;
        Ok(true)
    }

    /// Convert the selection (or the caret line) to upper, lower or title case
    /// with Unicode case mapping; the result stays selected.
    pub fn convert_case(&mut self, conversion: CaseConversion) -> Result<bool, EditError> {
        let range = if self.selection().is_empty() {
            self.line_bounds(self.line_of(self.cursor()))?
        } else {
            self.selection().range()
        };
        let text = self.buffer().slice(range.clone())?;
        let output = match conversion {
            CaseConversion::Upper => text.to_uppercase(),
            CaseConversion::Lower => text.to_lowercase(),
            CaseConversion::Title => text
                .split_word_bounds()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) if first.is_alphabetic() => first
                            .to_uppercase()
                            .chain(chars.flat_map(char::to_lowercase))
                            .collect(),
                        _ => word.to_string(),
                    }
                })
                .collect(),
        };
        if output == text {
            return Ok(false);
        }
        self.replace_range(range.clone(), &output)?;
        self.select(range.start, range.start + output.chars().count())?;
        Ok(true)
    }

    /// Toggle a line comment on the selected lines (or the caret line): when
    /// every non-empty line already starts with `prefix` after its indentation
    /// the prefix (and one following space) is removed, otherwise `prefix ` is
    /// added after the indentation of each non-empty line.
    pub fn toggle_line_comment(&mut self, prefix: &str) -> Result<bool, EditError> {
        let lines = self.selected_lines();
        let mut targets = Vec::new();
        for line in lines.clone() {
            let bounds = self.line_bounds(line)?;
            let text = self.buffer().slice(bounds.clone())?;
            if text.trim().is_empty() {
                continue;
            }
            let indent = text.chars().take_while(|c| *c == ' ' || *c == '\t').count();
            let body: String = text.chars().skip(indent).collect();
            targets.push((bounds.start + indent, body));
        }
        if targets.is_empty() {
            return Ok(false);
        }
        let commented = targets.iter().all(|(_, body)| body.starts_with(prefix));
        let edits: Vec<crate::TextEdit> = targets
            .iter()
            .map(|(at, body)| {
                if commented {
                    let mut remove = prefix.chars().count();
                    if body[prefix.len()..].starts_with(' ') {
                        remove += 1;
                    }
                    crate::TextEdit {
                        start: *at,
                        end: at + remove,
                        text: String::new(),
                    }
                } else {
                    crate::TextEdit {
                        start: *at,
                        end: *at,
                        text: format!("{prefix} "),
                    }
                }
            })
            .collect();
        let start = self.buffer().line_to_char(lines.start)?;
        let had_selection = !self.selection().is_empty();
        let caret_line = self.line_of(self.cursor());
        let caret_column = self.cursor() - self.buffer().line_to_char(caret_line)?;
        self.apply_edits(&edits)?;
        if had_selection {
            let end = self.line_bounds(lines.end - 1)?.end;
            self.select(start, end)?;
        } else {
            let line_start = self.buffer().line_to_char(caret_line)?;
            let end = self.line_bounds(caret_line)?.end;
            let caret = self.boundary_at_or_before((line_start + caret_column).min(end));
            self.select(caret, caret)?;
        }
        Ok(true)
    }
}

#[cfg(test)]
mod line_operation_tests {
    use super::*;

    #[test]
    fn line_operations_keep_newlines_and_are_one_undo_step() {
        let mut d = Document::from_text("b\r\na\r\nb\r\n  \r\nc  \t\r\nend");
        assert!(
            d.apply_line_operation(LineOperation::RemoveDuplicates)
                .unwrap()
        );
        assert_eq!(d.text(), "b\r\na\r\n  \r\nc  \t\r\nend");
        assert!(d.apply_line_operation(LineOperation::RemoveEmpty).unwrap());
        assert_eq!(d.text(), "b\r\na\r\nc  \t\r\nend");
        assert!(d.apply_line_operation(LineOperation::TrimTrailing).unwrap());
        assert_eq!(d.text(), "b\r\na\r\nc\r\nend");
        assert!(
            d.apply_line_operation(LineOperation::SortAscending)
                .unwrap()
        );
        assert_eq!(d.text(), "a\r\nb\r\nc\r\nend");
        assert!(
            !d.apply_line_operation(LineOperation::SortAscending)
                .unwrap()
        );
        assert!(
            d.apply_line_operation(LineOperation::SortDescending)
                .unwrap()
        );
        assert_eq!(d.text(), "end\r\nc\r\nb\r\na");
        for _ in 0..5 {
            d.undo().unwrap();
        }
        assert_eq!(d.text(), "b\r\na\r\nb\r\n  \r\nc  \t\r\nend");
        // A multi-line selection limits the operation to its lines.
        let mut d = Document::from_text("z\ny\nx\nw\n");
        d.select(2, 6).unwrap();
        d.apply_line_operation(LineOperation::SortAscending)
            .unwrap();
        assert_eq!(d.text(), "z\nx\ny\nw\n");
    }

    #[test]
    fn case_conversion_is_unicode_aware_and_keeps_selection() {
        let mut d = Document::from_text("xin chào VIỆT nam");
        d.select(4, 17).unwrap();
        d.convert_case(CaseConversion::Upper).unwrap();
        assert_eq!(d.text(), "xin CHÀO VIỆT NAM");
        d.convert_case(CaseConversion::Title).unwrap();
        assert_eq!(d.text(), "xin Chào Việt Nam");
        assert_eq!(d.selected_text(), "Chào Việt Nam");
        d.select(0, 0).unwrap();
        d.convert_case(CaseConversion::Lower).unwrap();
        assert_eq!(d.text(), "xin chào việt nam");
    }

    #[test]
    fn toggle_line_comment_round_trips_and_skips_blank_lines() {
        let mut d = Document::from_text("fn a() {\n    b();\n\n    c();\n}");
        d.select(0, d.buffer().len_chars()).unwrap();
        d.toggle_line_comment("//").unwrap();
        assert_eq!(d.text(), "// fn a() {\n    // b();\n\n    // c();\n// }");
        assert_eq!(d.history_status().undo, 1);
        d.toggle_line_comment("//").unwrap();
        assert_eq!(d.text(), "fn a() {\n    b();\n\n    c();\n}");
        let mut d = Document::from_text("x = 1\ny = 2");
        d.select(2, 2).unwrap();
        d.toggle_line_comment("#").unwrap();
        assert_eq!(d.text(), "# x = 1\ny = 2");
        assert_eq!(d.cursor(), 2);
    }
}
