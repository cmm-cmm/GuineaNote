use std::{
    ops::Range,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use crate::{EditError, RopeyBuffer, TextBuffer, coordinates::CharIndex};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Mixed,
}

impl LineEnding {
    pub fn detect(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let lf = text.bytes().filter(|&b| b == b'\n').count() - crlf;
        let bare_cr = text.bytes().filter(|&b| b == b'\r').count() - crlf;
        match (crlf > 0, lf > 0, bare_cr > 0) {
            (_, _, true) | (true, true, _) => Self::Mixed,
            (true, false, false) => Self::CrLf,
            _ => Self::Lf,
        }
    }
    pub fn newline(self) -> &'static str {
        match self {
            Self::CrLf => "\r\n",
            _ => "\n",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
            Self::Mixed => "Mixed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Movement {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Start,
    Finish,
}

#[derive(Debug, Clone)]
struct Snapshot {
    buffer: RopeyBuffer,
    selection: Selection,
    content_state: u64,
}

#[derive(Debug, Clone)]
struct Transaction {
    before: Snapshot,
    after: Snapshot,
    start: usize,
    removed: usize,
    inserted: usize,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: u64,
    pub path: Option<PathBuf>,
    pub readonly: bool,
    pub bom: bool,
    pub fingerprint: Option<Fingerprint>,
    buffer: RopeyBuffer,
    content_state: u64,
    saved_state: u64,
    next_content_state: u64,
    saved_bom: bool,
    selection: Selection,
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    revision: u64,
    preferred_column: Option<usize>,
    newline_policy: LineEnding,
    pub(crate) layout_cache: std::cell::RefCell<Option<std::sync::Arc<crate::view::LineLayout>>>,
}

impl Default for Document {
    fn default() -> Self {
        Self::from_text("")
    }
}

impl Document {
    pub fn from_text(text: &str) -> Self {
        let buffer = RopeyBuffer::from(text);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            path: None,
            readonly: false,
            bom: false,
            fingerprint: None,
            content_state: 0,
            saved_state: 0,
            next_content_state: 1,
            saved_bom: false,
            buffer,
            selection: Selection::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            preferred_column: None,
            newline_policy: LineEnding::detect(text),
            layout_cache: std::cell::RefCell::new(None),
        }
    }

