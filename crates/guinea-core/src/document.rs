use std::{
    collections::VecDeque,
    ops::Range,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use crate::{EditError, RopeyBuffer, TextBuffer, ViewState, coordinates::CharIndex};
use unicode_segmentation::UnicodeSegmentation;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Largest document a single replacement may produce.
pub const MAX_REPLACED_BYTES: usize = 128 * 1024 * 1024;

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
    /// Stable filesystem object identity when supplied by the I/O adapter.
    pub identity: Option<[u64; 2]>,
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
    /// Previous/next word boundary (Unicode word segmentation), crossing
    /// line ends like Notepad++/Scintilla Ctrl+Left/Right.
    WordLeft,
    WordRight,
}

/// Typing-style edits that may merge into one Undo step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Typing {
    Insert,
    Backspace,
    Delete,
}

/// Consecutive typing merges into one Undo step unless paused this long.
const TYPING_WINDOW: std::time::Duration = std::time::Duration::from_secs(1);

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
    charge: usize,
}

/// Bounds retained edit payload accounting and transaction count, not OS RSS.
#[derive(Debug, Clone, Copy)]
pub struct HistoryLimits {
    pub max_transactions: usize,
    pub max_edit_bytes: usize,
}
impl Default for HistoryLimits {
    fn default() -> Self {
        Self {
            max_transactions: 2048,
            max_edit_bytes: 64 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct HistoryStatus {
    pub undo: usize,
    pub redo: usize,
    pub charged_bytes: usize,
    pub trimmed: u64,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: u64,
    pub path: Option<PathBuf>,
    /// Recovery provenance only; never used as a save destination.
    pub recovery_source: Option<PathBuf>,
    pub readonly: bool,
    pub bom: bool,
    pub fingerprint: Option<Fingerprint>,
    buffer: RopeyBuffer,
    content_state: u64,
    saved_state: u64,
    next_content_state: u64,
    saved_bom: bool,
    pub(crate) view: ViewState,
    undo: VecDeque<Transaction>,
    history_bytes: usize,
    history_trimmed: u64,
    history_limits: HistoryLimits,
    redo: Vec<Transaction>,
    revision: u64,
    newline_policy: LineEnding,
    typing_group: Option<(Typing, std::time::Instant)>,
    /// (revision, first changed scalar) for recent revisions; lets display
    /// caches such as multi-line syntax state recompute only changed lines.
    change_log: VecDeque<(u64, usize)>,
    /// Bookmarked line numbers; they follow edits, Undo and Redo.
    pub(crate) bookmarks: std::collections::BTreeSet<usize>,
}

impl Default for Document {
    fn default() -> Self {
        Self::from_text("")
    }
}

impl Document {
    pub fn from_text(text: &str) -> Self {
        Self::from_buffer(RopeyBuffer::from(text))
    }
    pub fn from_utf8_reader(reader: impl std::io::Read) -> std::io::Result<Self> {
        RopeyBuffer::from_reader(reader).map(Self::from_buffer)
    }
    fn from_buffer(buffer: RopeyBuffer) -> Self {
        let mut cr = 0usize;
        let mut lf = 0usize;
        let mut pairs = 0usize;
        let mut previous = 0u8;
        buffer
            .visit_chunks(0..buffer.len_chars(), &mut |chunk| {
                for byte in chunk.bytes() {
                    cr += usize::from(byte == b'\r');
                    lf += usize::from(byte == b'\n');
                    pairs += usize::from(previous == b'\r' && byte == b'\n');
                    previous = byte;
                }
                true
            })
            .expect("full range");
        let newline_policy = if cr > pairs || (pairs > 0 && lf > pairs) {
            LineEnding::Mixed
        } else if pairs > 0 {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        };
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            path: None,
            recovery_source: None,
            readonly: false,
            bom: false,
            fingerprint: None,
            content_state: 0,
            saved_state: 0,
            next_content_state: 1,
            saved_bom: false,
            buffer,
            view: ViewState::default(),
            undo: VecDeque::new(),
            history_bytes: 0,
            history_trimmed: 0,
            history_limits: HistoryLimits::default(),
            redo: Vec::new(),
            revision: 0,
            newline_policy,
            typing_group: None,
            change_log: VecDeque::new(),
            bookmarks: std::collections::BTreeSet::new(),
        }
    }

    pub fn buffer(&self) -> &impl TextBuffer {
        &self.buffer
    }
    /// Concrete storage for crate-internal O(1) snapshots (layout workers).
    pub(crate) fn rope_buffer(&self) -> &RopeyBuffer {
        &self.buffer
    }
    pub fn text(&self) -> String {
        self.buffer.text()
    }
    /// Cheap immutable content snapshot for background rendering.
    pub fn snapshot_buffer(&self) -> crate::ContentSnapshot {
        self.buffer.content_snapshot()
    }
    /// History-free editable/access-preserving copy for background preparation.
    /// Does not share display caches with an I/O worker.
    pub fn worker_snapshot(&self) -> Self {
        Self {
            id: self.id,
            path: self.path.clone(),
            recovery_source: self.recovery_source.clone(),
            readonly: self.readonly,
            bom: self.bom,
            fingerprint: self.fingerprint.clone(),
            buffer: self.buffer.clone(),
            content_state: self.content_state,
            saved_state: self.saved_state,
            next_content_state: self.next_content_state,
            saved_bom: self.saved_bom,
            view: ViewState {
                selection: self.view.selection,
                ..ViewState::default()
            },
            undo: VecDeque::new(),
            history_bytes: 0,
            history_trimmed: 0,
            history_limits: HistoryLimits::default(),
            redo: Vec::new(),
            revision: self.revision,
            newline_policy: self.newline_policy,
            typing_group: None,
            change_log: VecDeque::new(),
            bookmarks: std::collections::BTreeSet::new(),
        }
    }
    /// Read-only linked view; never retains editable history or copies text.
    pub fn review_snapshot(&self) -> Self {
        let mut snapshot = self.worker_snapshot();
        snapshot.readonly = true;
        snapshot.view.selection = Selection::default();
        snapshot.view.layout_cache = self.view.layout_cache.clone();
        snapshot
    }
    pub fn history_status(&self) -> HistoryStatus {
        HistoryStatus {
            undo: self.undo.len(),
            redo: self.redo.len(),
            charged_bytes: self.history_bytes,
            trimmed: self.history_trimmed,
        }
    }
    pub fn set_history_limits(&mut self, limits: HistoryLimits) {
        self.history_limits = limits;
        self.trim_history();
    }
    fn clear_redo(&mut self) {
        self.history_bytes = self
            .history_bytes
            .saturating_sub(self.redo.iter().map(|t| t.charge).sum::<usize>());
        self.redo.clear();
    }
    fn push_history(&mut self, tx: Transaction) {
        self.typing_group = None;
        self.history_bytes = self.history_bytes.saturating_add(tx.charge);
        self.undo.push_back(tx);
        self.trim_history();
    }
    fn trim_history(&mut self) {
        // Keep the nearest Undo, or the nearest Redo if there is no Undo.
        while self.undo.len() + self.redo.len() > 1
            && (self.undo.len() + self.redo.len() > self.history_limits.max_transactions.max(1)
                || self.history_bytes > self.history_limits.max_edit_bytes)
        {
            let removed = if self.undo.len() > 1 || self.redo.is_empty() {
                self.undo.pop_front()
            } else if !self.redo.is_empty() {
                Some(self.redo.remove(0))
            } else {
                None
            };
            if let Some(tx) = removed {
                self.history_bytes = self.history_bytes.saturating_sub(tx.charge);
                self.history_trimmed += 1;
            } else {
                break;
            }
        }
    }
    /// Reopened content keeps the tab identity but advances its view revision,
    /// so cached previews cannot mistake a reload for the old saved buffer.
    pub fn continue_identity(&mut self, previous: &Self) {
        self.id = previous.id;
        self.revision = previous.revision + 1;
        self.clear_view_cache();
    }
    /// Swap an independent view without cloning document contents or history.
    /// Stale detached views are rejected instead of guessing edit mappings.
    pub fn swap_view(&mut self, view: &mut ViewState) -> Result<(), EditError> {
        if view
            .binding
            .is_some_and(|binding| binding != (self.id, self.revision))
        {
            return Err(EditError::RevisionConflict);
        }
        if !self.is_grapheme_boundary(view.selection.anchor)
            || !self.is_grapheme_boundary(view.selection.head)
        {
            return Err(EditError::GraphemeBoundary);
        }
        std::mem::swap(&mut self.view, view);
        self.view.binding = Some((self.id, self.revision));
        view.binding = Some((self.id, self.revision));
        Ok(())
    }
    pub fn selection(&self) -> Selection {
        self.view.selection
    }
    pub fn cursor(&self) -> usize {
        self.view.selection.head
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
        self.typing_group = None;
        self.saved_state = self.content_state;
        self.saved_bom = self.bom;
    }

    /// Recovery always creates a new unsaved draft, never a writable disk path.
    pub fn mark_recovered(&mut self) {
        self.path = None;
        self.fingerprint = None;
        self.content_state = self.next_content_state;
        self.next_content_state += 1;
        self.revision += 1;
        self.note_change(0);
    }

    /// Move bookmarks after `old[start..start + old_len]` became
    /// `new[start..start + new_len]` (`new` is the current buffer). Lines that
    /// were removed lose their bookmark; inserting whole lines at a line start
    /// moves the bookmarks of that line and below down with their text.
    fn shift_bookmarks(&mut self, old: &RopeyBuffer, start: usize, old_len: usize, new_len: usize) {
        if self.bookmarks.is_empty() {
            return;
        }
        let (Ok(first), Ok(old_end), Ok(new_end), Ok(line_start)) = (
            old.char_to_line(start),
            old.char_to_line(start + old_len),
            self.buffer.char_to_line(start + new_len),
            old.line_to_char(old.char_to_line(start).unwrap_or(0)),
        ) else {
            self.bookmarks.clear();
            return;
        };
        let removed = old_end - first;
        let inserted = new_end - first;
        // A change from a line start to a line start replaces whole lines:
        // those lines' bookmarks go, and the lines after it move with their
        // text (including plain insertion of new lines at a line start).
        let whole_lines =
            start == line_start && old.line_to_char(old_end).ok() == Some(start + old_len);
        let shifted = self
            .bookmarks
            .iter()
            .filter_map(|&line| {
                let kept = if whole_lines {
                    line < first
                } else {
                    line <= first
                };
                if kept {
                    Some(line)
                } else if line < first + removed + usize::from(!whole_lines) {
                    None
                } else {
                    Some(line + inserted - removed)
                }
            })
            .collect();
        self.bookmarks = shifted;
    }

    fn note_change(&mut self, start: usize) {
        self.change_log.push_back((self.revision, start));
        while self.change_log.len() > 64 {
            self.change_log.pop_front();
        }
    }

    /// The first scalar that may differ from `revision`'s text, `usize::MAX`
    /// when nothing changed, or `None` when the log no longer covers it.
    pub fn earliest_change_since(&self, revision: u64) -> Option<usize> {
        if revision >= self.revision {
            return Some(usize::MAX);
        }
        let newer = self
            .change_log
            .iter()
            .filter(|(at, _)| *at > revision)
            .collect::<Vec<_>>();
        let covered = newer.len() as u64 == self.revision - revision
            && newer.first().is_some_and(|(at, _)| *at == revision + 1);
        covered.then(|| newer.iter().map(|(_, start)| *start).min().unwrap_or(0))
    }

    /// Merge only metadata from the snapshot committed by a background save.
    /// Edits and selection made while I/O was running remain in this document.
    pub fn accept_saved_snapshot(&mut self, saved: &Self) {
        assert_eq!(self.id, saved.id);
        // Later typing must not merge into the step that reached the save.
        self.typing_group = None;
        self.path = saved.path.clone();
        self.recovery_source = saved.recovery_source.clone();
        self.fingerprint = saved.fingerprint.clone();
        self.saved_state = saved.saved_state;
        self.saved_bom = saved.saved_bom;
    }

    pub fn selected_text(&self) -> String {
        self.buffer
            .slice(self.view.selection.range())
            .expect("valid selection")
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            buffer: self.buffer.clone(),
            selection: self.view.selection,
            content_state: self.content_state,
        }
    }

    fn restore(&mut self, snapshot: &Snapshot) {
        self.buffer = snapshot.buffer.clone();
        self.view.selection = snapshot.selection;
        self.content_state = snapshot.content_state;
        self.revision += 1;
        self.view.preferred_column = None;
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
        self.view.selection = Selection { anchor, head };
        self.view.preferred_column = None;
        self.typing_group = None;
        Ok(())
    }

    pub fn select_all(&mut self) {
        self.typing_group = None;
        self.view.selection = Selection {
            anchor: 0,
            head: self.buffer.len_chars(),
        };
        self.view.preferred_column = None;
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
        let charge = self.buffer.char_to_byte(CharIndex(range.end))?.0
            - self.buffer.char_to_byte(CharIndex(range.start))?.0
            + text.len()
            + 4096;
        let changed = !self.buffer.range_equals(range.clone(), text)?;
        self.buffer.remove(range.clone())?;
        self.buffer.insert(range.start, text)?;
        let cursor = self.snap_forward(range.start + text.chars().count());
        self.view.selection = Selection {
            anchor: cursor,
            head: cursor,
        };
        self.view.preferred_column = None;
        if changed {
            self.content_state = self.next_content_state;
            self.next_content_state += 1;
            self.revision += 1;
            self.note_change(range.start);
            let inserted = text.chars().count();
            self.shift_bookmarks(&before.buffer, range.start, range.len(), inserted);
            self.apply_layout_edit(layout_edit, text);
            self.clear_redo();
            self.push_history(Transaction {
                before,
                after: self.snapshot(),
                start: range.start,
                removed: range.len(),
                inserted: text.chars().count(),
                charge,
            });
        }
        Ok(())
    }

    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        self.replace_range(self.view.selection.range(), text)
    }

    /// Insert typed text. Contiguous typing within [`TYPING_WINDOW`] of the
    /// previous keystroke extends the same Undo step (including newlines, so a
    /// paste delivered as key events stays one step); caret moves, other
    /// edits, Undo/Redo and saves start a new step.
    pub fn type_text(&mut self, text: &str) -> Result<(), EditError> {
        self.typed(Typing::Insert, self.view.selection.range(), text)
    }

    /// Backspace that merges with preceding typed Backspaces.
    pub fn type_backspace(&mut self) -> Result<(), EditError> {
        let range = if self.view.selection.is_empty() {
            self.previous(self.cursor())..self.cursor()
        } else {
            self.view.selection.range()
        };
        self.typed(Typing::Backspace, range, "")
    }

    /// Forward Delete that merges with preceding typed Deletes.
    pub fn type_delete(&mut self) -> Result<(), EditError> {
        let range = if self.view.selection.is_empty() {
            self.cursor()..self.next(self.cursor())
        } else {
            self.view.selection.range()
        };
        self.typed(Typing::Delete, range, "")
    }

    /// End the current typing group; the next keystroke starts a new Undo step.
    pub fn break_typing(&mut self) {
        self.typing_group = None;
    }

    fn typed(&mut self, kind: Typing, range: Range<usize>, text: &str) -> Result<(), EditError> {
        let group = self.typing_group.take();
        let selection_replaced = !self.view.selection.is_empty();
        let revision = self.revision;
        self.replace_range(range, text)?;
        if self.revision == revision {
            return Ok(());
        }
        let mergeable = !selection_replaced
            && group.is_some_and(|(previous, at)| previous == kind && at.elapsed() < TYPING_WINDOW)
            && self.undo.len() >= 2;
        if mergeable && let Some(new) = self.undo.pop_back() {
            let previous = self.undo.back_mut().expect("two undo steps");
            let contiguous = match kind {
                Typing::Insert => {
                    new.removed == 0 && new.start == previous.start + previous.inserted
                }
                Typing::Backspace => {
                    previous.inserted == 0
                        && new.inserted == 0
                        && new.start + new.removed == previous.start
                }
                Typing::Delete => {
                    previous.inserted == 0 && new.inserted == 0 && new.start == previous.start
                }
            };
            if contiguous {
                match kind {
                    Typing::Insert => previous.inserted += new.inserted,
                    Typing::Backspace => {
                        previous.start = new.start;
                        previous.removed += new.removed;
                    }
                    Typing::Delete => previous.removed += new.removed,
                }
                previous.after = new.after;
                // Each step charges a 4096-byte base; a merged step keeps one.
                previous.charge = previous
                    .charge
                    .saturating_add(new.charge.saturating_sub(4096));
                self.history_bytes = self.history_bytes.saturating_sub(4096);
            } else {
                self.undo.push_back(new);
            }
        }
        self.typing_group = Some((kind, std::time::Instant::now()));
        Ok(())
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
                .range_equals(e.start..e.end, &e.text)
                .unwrap_or(false)
        }) {
            return Ok(());
        }
        let mut prepared = self.review_snapshot();
        if ordered.len() <= 32 {
            for edit in ordered.into_iter().rev() {
                prepared.buffer.remove(edit.start..edit.end)?;
                prepared.buffer.insert(edit.start, &edit.text)?;
            }
        } else {
            // Many edits (indent/comment a whole file, remove marked lines):
            // compose the edited span in one pass and splice it once, instead
            // of one rope insert/remove per line.
            let span = ordered[0].start..ordered[ordered.len() - 1].end;
            let original = prepared.buffer.slice(span.clone())?;
            let mut output = String::with_capacity(
                original.len() + ordered.iter().map(|e| e.text.len()).sum::<usize>(),
            );
            let mut chars = original.char_indices();
            let (mut position, mut byte) = (span.start, 0);
            let mut advance = |target: usize| {
                while position < target {
                    let (at, c) = chars.next().expect("validated edit bounds");
                    byte = at + c.len_utf8();
                    position += 1;
                }
                byte
            };
            let mut copied = 0;
            for edit in &ordered {
                let start = advance(edit.start);
                output.push_str(&original[copied..start]);
                output.push_str(&edit.text);
                copied = advance(edit.end);
            }
            output.push_str(&original[copied..]);
            prepared.buffer.remove(span.clone())?;
            prepared.buffer.insert(span.start, &output)?;
        }
        prepared.view.selection = Selection::default();
        self.accept_prepared(&prepared, self.revision)
    }

    /// Replace `span` with `text` as one Undo step, keeping bookmarks on their
    /// line numbers. For whole-block rewrites that keep the line count (indent,
    /// comment): two rope edits instead of one per line.
    pub(crate) fn replace_block(
        &mut self,
        span: Range<usize>,
        text: &str,
    ) -> Result<(), EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        if span.end > self.buffer.len_chars() || span.start > span.end {
            return Err(EditError::OutOfBounds);
        }
        if !self.is_grapheme_boundary(span.start) || !self.is_grapheme_boundary(span.end) {
            return Err(EditError::GraphemeBoundary);
        }
        let mut prepared = self.review_snapshot();
        prepared.buffer.remove(span.clone())?;
        prepared.buffer.insert(span.start, text)?;
        prepared.view.selection = Selection::default();
        self.accept_prepared(&prepared, self.revision)
    }

    /// Commit an off-thread prepared snapshot by swapping shared buffer state,
    /// retaining this document's history, identity, path and save metadata.
    /// History trimming/cache release also run here and have separate costs.
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
        let charge = self
            .buffer
            .len_bytes()
            .saturating_add(prepared.buffer.len_bytes())
            .saturating_add(4096);
        self.buffer = prepared.buffer.clone();
        self.view.selection = prepared.view.selection;
        self.content_state = self.next_content_state;
        self.next_content_state += 1;
        self.revision += 1;
        self.note_change(0);
        let lines = self.buffer.len_lines();
        self.bookmarks.retain(|line| *line < lines);
        self.view.preferred_column = None;
        self.clear_view_cache();
        self.clear_redo();
        self.push_history(Transaction {
            before,
            after: self.snapshot(),
            start: 0,
            removed,
            inserted: self.buffer.len_chars(),
            charge,
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
        let range = if self.view.selection.is_empty() {
            self.previous(self.cursor())..self.cursor()
        } else {
            self.view.selection.range()
        };
        self.replace_range(range, "")
    }

    pub fn delete_forward(&mut self) -> Result<(), EditError> {
        let range = if self.view.selection.is_empty() {
            self.cursor()..self.next(self.cursor())
        } else {
            self.view.selection.range()
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
            .view
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
            Movement::Left if !extend && !self.view.selection.is_empty() => {
                self.view.selection.range().start
            }
            Movement::Right if !extend && !self.view.selection.is_empty() => {
                self.view.selection.range().end
            }
            Movement::Left => self.previous(self.cursor()),
            Movement::Right => self.next(self.cursor()),
            Movement::Start => 0,
            Movement::Finish => self.buffer.len_chars(),
            Movement::Home => self.buffer.line_to_char(line).expect("valid line"),
            Movement::End => self.line_end(line),
            Movement::WordLeft => self.word_left(self.cursor()),
            Movement::WordRight => self.word_right(self.cursor()),
            Movement::Up | Movement::Down => {
                let goal = self
                    .view
                    .preferred_column
                    .unwrap_or_else(|| self.position().1);
                self.view.preferred_column = Some(goal);
                let target = if matches!(movement, Movement::Up) {
                    line.saturating_sub(1)
                } else {
                    (line + 1).min(self.buffer.len_lines() - 1)
                };
                self.grapheme_column_to_char(target, goal)
                    .expect("valid line")
            }
        };
        self.typing_group = None;
        self.view.selection.head = idx;
        if !extend {
            self.view.selection.anchor = idx;
        }
        if !vertical {
            self.view.preferred_column = None;
        }
    }

    fn line_end(&self, line: usize) -> usize {
        self.line_bounds(line).expect("valid line").end
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        if self.readonly {
            return Err(EditError::ReadOnly);
        }
        self.typing_group = None;
        if let Some(tx) = self.undo.pop_back() {
            let replacement = self.view.layout_cache.borrow().as_ref().map(|_| {
                tx.before
                    .buffer
                    .slice(tx.start..tx.start + tx.removed)
                    .expect("history range")
            });
            let layout_edit = replacement.as_ref().and_then(|text| {
                self.prepare_layout_edit(&(tx.start..tx.start + tx.inserted), text)
            });
            let after = tx.after.buffer.clone();
            self.restore(&tx.before);
            self.note_change(tx.start);
            self.shift_bookmarks(&after, tx.start, tx.inserted, tx.removed);
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
        self.typing_group = None;
        if let Some(tx) = self.redo.pop() {
            let replacement = self.view.layout_cache.borrow().as_ref().map(|_| {
                tx.after
                    .buffer
                    .slice(tx.start..tx.start + tx.inserted)
                    .expect("history range")
            });
            let layout_edit = replacement.as_ref().and_then(|text| {
                self.prepare_layout_edit(&(tx.start..tx.start + tx.removed), text)
            });
            let before = tx.before.buffer.clone();
            self.restore(&tx.after);
            self.note_change(tx.start);
            self.shift_bookmarks(&before, tx.start, tx.removed, tx.inserted);
            self.apply_layout_edit(layout_edit, replacement.as_deref().unwrap_or(""));
            self.undo.push_back(tx);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Literal search wraps; matches splitting a grapheme are skipped.
    pub fn find(&mut self, needle: &str) -> bool {
        self.find_with(needle, crate::SearchOptions::default())
    }

    /// Select the next match at or after the selection end, wrapping once.
    pub fn find_with(&mut self, needle: &str, options: crate::SearchOptions) -> bool {
        if needle.is_empty() {
            return false;
        }
        let text = self.text();
        let cursor_byte = crate::coordinates::char_to_byte(
            &text,
            crate::coordinates::CharIndex(self.view.selection.range().end),
        )
        .expect("valid selection")
        .0;
        let mut first = None;
        let mut chosen = None;
        for range in crate::search::find_matches(&text, needle, options) {
            first.get_or_insert(range.clone());
            if range.start >= cursor_byte {
                chosen = Some(range);
                break;
            }
        }
        if let Some(range) = chosen.or(first) {
            let start = text[..range.start].chars().count();
            let end = start + text[range].chars().count();
            self.select(start, end).expect("checked boundaries");
            true
        } else {
            false
        }
    }

    /// Replace the one match at scalar `range` (as found by a search with the
    /// same query and options), expanding regex capture references.
    pub fn replace_match(
        &mut self,
        range: Range<usize>,
        needle: &str,
        replacement: &str,
        options: crate::SearchOptions,
    ) -> Result<(), EditError> {
        let text = if options.regex {
            let start = self.buffer.char_to_byte(CharIndex(range.start))?.0;
            let end = self.buffer.char_to_byte(CharIndex(range.end))?.0;
            crate::search::expand_replacement(
                &self.text(),
                start..end,
                needle,
                options,
                replacement,
            )
        } else {
            replacement.to_string()
        };
        if text.len() > MAX_REPLACED_BYTES {
            return Err(EditError::TooLarge);
        }
        self.replace_range(range, &text)
    }

    pub fn replace_all(&mut self, needle: &str, replacement: &str) -> Result<usize, EditError> {
        self.replace_all_with(needle, replacement, crate::SearchOptions::default())
    }

    /// Replace every match as one transaction (one Undo step).
    pub fn replace_all_with(
        &mut self,
        needle: &str,
        replacement: &str,
        options: crate::SearchOptions,
    ) -> Result<usize, EditError> {
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
        for range in crate::search::find_matches(&text, needle, options) {
            output.push_str(&text[previous_byte..range.start]);
            output.push_str(&crate::search::expand_replacement(
                &text,
                range.clone(),
                needle,
                options,
                replacement,
            ));
            previous_byte = range.end;
            count += 1;
            // `$0$0…` in regex mode can grow output far beyond the query.
            if output.len() > MAX_REPLACED_BYTES {
                return Err(EditError::TooLarge);
            }
        }
        if count > 0 {
            output.push_str(&text[previous_byte..]);
            self.replace_range(0..self.buffer.len_chars(), &output)?;
        }
        Ok(count)
    }
}

#[cfg(test)]
fn byte_is_grapheme_boundary(text: &str, byte: usize) -> bool {
    unicode_segmentation::GraphemeCursor::new(byte, text.len(), true)
        .is_boundary(text, 0)
        .expect("complete UTF-8 snapshot provides all grapheme context")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_snapshot_is_history_free_and_preserves_original_access_and_selection() {
        let mut d = Document::from_text("Việt 👩‍💻\r\n");
        for _ in 0..40 {
            d.insert("x").unwrap();
        }
        let mut worker = d.worker_snapshot();
        let content = d.snapshot_buffer();
        assert_eq!(worker.selection(), d.selection());
        assert_eq!(worker.history_status().undo, 0);
        assert_eq!(worker.history_status().redo, 0);
        worker.insert("worker").unwrap();
        assert_eq!(content.text(), d.text());
        assert_ne!(worker.text(), d.text());
        assert_eq!(d.history_status().undo, 40);
        d.readonly = true;
        assert!(d.worker_snapshot().insert("no").is_err());
    }

    #[test]
    fn bounded_history_keeps_latest_undo_and_never_marks_trimmed_content_saved() {
        let mut d = Document::default();
        d.set_history_limits(HistoryLimits {
            max_transactions: 3,
            max_edit_bytes: 20_000,
        });
        for _ in 0..20 {
            d.insert("界").unwrap();
        }
        assert_eq!(d.history_status().undo, 3);
        assert_eq!(d.history_status().trimmed, 17);
        for _ in 0..3 {
            assert!(d.undo().unwrap());
        }
        assert!(!d.undo().unwrap());
        assert_eq!(d.text(), "界".repeat(17));
        assert!(d.is_dirty());
        d.redo().unwrap();
        d.mark_saved();
        d.redo().unwrap();
        assert!(d.is_dirty());
        d.undo().unwrap();
        assert!(!d.is_dirty());
        d.insert("new branch").unwrap();
        assert!(!d.redo().unwrap());
        assert!(d.is_dirty());
        assert!(d.history_status().charged_bytes <= 20_000);
    }

    #[test]
    fn independent_view_states_do_not_fork_text_or_history_and_reject_stale_positions() {
        let mut d = Document::from_text("e\u{301}界\nother");
        d.select(2, 3).unwrap();
        let mut second = ViewState::default();
        d.swap_view(&mut second).unwrap();
        assert_eq!(d.cursor(), 0);
        assert_eq!(second.selection(), Selection { anchor: 2, head: 3 });
        d.select(4, 4).unwrap();
        d.swap_view(&mut second).unwrap();
        assert_eq!(d.selection(), Selection { anchor: 2, head: 3 });
        assert_eq!(second.selection().head, 4);
        d.insert("X").unwrap();
        assert_eq!(d.text(), "e\u{301}X\nother");
        assert_eq!(d.swap_view(&mut second), Err(EditError::RevisionConflict));
        d.undo().unwrap();
        assert_eq!(d.text(), "e\u{301}界\nother");
    }
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
                prop_assert!(d.is_grapheme_boundary(d.selection().anchor));
                prop_assert!(d.is_grapheme_boundary(d.selection().head));
            }
            let final_text = d.text();
            while d.undo().unwrap() {}
            prop_assert_eq!(d.text(), original);
            while d.redo().unwrap() {}
            prop_assert_eq!(d.text(), final_text);
        }
    }

    #[test]
    fn regex_replace_all_and_replace_match_expand_groups_in_one_undo() {
        let options = crate::SearchOptions {
            regex: true,
            ..Default::default()
        };
        let mut d = Document::from_text("x=1\r\ny=22\r\n");
        assert_eq!(
            d.replace_all_with(r"(\w)=(\d+)", "$2=$1", options).unwrap(),
            2
        );
        assert_eq!(d.text(), "1=x\r\n22=y\r\n");
        assert!(d.undo().unwrap());
        assert_eq!(d.text(), "x=1\r\ny=22\r\n");
        // Scalar range 5..9 is "y=22" after the CRLF.
        d.replace_match(5..9, r"(\w)=(\d+)", "[$1]", options)
            .unwrap();
        assert_eq!(d.text(), "x=1\r\n[y]\r\n");
    }
}