    pub fn buffer(&self) -> &impl TextBuffer {
        &self.buffer
    }
    pub fn text(&self) -> String {
        self.buffer.text()
    }
    /// Cheap immutable content snapshot for background rendering.
    pub fn snapshot_buffer(&self) -> RopeyBuffer {
        self.buffer.clone()
    }
    /// Read-only view of this document, sharing text/layout without copying or
    /// retaining editable history. Its id links it back to the source tab.
    pub fn review_snapshot(&self) -> Self {
        Self {
            id: self.id,
            path: self.path.clone(),
            readonly: true,
            bom: self.bom,
            fingerprint: self.fingerprint.clone(),
            buffer: self.buffer.clone(),
            content_state: self.content_state,
            saved_state: self.saved_state,
            next_content_state: self.next_content_state,
            saved_bom: self.saved_bom,
            selection: Selection::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            revision: self.revision,
            preferred_column: None,
            newline_policy: self.newline_policy,
            layout_cache: self.layout_cache.clone(),
        }
    }
    /// Reopened content keeps the tab identity but advances its view revision,
    /// so cached previews cannot mistake a reload for the old saved buffer.
    pub fn continue_identity(&mut self, previous: &Self) {
        self.id = previous.id;
        self.revision = previous.revision + 1;
        *self.layout_cache.borrow_mut() = None;
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub fn cursor(&self) -> usize {
        self.selection.head
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn is_dirty(&self) -> bool {
        self.bom != self.saved_bom || self.content_state != self.saved_state
    }
    pub fn line_ending(&self) -> LineEnding {
        self.newline_policy
    }

    pub fn mark_saved(&mut self) {
        self.saved_state = self.content_state;
        self.saved_bom = self.bom;
    }

    /// Merge only metadata from the snapshot committed by a background save.
    /// Edits and selection made while I/O was running remain in this document.
    pub fn accept_saved_snapshot(&mut self, saved: &Self) {
        assert_eq!(self.id, saved.id);
        self.path = saved.path.clone();
        self.fingerprint = saved.fingerprint.clone();
        self.saved_state = saved.saved_state;
        self.saved_bom = saved.saved_bom;
    }

    pub fn selected_text(&self) -> String {
        self.buffer
            .slice(self.selection.range())
            .expect("valid selection")
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            buffer: self.buffer.clone(),
            selection: self.selection,
            content_state: self.content_state,
        }
    }

    fn restore(&mut self, snapshot: &Snapshot) {
        self.buffer = snapshot.buffer.clone();
        self.selection = snapshot.selection;
        self.content_state = snapshot.content_state;
        self.revision += 1;
        self.preferred_column = None;
    }

    pub fn is_grapheme_boundary(&self, idx: usize) -> bool {
        crate::graphemes::is_boundary(&self.buffer, idx)
    }

    fn snap_forward(&self, idx: usize) -> usize {
        if self.is_grapheme_boundary(idx) {
            idx
        } else {
            crate::graphemes::next(&self.buffer, idx).expect("valid index")
        }
    }

    pub fn select(&mut self, anchor: usize, head: usize) -> Result<(), EditError> {
        if !self.is_grapheme_boundary(anchor) || !self.is_grapheme_boundary(head) {
            return Err(EditError::GraphemeBoundary);
        }
        self.selection = Selection { anchor, head };
        self.preferred_column = None;
        Ok(())
    }

    pub fn select_all(&mut self) {
        self.selection = Selection {
            anchor: 0,
            head: self.buffer.len_chars(),
        };
        self.preferred_column = None;
    }

    pub fn replace_range(&mut self, range: Range<usize>, text: &str) -> Result<(), EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        if range.start > range.end || range.end > self.buffer.len_chars() {
            return Err(EditError::OutOfBounds);
        }
        if !self.is_grapheme_boundary(range.start) || !self.is_grapheme_boundary(range.end) {
            return Err(EditError::GraphemeBoundary);
        }
        let before = self.snapshot();
        let layout_edit = self.prepare_layout_edit(&range, text);
        let changed = self.buffer.slice(range.clone())? != text;
        self.buffer.delete(range.clone())?;
        self.buffer.insert(range.start, text)?;
        let cursor = self.snap_forward(range.start + text.chars().count());
        self.selection = Selection {
            anchor: cursor,
            head: cursor,
        };
        self.preferred_column = None;
        if changed {
            self.content_state = self.next_content_state;
            self.next_content_state += 1;
            self.revision += 1;
            self.apply_layout_edit(layout_edit, text);
            self.redo.clear();
            self.undo.push(Transaction {
                before,
                after: self.snapshot(),
                start: range.start,
                removed: range.len(),
                inserted: text.chars().count(),
            });
        }
        Ok(())
    }

    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        self.replace_range(self.selection.range(), text)
    }

    /// Validate every scalar/grapheme range before changing anything. All edits
    /// refer to the original snapshot; one call creates one undo transaction.
    pub fn apply_edits(&mut self, edits: &[TextEdit]) -> Result<(), EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        let mut ordered = edits.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|e| (e.start, e.end));
        for (index, edit) in ordered.iter().enumerate() {
            if edit.start > edit.end || edit.end > self.buffer.len_chars() {
                return Err(EditError::OutOfBounds);
            }
            if !self.is_grapheme_boundary(edit.start) || !self.is_grapheme_boundary(edit.end) {
                return Err(EditError::GraphemeBoundary);
            }
            if index > 0
                && (ordered[index - 1].end > edit.start || ordered[index - 1].start == edit.start)
            {
                return Err(EditError::OutOfBounds);
            }
        }
        if edits.is_empty() {
            return Ok(());
        }
        if edits.iter().all(|e| {
            self.buffer
                .slice(e.start..e.end)
                .is_ok_and(|text| text == e.text)
        }) {
            return Ok(());
        }
        let mut prepared = self.review_snapshot();
        for edit in ordered.into_iter().rev() {
            prepared.buffer.delete(edit.start..edit.end)?;
            prepared.buffer.insert(edit.start, &edit.text)?;
        }
        prepared.selection = Selection::default();
        self.accept_prepared(&prepared, self.revision)
    }

    /// Commit an off-thread prepared snapshot in O(1) shared-buffer work while
    /// retaining this document's history, identity, path and save metadata.
    pub fn accept_prepared(
        &mut self,
        prepared: &Self,
        expected_revision: u64,
    ) -> Result<(), EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        if self.revision != expected_revision || self.id != prepared.id {
            return Err(EditError::RevisionConflict);
        }
        let before = self.snapshot();
        let removed = self.buffer.len_chars();
        self.buffer = prepared.buffer.clone();
        self.selection = prepared.selection;
        self.content_state = self.next_content_state;
        self.next_content_state += 1;
        self.revision += 1;
        self.preferred_column = None;
        *self.layout_cache.borrow_mut() = None;
        self.redo.clear();
        self.undo.push(Transaction {
            before,
            after: self.snapshot(),
            start: 0,
            removed,
            inserted: self.buffer.len_chars(),
        });
        Ok(())
    }

    pub fn newline(&mut self) -> Result<(), EditError> {
        self.insert(self.newline_policy.newline())
    }

    fn previous(&self, idx: usize) -> usize {
        crate::graphemes::previous(&self.buffer, idx).expect("valid cursor")
    }

    fn next(&self, idx: usize) -> usize {
        crate::graphemes::next(&self.buffer, idx).expect("valid cursor")
    }

    pub fn backspace(&mut self) -> Result<(), EditError> {
        let range = if self.selection.is_empty() {
            self.previous(self.cursor())..self.cursor()
        } else {
            self.selection.range()
        };
        self.replace_range(range, "")
    }

    pub fn delete_forward(&mut self) -> Result<(), EditError> {
        let range = if self.selection.is_empty() {
            self.cursor()..self.next(self.cursor())
        } else {
            self.selection.range()
        };
        self.replace_range(range, "")
    }

    /// Zero-based line and grapheme column, independent of terminal cell width.
    pub fn position(&self) -> (usize, usize) {
        let line = self
            .buffer
            .char_to_line(self.cursor())
            .expect("valid cursor");
        let start = self.buffer.line_to_char(line).expect("valid line");
        let tab_width = self
            .layout_cache
            .borrow()
            .as_ref()
            .map_or(4, |cache| cache.tab_width());
        if let Some(layout) = self.indexed_line(line, tab_width, self.cursor() - start, 0) {
            return (line, layout.position(self.cursor()).0);
        }
        let bytes = self
            .buffer
            .char_to_byte(CharIndex(self.cursor()))
            .unwrap()
            .0
            - self.buffer.char_to_byte(CharIndex(start)).unwrap().0;
        let column = if bytes == self.cursor() - start {
            bytes
        } else {
            self.buffer
                .slice(start..self.cursor())
                .unwrap()
                .graphemes(true)
                .count()
        };
        (line, column)
    }

    pub fn move_cursor(&mut self, movement: Movement, extend: bool) {
        let line = self
            .buffer
            .char_to_line(self.cursor())
            .expect("valid cursor");
        let vertical = matches!(movement, Movement::Up | Movement::Down);
        let idx = match movement {
            Movement::Left if !extend && !self.selection.is_empty() => self.selection.range().start,
            Movement::Right if !extend && !self.selection.is_empty() => self.selection.range().end,
            Movement::Left => self.previous(self.cursor()),
            Movement::Right => self.next(self.cursor()),
            Movement::Start => 0,
            Movement::Finish => self.buffer.len_chars(),
            Movement::Home => self.buffer.line_to_char(line).expect("valid line"),
            Movement::End => self.line_end(line),
            Movement::Up | Movement::Down => {
                let goal = self.preferred_column.unwrap_or_else(|| self.position().1);
                self.preferred_column = Some(goal);
                let target = if matches!(movement, Movement::Up) {
                    line.saturating_sub(1)
                } else {
                    (line + 1).min(self.buffer.len_lines() - 1)
                };
                let start = self.buffer.line_to_char(target).expect("valid line");
                let end = self.line_end(target);
                let bytes = self.buffer.char_to_byte(CharIndex(end)).unwrap().0
                    - self.buffer.char_to_byte(CharIndex(start)).unwrap().0;
                if bytes == end - start {
                    start + goal.min(end - start)
                } else {
                    start
                        + self
                            .buffer
                            .slice(start..end)
                            .unwrap()
                            .graphemes(true)
                            .take(goal)
                            .map(|g| g.chars().count())
                            .sum::<usize>()
                }
            }
        };
        self.selection.head = idx;
        if !extend {
            self.selection.anchor = idx;
        }
        if !vertical {
            self.preferred_column = None;
        }
    }

    fn line_end(&self, line: usize) -> usize {
        self.line_bounds(line).expect("valid line").end
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        if let Some(tx) = self.undo.pop() {
            let replacement = self.layout_cache.borrow().as_ref().map(|_| {
                tx.before
                    .buffer
                    .slice(tx.start..tx.start + tx.removed)
                    .expect("history range")
            });
            let layout_edit = replacement.as_ref().and_then(|text| {
                self.prepare_layout_edit(&(tx.start..tx.start + tx.inserted), text)
            });
            self.restore(&tx.before);
            self.apply_layout_edit(layout_edit, replacement.as_deref().unwrap_or(""));
            self.redo.push(tx);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        if let Some(tx) = self.redo.pop() {
            let replacement = self.layout_cache.borrow().as_ref().map(|_| {
                tx.after
                    .buffer
                    .slice(tx.start..tx.start + tx.inserted)
                    .expect("history range")
            });
            let layout_edit = replacement.as_ref().and_then(|text| {
                self.prepare_layout_edit(&(tx.start..tx.start + tx.removed), text)
            });
            self.restore(&tx.after);
            self.apply_layout_edit(layout_edit, replacement.as_deref().unwrap_or(""));
            self.undo.push(tx);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Literal search wraps; matches splitting a grapheme are skipped.
    pub fn find(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }
        let text = self.text();
        let cursor_byte = crate::coordinates::char_to_byte(
            &text,
            crate::coordinates::CharIndex(self.selection.range().end),
        )
        .expect("valid selection")
        .0;
        let mut first = None;
        let mut chosen = None;
        for (byte, matched) in text.match_indices(needle) {
            let end = byte + matched.len();
            if byte_is_grapheme_boundary(&text, byte) && byte_is_grapheme_boundary(&text, end) {
                first.get_or_insert((byte, end));
                if byte >= cursor_byte {
                    chosen = Some((byte, end));
                    break;
                }
            }
        }
        if let Some((byte, end_byte)) = chosen.or(first) {
            let start = text[..byte].chars().count();
            let end = start + text[byte..end_byte].chars().count();
            self.select(start, end).expect("checked boundaries");
            true
        } else {
            false
        }
    }

    pub fn replace_all(&mut self, needle: &str, replacement: &str) -> Result<usize, EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        if needle.is_empty() {
            return Ok(0);
        }
        let text = self.text();
        let mut output = String::new();
        let mut previous_byte = 0;
        let mut count = 0;
        for (byte, matched) in text.match_indices(needle) {
            let end = byte + matched.len();
            if byte_is_grapheme_boundary(&text, byte) && byte_is_grapheme_boundary(&text, end) {
                output.push_str(&text[previous_byte..byte]);
                output.push_str(replacement);
                previous_byte = byte + matched.len();
                count += 1;
            }
        }
        if count > 0 {
            output.push_str(&text[previous_byte..]);
            self.replace_range(0..self.buffer.len_chars(), &output)?;
        }
        Ok(count)
    }
}

fn byte_is_grapheme_boundary(text: &str, byte: usize) -> bool {
    GraphemeCursor::new(byte, text.len(), true)
        .is_boundary(text, 0)
        .expect("complete UTF-8 snapshot provides all grapheme context")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_edits_are_atomic_one_undo_and_revision_checked() {
        let original = "a\u{301}👩‍💻\r\nxyz";
        let mut d = Document::from_text(original);
        let revision = d.revision();
        assert!(
            d.apply_edits(&[TextEdit {
                start: 2,
                end: 4,
                text: "bad".into()
            }])
            .is_err()
        );
        assert_eq!(d.text(), original);
        assert_eq!(d.revision(), revision);
        assert!(!d.is_dirty());
        d.apply_edits(&[TextEdit {
            start: 0,
            end: 2,
            text: "a\u{301}".into(),
        }])
        .unwrap();
        assert_eq!(d.revision(), revision);
        assert!(!d.is_dirty());
        assert!(!d.undo().unwrap());
        d.apply_edits(&[
            TextEdit {
                start: 2,
                end: 5,
                text: "B".into(),
            },
            TextEdit {
                start: 7,
                end: 10,
                text: "tail".into(),
            },
        ])
        .unwrap();
        assert_eq!(d.text(), "a\u{301}B\r\ntail");
        assert!(d.undo().unwrap());
        assert_eq!(d.text(), original);
        assert!(!d.is_dirty());
        assert!(d.redo().unwrap());
        assert_eq!(d.text(), "a\u{301}B\r\ntail");
        let prepared = d.review_snapshot();
        let old = d.revision();
        d.insert("user").unwrap();
        assert_eq!(
            d.accept_prepared(&prepared, old),
            Err(EditError::RevisionConflict)
        );
    }
    use proptest::prelude::*;
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn unicode_navigation_and_deletion() {
        let mut d = Document::from_text("a\u{301}👩‍💻🇻🇳\r\n界");
        let expected = [2, 5, 7, 9, 10];
        for idx in expected {
            d.move_cursor(Movement::Right, false);
            assert_eq!(d.cursor(), idx);
        }
        d.backspace().unwrap();
        d.backspace().unwrap();
        assert_eq!(d.text(), "a\u{301}👩‍💻🇻🇳");
        assert!(d.select(1, 2).is_err());
    }

    #[test]
    fn join_combining_sequence_snaps_cursor() {
        let mut d = Document::from_text("aX\u{301}");
        d.select(1, 3).unwrap();
        d.insert("\u{301}").unwrap();
        assert_eq!(d.text(), "a\u{301}");
        assert_eq!(d.cursor(), 2);
        d.undo().unwrap();
        assert_eq!(d.selection(), Selection { anchor: 1, head: 3 });
    }

    #[test]
    fn savepoint_and_history_branch() {
        let mut d = Document::default();
        d.insert("Việt").unwrap();
        d.mark_saved();
        d.insert(" 🐹").unwrap();
        assert!(d.is_dirty());
        d.undo().unwrap();
        assert!(!d.is_dirty());
        d.undo().unwrap();
        assert!(d.is_dirty());
        d.redo().unwrap();
        assert!(!d.is_dirty());
        d.undo().unwrap();
        d.insert("branch").unwrap();
        assert!(!d.redo().unwrap());
    }

    #[test]
    fn chunk_edges_match_extended_graphemes() {
        for sequence in [
            "e\u{301}",
            "👩‍💻",
            "🇻🇳",
            "\r\n",
            "\u{600}a",
            "a\u{301}\u{302}",
        ] {
            for prefix in [990, 1000, 1010, 1020, 2030] {
                let text = format!("{}{}tail", "x".repeat(prefix), sequence.repeat(12));
                let mut d = Document::from_text(&text);
                let mut chars = prefix;
                d.select(prefix, prefix).unwrap();
                for g in text[prefix..].graphemes(true) {
                    chars += g.chars().count();
                    d.move_cursor(Movement::Right, false);
                    assert_eq!(d.cursor(), chars, "{sequence:?} at {prefix}");
                    assert!(d.is_grapheme_boundary(chars));
                }
                for g in text[prefix..].graphemes(true).rev() {
                    chars -= g.chars().count();
                    d.move_cursor(Movement::Left, false);
                    assert_eq!(d.cursor(), chars);
                }
            }
        }
    }

    #[test]
    fn save_snapshot_preserves_later_edits_selection_and_history() {
        let mut d = Document::from_text("original");
        d.insert("first ").unwrap();
        let mut saved = d.clone();
        saved.mark_saved();
        saved.path = Some("saved.txt".into());
        d.insert("later ").unwrap();
        let selection = d.selection();
        d.accept_saved_snapshot(&saved);
        assert_eq!(d.text(), "first later original");
        assert_eq!(d.selection(), selection);
        assert!(d.is_dirty());
        d.undo().unwrap();
        assert!(!d.is_dirty());
        d.undo().unwrap();
        d.insert("first ").unwrap();
        // A new history branch gets a distinct state even with identical bytes.
        assert_eq!(d.text(), saved.text());
        assert!(d.is_dirty());
    }

    #[test]
    fn selection_search_replace_is_one_transaction() {
        let mut d = Document::from_text("e\u{301} Việt Việt\r\n");
        assert!(!d.find("\u{301}"));
        assert!(d.find("Việt"));
        let selection = d.selection();
        assert_eq!(d.replace_all("Việt", "🐹").unwrap(), 2);
        assert_eq!(d.text(), "e\u{301} 🐹 🐹\r\n");
        d.undo().unwrap();
        assert_eq!(d.selection(), selection);
        assert!(!d.is_dirty());
    }

    #[test]
    fn bare_cr_is_visible_content_and_end_removes_only_one_line_ending() {
        let mut d = Document::from_text("a\r\r\nb\r");
        d.move_cursor(Movement::End, false);
        assert_eq!(d.cursor(), 2);
        d.move_cursor(Movement::Down, false);
        assert_eq!(d.cursor(), 6);
        d.move_cursor(Movement::End, false);
        assert_eq!(d.cursor(), 6);
        assert_eq!(d.text(), "a\r\r\nb\r");
    }

    #[test]
    fn newline_policy_and_readonly() {
        let mut d = Document::from_text("a\r\nb\r\n");
        d.newline().unwrap();
        assert!(d.text().starts_with("\r\n"));
        let mut mixed = Document::from_text("a\r\nb\n");
        mixed.newline().unwrap();
        assert!(mixed.text().starts_with("\na"));
        mixed.readonly = true;
        assert_eq!(mixed.insert("x"), Err(EditError::ReadOnly));
        assert_eq!(mixed.undo(), Err(EditError::ReadOnly));
    }

    proptest! {
        #[test]
        fn navigation_matches_unicode_segmentation(text in ".{0,120}") {
            let mut d = Document::from_text(&text);
            let mut boundaries = vec![0];
            let mut chars = 0;
            for grapheme in text.graphemes(true) {
                chars += grapheme.chars().count();
                boundaries.push(chars);
            }
            for &expected in boundaries.iter().skip(1) {
                d.move_cursor(Movement::Right, false);
                prop_assert_eq!(d.cursor(), expected);
                prop_assert!(d.is_grapheme_boundary(expected));
            }
            for &expected in boundaries.iter().rev().skip(1) {
                d.move_cursor(Movement::Left, false);
                prop_assert_eq!(d.cursor(), expected);
            }
        }

        #[test]
        fn search_boundary_check_matches_full_segmentation(text in ".{0,100}") {
            let expected = text.grapheme_indices(true).map(|(byte, _)| byte).chain(std::iter::once(text.len())).collect::<Vec<_>>();
            for byte in text.char_indices().map(|(byte, _)| byte).chain(std::iter::once(text.len())) {
                prop_assert_eq!(byte_is_grapheme_boundary(&text, byte), expected.contains(&byte));
            }
        }

        #[test]
        fn random_commands_preserve_graphemes_and_undo(ops in prop::collection::vec((0u8..7, any::<usize>()), 0..100)) {
            let mut d = Document::from_text("Việt e\u{301} 👩‍💻\r\n界");
            let original = d.text();
            for (op, n) in ops {
                match op {
                    0 => { d.insert(["a", "\u{301}", "👩‍💻", "\r\n"][n % 4]).unwrap(); }
                    1 => d.move_cursor(Movement::Left, n % 2 == 0),
                    2 => d.move_cursor(Movement::Right, n % 2 == 0),
                    3 => { d.backspace().unwrap(); }
                    4 => { d.delete_forward().unwrap(); }
                    5 => d.move_cursor(Movement::Up, n % 2 == 0),
                    _ => d.move_cursor(Movement::Down, n % 2 == 0),
                }
                prop_assert!(d.is_grapheme_boundary(d.selection.anchor));
                prop_assert!(d.is_grapheme_boundary(d.selection.head));
            }
            let final_text = d.text();
            while d.undo().unwrap() {}
            prop_assert_eq!(d.text(), original);
            while d.redo().unwrap() {}
            prop_assert_eq!(d.text(), final_text);
        }
    }
}
