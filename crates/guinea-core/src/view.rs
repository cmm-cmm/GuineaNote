use crate::{
    Document, EditError, TextBuffer,
    coordinates::{CharIndex, cell_column, display_grapheme},
};
use std::{borrow::Cow, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

/// Keep deleted ASCII prefix bytes as reusable space in the display cache.
/// The document's Ropey storage remains the source of truth. This avoids moving
/// a multi-megabyte suffix for each prefix Backspace/Undo/Redo.
#[derive(Debug, Clone)]
struct LayoutText<'a> {
    storage: Cow<'a, str>,
    head: usize,
}
impl<'a> From<Cow<'a, str>> for LayoutText<'a> {
    fn from(storage: Cow<'a, str>) -> Self {
        Self { storage, head: 0 }
    }
}
impl std::ops::Deref for LayoutText<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.storage[self.head..]
    }
}
impl LayoutText<'_> {
    fn replace(&mut self, bytes: std::ops::Range<usize>, text: &str, ascii: bool) {
        let end = self.head + bytes.end;
        if ascii && bytes.start == 0 && end >= text.len() {
            // Only printable ASCII is eligible, so the reused prefix starts
            // and ends on UTF-8 boundaries. The physical length stays fixed.
            let head = end - text.len();
            if !text.is_empty() {
                self.storage.to_mut().replace_range(head..end, text);
            }
            self.head = head;
        } else {
            self.storage
                .to_mut()
                .replace_range(self.head + bytes.start..end, text);
        }
    }
}

#[derive(Debug, Clone)]
struct Checkpoint {
    chars: usize,
    byte: usize,
    cells: usize,
    graphemes: usize,
}

/// Sparse display index for long lines, including ASCII and tabs. Short lines
/// keep their direct path; cached text and indexes are shared by clones.
#[derive(Debug, Clone)]
pub(crate) struct LineLayout<'a> {
    line: usize,
    revision: u64,
    tab: usize,
    start: usize,
    text: LayoutText<'a>,
    checkpoints: Vec<Checkpoint>,
    scanned: Checkpoint,
    first_tab: Option<(usize, usize)>,
}

fn cells(g: &str, cell: usize, tab: usize) -> usize {
    if g.len() == 1 && g != "\t" {
        return 1;
    }
    let first = g.chars().next().expect("nonempty grapheme");
    if g.len() == first.len_utf8() && first != '\t' {
        return unicode_width::UnicodeWidthChar::width(first)
            .unwrap_or(1)
            .max(1);
    }
    if g == "\t" {
        tab.max(1) - cell % tab.max(1)
    } else if g.chars().any(char::is_control) {
        1
    } else {
        unicode_width::UnicodeWidthStr::width(g).max(1)
    }
}

impl LineLayout<'_> {
    fn scan_to(&mut self, through_char: usize, through_cell: usize) {
        while self.scanned.byte < self.text.len()
            && (self.scanned.chars < through_char || self.scanned.cells < through_cell)
        {
            let point = &mut self.scanned;
            let tail = &self.text[point.byte..];
            // Inspect only the needed prefix plus one lookahead byte. A long
            // hidden ASCII suffix must not be scanned for a small viewport.
            let needed = through_char
                .saturating_sub(point.chars)
                .max(through_cell.saturating_sub(point.cells))
                .saturating_add(1);
            let ascii = tail
                .bytes()
                .take(needed)
                .take_while(|b| b.is_ascii() && *b != b'\r')
                .count();
            // The last ASCII scalar can combine with the following Unicode
            // scalar (e + combining accent); leave it to segmentation.
            let ascii = if ascii < tail.len() && !tail.as_bytes()[ascii].is_ascii() {
                ascii.saturating_sub(1)
            } else {
                ascii
            };
            if ascii > 0 {
                for byte in tail[..ascii].bytes() {
                    if point.chars >= through_char && point.cells >= through_cell {
                        break;
                    }
                    if point.chars - self.checkpoints.last().unwrap().chars >= 512 {
                        self.checkpoints.push(point.clone());
                    }
                    point.byte += 1;
                    point.chars += 1;
                    point.graphemes += 1;
                    if byte == b'\t' && self.first_tab.is_none() {
                        self.first_tab = Some((point.chars - 1, point.cells));
                    }
                    point.cells += if byte == b'\t' {
                        self.tab.max(1) - point.cells % self.tab.max(1)
                    } else {
                        1
                    };
                }
            } else {
                let g = tail.graphemes(true).next().unwrap();
                if g == "\t" && self.first_tab.is_none() {
                    self.first_tab = Some((point.chars, point.cells));
                }
                if point.chars - self.checkpoints.last().unwrap().chars >= 512 {
                    self.checkpoints.push(point.clone());
                }
                point.byte += g.len();
                point.chars += g.chars().count();
                point.cells += cells(g, point.cells, self.tab);
                point.graphemes += 1;
            }
        }
    }
    fn parallel_index(&mut self) {
        let workers = std::thread::available_parallelism().map_or(2, |n| n.get().min(8));
        let mut boundaries = vec![0];
        for worker in 1..workers {
            let mut byte = self.text.len() * worker / workers;
            while !self.text.is_char_boundary(byte) {
                byte += 1;
            }
            let mut cursor = unicode_segmentation::GraphemeCursor::new(byte, self.text.len(), true);
            if !cursor.is_boundary(&self.text, 0).expect("complete text") {
                byte = cursor
                    .next_boundary(&self.text, 0)
                    .expect("complete text")
                    .unwrap_or(self.text.len());
            }
            if byte > *boundaries.last().unwrap() {
                boundaries.push(byte);
            }
        }
        boundaries.push(self.text.len());
        let tab = self.tab;
        let text: &str = &self.text;
        let segments = std::thread::scope(|scope| {
            let jobs = boundaries
                .windows(2)
                .map(|range| {
                    let text = &text[range[0]..range[1]];
                    scope.spawn(move || {
                        let zero = Checkpoint {
                            chars: 0,
                            byte: 0,
                            cells: 0,
                            graphemes: 0,
                        };
                        let mut segment = LineLayout {
                            line: 0,
                            revision: 0,
                            tab,
                            start: 0,
                            // Scoped workers borrow disjoint ranges of the
                            // existing line; only the sparse index is allocated.
                            text: Cow::Borrowed(text).into(),
                            checkpoints: vec![zero.clone()],
                            scanned: zero,
                            first_tab: None,
                        };
                        segment.scan_to(usize::MAX, usize::MAX);
                        segment
                    })
                })
                .collect::<Vec<_>>();
            jobs.into_iter()
                .map(|job| job.join().expect("display index worker"))
                .collect::<Vec<_>>()
        });
        let mut base = Checkpoint {
            chars: 0,
            byte: 0,
            cells: 0,
            graphemes: 0,
        };
        self.checkpoints.clear();
        let mut first_tab = None;
        for segment in segments {
            if first_tab.is_none() {
                first_tab = segment
                    .first_tab
                    .map(|(chars, cell)| (base.chars + chars, base.cells + cell));
            }
            let offset = |point: &Checkpoint| {
                let cells = if let Some((chars, before)) =
                    segment.first_tab.filter(|(chars, _)| point.chars > *chars)
                {
                    let _ = chars;
                    let local_after = before + tab.max(1) - before % tab.max(1);
                    let global_after =
                        base.cells + before + tab.max(1) - (base.cells + before) % tab.max(1);
                    point.cells + global_after - local_after
                } else {
                    point.cells + base.cells
                };
                Checkpoint {
                    chars: point.chars + base.chars,
                    byte: point.byte + base.byte,
                    cells,
                    graphemes: point.graphemes + base.graphemes,
                }
            };
            self.checkpoints
                .extend(segment.checkpoints.iter().map(offset));
            base = offset(&segment.scanned);
        }
        self.scanned = base;
        self.first_tab = first_tab;
    }
    pub(crate) fn tab_width(&self) -> usize {
        self.tab
    }
    pub(crate) fn position(&self, cursor: usize) -> (usize, usize) {
        let target = cursor - self.start;
        let checkpoint = &self.checkpoints[self
            .checkpoints
            .partition_point(|p| p.chars <= target)
            .saturating_sub(1)];
        let mut chars = checkpoint.chars;
        let mut col = checkpoint.graphemes;
        let mut cell = checkpoint.cells;
        for g in self.text[checkpoint.byte..].graphemes(true) {
            if chars >= target {
                break;
            }
            chars += g.chars().count();
            cell += cells(g, cell, self.tab);
            col += 1;
        }
        (col, cell)
    }
    fn window(&self, left: usize, width: usize) -> LineWindow {
        let checkpoint = &self.checkpoints[self
            .checkpoints
            .partition_point(|p| p.cells <= left)
            .saturating_sub(1)];
        let mut cell = checkpoint.cells;
        let mut chars = checkpoint.chars;
        let mut start = self.text.len();
        let mut end = start;
        let mut cell_start = cell;
        let mut char_start = chars;
        for (offset, g) in self.text[checkpoint.byte..].grapheme_indices(true) {
            if cell >= left.saturating_add(width) {
                break;
            }
            let count = cells(g, cell, self.tab);
            if cell + count > left {
                if start == self.text.len() {
                    start = checkpoint.byte + offset;
                    cell_start = cell;
                    char_start = chars;
                }
                end = checkpoint.byte + offset + g.len();
            }
            cell += count;
            chars += g.chars().count();
        }
        if start == self.text.len() {
            char_start = chars;
            cell_start = cell;
        }
        LineWindow {
            text: self.text[start..end].into(),
            char_start: self.start + char_start,
            cell_start,
        }
    }
}

/// A bounded viewport with absolute scalar and cell origins.
pub struct LineWindow {
    pub text: String,
    pub char_start: usize,
    pub cell_start: usize,
}
pub(crate) struct LayoutEdit {
    bytes: Option<std::ops::Range<usize>>,
    chars: usize,
    line: usize,
    revision: u64,
    delta: isize,
}

impl Document {
    /// Prepare bounded initial line indexes on the file-open worker.
    pub fn prepare_view(&self, tab_width: usize) {
        if self.buffer().len_bytes() <= 10 * 1024 * 1024 {
            for line in 0..self.buffer().len_lines().min(64) {
                let _ = self.indexed_line(
                    line,
                    tab_width,
                    self.line_bounds(line).map_or(0, |r| r.len()),
                    0,
                );
            }
            let _ = self.indexed_line(0, tab_width, 0, 0);
        }
    }
    pub fn view_cache_bytes(&self) -> usize {
        self.view
            .layout_cache
            .borrow()
            .as_ref()
            .map_or(0, |c| c.text.storage.len())
            + self
                .view
                .layout_others
                .borrow()
                .iter()
                .map(|c| c.text.storage.len())
                .sum::<usize>()
    }
    pub fn clear_view_cache(&self) {
        *self.view.layout_cache.borrow_mut() = None;
        self.view.layout_others.borrow_mut().clear();
    }
    pub(crate) fn prepare_layout_edit(
        &self,
        range: &std::ops::Range<usize>,
        text: &str,
    ) -> Option<LayoutEdit> {
        if text.contains(['\r', '\n']) {
            return None;
        }
        let line = self.buffer().char_to_line(range.start).ok()?;
        let bounds = self.line_bounds(line).ok()?;
        if range.end > bounds.end {
            return None;
        }
        let tab = self
            .view
            .layout_cache
            .borrow()
            .as_ref()
            .map_or(4, |c| c.tab);
        let indexed = self.indexed_line(line, tab, 0, 0).is_some();
        let cache = self.view.layout_cache.borrow();
        let bytes = if indexed {
            let layout = cache.as_ref()?;
            let origin = self.buffer().char_to_byte(CharIndex(layout.start)).ok()?.0;
            Some(
                self.buffer().char_to_byte(CharIndex(range.start)).ok()?.0 - origin
                    ..self.buffer().char_to_byte(CharIndex(range.end)).ok()?.0 - origin,
            )
        } else {
            None
        };
        Some(LayoutEdit {
            bytes,
            chars: range.start - bounds.start,
            line,
            revision: self.revision(),
            delta: text.chars().count() as isize - range.len() as isize,
        })
    }
    pub(crate) fn apply_layout_edit(&self, edit: Option<LayoutEdit>, text: &str) {
        let mut cache = self.view.layout_cache.borrow_mut();
        let Some(edit) = edit else {
            *cache = None;
            self.view.layout_others.borrow_mut().clear();
            return;
        };
        let Some(bytes) = edit.bytes else {
            let valid =
                |c: &Arc<LineLayout<'static>>| c.revision == edit.revision && c.line != edit.line;
            let remap = |c: &mut Arc<LineLayout<'static>>| {
                let c = Arc::make_mut(c);
                c.revision = self.revision();
                if c.line > edit.line {
                    c.start = c.start.saturating_add_signed(edit.delta);
                }
            };
            if cache.as_ref().is_some_and(valid) {
                remap(cache.as_mut().unwrap());
            } else {
                *cache = None;
            }
            let mut others = self.view.layout_others.borrow_mut();
            others.retain(valid);
            for c in others.iter_mut() {
                remap(c);
            }
            return;
        };
        let chars = edit.chars;
        let Some(layout) = cache.as_mut() else {
            return;
        };
        let layout = Arc::make_mut(layout);
        let scalar_delta =
            text.chars().count() as isize - layout.text[bytes.clone()].chars().count() as isize;
        let mut others = self.view.layout_others.borrow_mut();
        others.retain(|c| {
            c.line != layout.line && c.revision == layout.revision && c.tab == layout.tab
        });
        for cached in others.iter_mut() {
            let cached = Arc::make_mut(cached);
            cached.revision = self.revision();
            if cached.line > layout.line {
                cached.start = cached.start.saturating_add_signed(scalar_delta);
            }
        }
        // Complete indexes survive small printable-ASCII edits between ASCII
        // neighbors. Before the first tab, cell shifts become a multiple of
        // tab width after that tab; later stops preserve the same alignment.
        // Other edits keep conservative invalidation below.
        let printable = |b: &u8| matches!(b, 0x20..=0x7e);
        let ascii_edit = bytes.len().max(text.len()) <= 4096
            && text.as_bytes().iter().all(printable)
            && layout.text.as_bytes()[bytes.clone()].iter().all(printable);
        let first_tab = layout.first_tab;
        let retain_index = layout.scanned.byte == layout.text.len()
            && first_tab.is_none_or(|(at, _)| at >= chars + bytes.len())
            && ascii_edit
            && (bytes.start == 0
                || layout
                    .text
                    .as_bytes()
                    .get(bytes.start - 1)
                    .is_some_and(printable))
            && layout.text.as_bytes().get(bytes.end).is_none_or(printable);
        let old_end = bytes.end;
        let edit_start = bytes.start;
        let delta = text.len() as isize - bytes.len() as isize;
        // Prefix storage reuse is independent of index retention: a tabbed
        // suffix still needs fresh cell checkpoints after an edit, but its
        // bytes do not need to move for an eligible ASCII prefix deletion.
        layout.text.replace(bytes, text, ascii_edit);
        if retain_index {
            let tab = layout.tab.max(1);
            let after_tab_delta = first_tab.map_or(delta, |(_, before)| {
                let shifted = before.saturating_add_signed(delta);
                let old_after = before + tab - before % tab;
                let new_after = shifted + tab - shifted % tab;
                new_after as isize - old_after as isize
            });
            layout
                .checkpoints
                .retain(|p| p.byte <= edit_start || p.byte >= old_end);
            let shift = |p: &mut Checkpoint| {
                if p.byte >= old_end && p.byte > edit_start {
                    let cell_delta = if first_tab.is_some_and(|(at, _)| p.chars > at) {
                        after_tab_delta
                    } else {
                        delta
                    };
                    p.byte = p.byte.saturating_add_signed(delta);
                    p.chars = p.chars.saturating_add_signed(delta);
                    p.cells = p.cells.saturating_add_signed(cell_delta);
                    p.graphemes = p.graphemes.saturating_add_signed(delta);
                }
            };
            for point in &mut layout.checkpoints {
                shift(point);
            }
            shift(&mut layout.scanned);
            layout.first_tab = first_tab.map(|(at, before)| {
                (
                    at.saturating_add_signed(delta),
                    before.saturating_add_signed(delta),
                )
            });
            layout.revision = self.revision();
            return;
        }
        // Resume before the edit so joining accents/ZWJ/RI clusters are rebuilt.
        let retain = layout
            .checkpoints
            .partition_point(|point| point.chars < chars)
            .max(1);
        layout.checkpoints.truncate(retain);
        layout.scanned = layout.checkpoints.last().unwrap().clone();
        if layout
            .first_tab
            .is_some_and(|(at, _)| at >= layout.scanned.chars)
        {
            layout.first_tab = None;
        }
        layout.revision = self.revision();
    }
    pub(crate) fn indexed_line(
        &self,
        line: usize,
        tab: usize,
        through_char: usize,
        through_cell: usize,
    ) -> Option<Arc<LineLayout<'static>>> {
        let mut cache = self.view.layout_cache.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|c| c.line != line || c.revision != self.revision() || c.tab != tab)
        {
            let mut others = self.view.layout_others.borrow_mut();
            if let Some(index) = others
                .iter()
                .position(|c| c.line == line && c.revision == self.revision() && c.tab == tab)
            {
                let found = others.remove(index).expect("cached index");
                if let Some(old) = cache.replace(found) {
                    others.push_back(old);
                }
            } else {
                let range = self.line_bounds(line).ok()?;
                if range.len() < 8192 {
                    return None;
                }
                // Do not add a full ASCII text copy for oversized opt-in lines.
                // Their direct path remains available outside the 10 MiB corpus.
                if range.len() > 16 * 1024 * 1024 {
                    let bytes = self.buffer().char_to_byte(CharIndex(range.end)).ok()?.0
                        - self.buffer().char_to_byte(CharIndex(range.start)).ok()?.0;
                    if bytes == range.len() {
                        return None;
                    }
                }
                if let Some(old) = cache.take() {
                    others.push_back(old);
                }
                *cache = Some(Arc::new(LineLayout {
                    line,
                    revision: self.revision(),
                    tab,
                    start: range.start,
                    text: Cow::<str>::Owned(self.buffer().slice(range).ok()?).into(),
                    checkpoints: vec![Checkpoint {
                        chars: 0,
                        byte: 0,
                        cells: 0,
                        graphemes: 0,
                    }],
                    first_tab: None,
                    scanned: Checkpoint {
                        chars: 0,
                        byte: 0,
                        cells: 0,
                        graphemes: 0,
                    },
                }));
            }
            while others.len() > 63
                || others.iter().map(|c| c.text.storage.len()).sum::<usize>()
                    + cache.as_ref().map_or(0, |c| c.text.storage.len())
                    > 16 * 1024 * 1024
            {
                if others.pop_front().is_none() {
                    break;
                }
            }
        }
        let layout = cache.as_mut().unwrap();
        if (layout.scanned.chars < through_char || layout.scanned.cells < through_cell)
            && layout.scanned.byte < layout.text.len()
        {
            let layout = Arc::make_mut(layout);
            if layout.text.len() >= 1024 * 1024
                && (through_char.saturating_sub(layout.scanned.chars) > 128 * 1024
                    || through_cell.saturating_sub(layout.scanned.cells) > 128 * 1024)
            {
                layout.parallel_index();
            } else {
                layout.scan_to(through_char, through_cell);
            }
        }
        Some(layout.clone())
    }
    pub fn line_bounds(&self, line: usize) -> Result<std::ops::Range<usize>, EditError> {
        let start = self.buffer().line_to_char(line)?;
        let mut end = if line + 1 < self.buffer().len_lines() {
            self.buffer().line_to_char(line + 1)?
        } else {
            self.buffer().len_chars()
        };
        let tail = self.buffer().slice(end.saturating_sub(2).max(start)..end)?;
        if tail.ends_with("\r\n") {
            end -= 2;
        } else if tail.ends_with('\n') {
            end -= 1;
        }
        Ok(start..end)
    }

    pub fn cursor_cell(&self, tab_width: usize) -> usize {
        let line = self
            .buffer()
            .char_to_line(self.cursor())
            .expect("cursor line");
        let start = self.buffer().line_to_char(line).expect("line");
        if let Some(layout) = self.indexed_line(line, tab_width, self.cursor() - start, 0) {
            return layout.position(self.cursor()).1;
        }
        let end = self.cursor();
        let bytes = self.buffer().char_to_byte(CharIndex(end)).unwrap().0
            - self.buffer().char_to_byte(CharIndex(start)).unwrap().0;
        if bytes == end - start {
            let mut cell = 0;
            self.buffer()
                .visit_chunks(start..end, &mut |chunk| {
                    // Printable ASCII runs need no grapheme allocation.
                    for byte in chunk.bytes() {
                        cell += if byte == b'\t' {
                            tab_width.max(1) - cell % tab_width.max(1)
                        } else {
                            1
                        };
                    }
                    true
                })
                .unwrap();
            cell
        } else {
            cell_column(
                &self.buffer().slice(start..end).unwrap(),
                CharIndex(end - start),
                tab_width,
            )
            .unwrap()
            .0
        }
    }

    pub fn line_window(
        &self,
        line: usize,
        left: usize,
        width: usize,
        tab_width: usize,
    ) -> Result<LineWindow, EditError> {
        if let Some(layout) = self.indexed_line(line, tab_width, 0, left.saturating_add(width)) {
            return Ok(layout.window(left, width));
        }
        let range = self.line_bounds(line)?;
        let bytes = self.buffer().char_to_byte(CharIndex(range.end))?.0
            - self.buffer().char_to_byte(CharIndex(range.start))?.0;
        if bytes == range.len() {
            let mut cell = 0;
            let mut index = range.start;
            let mut start = range.start;
            let mut cell_start = 0;
            let mut text = String::with_capacity(width + 1);
            self.buffer().visit_chunks(range, &mut |chunk| {
                for byte in chunk.bytes() {
                    if cell >= left.saturating_add(width) {
                        return false;
                    }
                    let cells = if byte == b'\t' {
                        tab_width.max(1) - cell % tab_width.max(1)
                    } else {
                        1
                    };
                    if cell + cells > left {
                        if text.is_empty() {
                            start = index;
                            cell_start = cell;
                        }
                        text.push(byte as char);
                    }
                    cell += cells;
                    index += 1;
                }
                true
            })?;
            if text.is_empty() {
                start = index;
                cell_start = cell;
            }
            return Ok(LineWindow {
                text,
                char_start: start,
                cell_start,
            });
        }
        // Walk only as far as the requested viewport, including full clusters.
        let mut index = range.start;
        let mut cell = 0;
        while index < range.end && cell < left {
            let next = crate::graphemes::next(self.buffer(), index)?;
            let text = self.buffer().slice(index..next)?;
            let cells = display_grapheme(&text, cell, tab_width).1;
            if cell + cells > left {
                break;
            }
            cell += cells;
            index = next;
        }
        let start = index;
        let cell_start = cell;
        while index < range.end && cell < left.saturating_add(width) {
            let next = crate::graphemes::next(self.buffer(), index)?;
            cell += display_grapheme(&self.buffer().slice(index..next)?, cell, tab_width).1;
            index = next;
        }
        Ok(LineWindow {
            text: self.buffer().slice(start..index)?,
            char_start: start,
            cell_start,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_ascii_line_does_not_allocate_a_full_layout_copy() {
        let document = Document::from_text(&"a".repeat(17 * 1024 * 1024));
        let window = document.line_window(0, 0, 40, 4).unwrap();
        assert_eq!(window.text, "a".repeat(40));
        assert_eq!(document.view_cache_bytes(), 0);
    }

    #[test]
    fn prefix_ascii_edits_preserve_global_tab_stops_without_reindexing() {
        let original = format!("abc\t{}", "界 e\u{301} 👩‍💻 ab\t".repeat(1000));
        for tab in [3, 4, 8] {
            let mut document = Document::from_text(&original);
            document.prepare_view(tab);
            for count in [1, 5, 8] {
                document.select(0, 0).unwrap();
                document.insert(&"x".repeat(count)).unwrap();
                for history in 0..3 {
                    if history == 1 {
                        document.undo().unwrap();
                    }
                    if history == 2 {
                        document.redo().unwrap();
                    }
                    assert!(
                        document
                            .view
                            .layout_cache
                            .borrow()
                            .as_ref()
                            .is_some_and(|c| c.scanned.byte == c.text.len()),
                        "index must stay complete"
                    );
                    let text = document.text();
                    document.move_cursor(crate::Movement::Finish, false);
                    assert_eq!(document.position().1, text.graphemes(true).count());
                    let expected = cell_column(&text, CharIndex(text.chars().count()), tab)
                        .unwrap()
                        .0;
                    assert_eq!(document.cursor_cell(tab), expected);
                    for left in [0, 2, 512, expected.saturating_sub(40)] {
                        let window = document.line_window(0, left, 40, tab).unwrap();
                        let prefix = document.buffer().slice(0..window.char_start).unwrap();
                        assert_eq!(
                            window.cell_start,
                            cell_column(&prefix, CharIndex(prefix.chars().count()), tab)
                                .unwrap()
                                .0
                        );
                        assert!(document.is_grapheme_boundary(window.char_start));
                    }
                }
                document.select(0, count).unwrap();
                document.insert("").unwrap();
                assert_eq!(document.text(), original);
                assert!(
                    document
                        .view
                        .layout_cache
                        .borrow()
                        .as_ref()
                        .is_some_and(|c| c.scanned.byte == c.text.len())
                );
                document.move_cursor(crate::Movement::Finish, false);
                assert_eq!(
                    document.cursor_cell(tab),
                    cell_column(&original, CharIndex(original.chars().count()), tab)
                        .unwrap()
                        .0
                );
            }
        }
    }

    #[test]
    fn long_ascii_cache_preserves_combining_edits_at_checkpoints() {
        let mut document = Document::from_text(&"a".repeat(20_000));
        document.prepare_view(4);
        assert!(document.view.layout_cache.borrow().is_some());
        document.select(512, 512).unwrap();
        document.insert("\u{301}").unwrap();
        assert_eq!(document.position(), (0, 512));
        assert_eq!(document.cursor_cell(4), 512);
        let window = document.line_window(0, 512, 10, 4).unwrap();
        assert_eq!(window.char_start, 513);
        assert!(document.is_grapheme_boundary(window.char_start));
        document.undo().unwrap();
        assert_eq!(document.text(), "a".repeat(20_000));
        document.redo().unwrap();
        assert_eq!(document.position(), (0, 512));

        let text = format!("{}\u{301}{}", "a".repeat(983), "b".repeat(20_000));
        let mut joined = Document::from_text(&text);
        joined.prepare_view(4);
        joined.move_cursor(crate::Movement::Finish, false);
        assert_eq!(joined.position().1, 20_983);
        assert_eq!(joined.cursor_cell(4), 20_983);
    }

    #[test]
    fn ascii_tab_cache_reuses_prefix_storage_and_matches_full_unicode_layout() {
        let original = format!("{}\r\nsecond\n", "a\tbc\r".repeat(4000));
        let mut document = Document::from_text(&original);
        document.prepare_view(4);
        document.insert("xyz").unwrap();
        let snapshot = document.review_snapshot();
        document.select(0, 3).unwrap();
        document.insert("").unwrap();
        assert!(
            document
                .view
                .layout_cache
                .borrow()
                .as_ref()
                .unwrap()
                .text
                .head
                > 0
        );
        assert_eq!(document.text(), original);
        assert_eq!(snapshot.text(), format!("xyz{original}"));
        for inserted in ["abc", "\t", "e\u{301}", "👩‍💻", "\r\n"] {
            document.select(512, 512).unwrap();
            document.insert(inserted).unwrap();
            for history in 0..3 {
                if history == 1 {
                    document.undo().unwrap();
                }
                if history == 2 {
                    document.redo().unwrap();
                }
                let text = document
                    .buffer()
                    .slice(document.line_bounds(0).unwrap())
                    .unwrap();
                document.move_cursor(crate::Movement::Start, false);
                document.move_cursor(crate::Movement::End, false);
                assert_eq!(document.position().1, text.graphemes(true).count());
                for tab in [3, 4, 8] {
                    let expected = cell_column(&text, CharIndex(text.chars().count()), tab)
                        .unwrap()
                        .0;
                    assert_eq!(document.cursor_cell(tab), expected);
                    let window = document
                        .line_window(0, expected.saturating_sub(80), 80, tab)
                        .unwrap();
                    assert!(document.is_grapheme_boundary(window.char_start));
                    assert!(
                        document
                            .is_grapheme_boundary(window.char_start + window.text.chars().count())
                    );
                }
            }
            document.undo().unwrap();
            assert_eq!(document.text(), original);
        }
    }

    #[test]
    fn prefix_cache_reuse_preserves_snapshots_unicode_fallback_and_coordinates() {
        let original = format!("marker {}", "e\u{301} 界 👩‍💻 🇻🇳 ".repeat(1500));
        let mut d = Document::from_text(&original);
        d.prepare_view(4);
        for _ in 0..20 {
            d.move_cursor(crate::Movement::Start, false);
            d.insert("abc").unwrap();
            let snapshot = d.review_snapshot();
            d.select(0, 3).unwrap();
            d.insert("").unwrap();
            assert!(d.view.layout_cache.borrow().as_ref().unwrap().text.head > 0);
            assert_eq!(d.text(), original);
            let fresh_snapshot = Document::from_text(&format!("abc{original}"));
            assert_eq!(
                snapshot.line_window(0, 0, 20, 4).unwrap().text,
                fresh_snapshot.line_window(0, 0, 20, 4).unwrap().text
            );
            for action in 0..3 {
                if action == 1 {
                    d.undo().unwrap();
                }
                if action == 2 {
                    d.redo().unwrap();
                }
                let fresh = Document::from_text(&d.text());
                for left in [0, 100, 10_003] {
                    let actual = d.line_window(0, left, 30, 4).unwrap();
                    let expected = fresh.line_window(0, left, 30, 4).unwrap();
                    assert_eq!(
                        (actual.text, actual.char_start, actual.cell_start),
                        (expected.text, expected.char_start, expected.cell_start)
                    );
                }
                d.move_cursor(crate::Movement::Finish, false);
                assert_eq!(d.position().1, d.text().graphemes(true).count());
            }
        }
        d.move_cursor(crate::Movement::Start, false);
        d.insert("e\u{301}\t").unwrap();
        let fresh = Document::from_text(&d.text());
        assert_eq!(
            d.line_window(0, 0, 30, 4).unwrap().text,
            fresh.line_window(0, 0, 30, 4).unwrap().text
        );
        d.undo().unwrap();
        assert_eq!(d.text(), original);
        let charged = d.view_cache_bytes();
        assert!(charged >= original.len());
    }

    #[test]
    fn multiline_cache_matches_fresh_layout_after_edits_history_and_tab_changes() {
        let text = (0..90)
            .map(|i| format!("line {i}\t{}tail\n", "Việt e\u{301} 👩‍💻 ".repeat(1000)))
            .collect::<String>();
        let mut document = Document::from_text(&text);
        document.prepare_view(4);
        assert!(!document.view.layout_others.borrow().is_empty());
        assert!(document.view.layout_others.borrow().len() <= 63);
        for edit_line in [0, 40, 89, 2] {
            let start = document.buffer().line_to_char(edit_line).unwrap();
            document.select(start, start).unwrap();
            // Last painted line differs from the edited line.
            document.line_window(70, 0, 40, 8).unwrap();
            document.insert("abc e\u{301}").unwrap();
            for action in 0..3 {
                if action == 1 {
                    document.undo().unwrap();
                }
                if action == 2 {
                    document.redo().unwrap();
                }
                let fresh = Document::from_text(&document.text());
                for tab in [4, 8, 3, 4] {
                    for line in [0, 2, 40, 70, 89] {
                        let actual = document.line_window(line, 15, 20, tab).unwrap();
                        let expected = fresh.line_window(line, 15, 20, tab).unwrap();
                        assert_eq!(
                            (actual.char_start, actual.cell_start, actual.text),
                            (expected.char_start, expected.cell_start, expected.text)
                        );
                    }
                }
            }
        }
        document.clear_view_cache();
        assert_eq!(document.view_cache_bytes(), 0);
    }

    #[test]
    fn short_line_edit_keeps_prepared_long_line_with_shifted_coordinates() {
        let mut document =
            Document::from_text(&format!("prefix\n{}", "Việt e\u{301} 👩‍💻 ".repeat(2000)));
        document.prepare_view(4);
        let bytes = document.view_cache_bytes();
        assert!(bytes > 0);
        document.insert("edited ").unwrap();
        assert_eq!(document.view_cache_bytes(), bytes);
        let actual = document.line_window(1, 17000, 30, 4).unwrap();
        let fresh = Document::from_text(&document.text());
        let expected = fresh.line_window(1, 17000, 30, 4).unwrap();
        assert_eq!(
            (actual.text, actual.char_start, actual.cell_start),
            (expected.text, expected.char_start, expected.cell_start)
        );
    }

    #[test]
    fn prepared_indexes_survive_ascii_edits_and_rebuild_for_unicode_and_tabs() {
        for unit in ["ascii Việt e\u{301} 👩‍💻 ", "ascii\tViệt e\u{301} 👩‍💻 "] {
            let mut d = Document::from_text(&unit.repeat(50000));
            d.prepare_view(4);
            assert!(
                d.view
                    .layout_cache
                    .borrow()
                    .as_ref()
                    .is_some_and(|l| l.scanned.byte == l.text.len())
            );
            for text in ["x", "longer", "e\u{301}"] {
                d.move_cursor(crate::Movement::Start, false);
                d.insert(text).unwrap();
                if !unit.contains('\t') && text.is_ascii() {
                    assert!(
                        d.view
                            .layout_cache
                            .borrow()
                            .as_ref()
                            .is_some_and(|l| l.scanned.byte == l.text.len()),
                        "ASCII edit lost its prepared index"
                    );
                }
                let full = d.text();
                d.move_cursor(crate::Movement::Finish, false);
                assert_eq!(d.position().1, full.graphemes(true).count());
                assert_eq!(
                    d.cursor_cell(4),
                    cell_column(&full, CharIndex(full.chars().count()), 4)
                        .unwrap()
                        .0
                );
                d.undo().unwrap();
                d.move_cursor(crate::Movement::Finish, false);
                let full = d.text();
                assert_eq!(
                    d.cursor_cell(4),
                    cell_column(&full, CharIndex(full.chars().count()), 4)
                        .unwrap()
                        .0
                );
            }
            // ASCII removal in the middle must shift later checkpoints safely.
            d.select(2, 4).unwrap();
            d.insert("z").unwrap();
            d.move_cursor(crate::Movement::Finish, false);
            let full = d.text();
            assert_eq!(d.position().1, full.graphemes(true).count());
            assert_eq!(
                d.cursor_cell(4),
                cell_column(&full, CharIndex(full.chars().count()), 4)
                    .unwrap()
                    .0
            );
        }
    }
    #[test]
    fn partial_ascii_scan_preserves_joining_accents_crlf_and_tabs() {
        for text in [
            "abcde\u{301}\r\n\t界",
            "long ascii\rbare\t👩‍💻 tail",
            "ae\u{301}x",
        ] {
            let zero = Checkpoint {
                chars: 0,
                byte: 0,
                cells: 0,
                graphemes: 0,
            };
            let mut layout = LineLayout {
                line: 0,
                revision: 0,
                tab: 3,
                start: 0,
                text: Cow::Borrowed(text).into(),
                checkpoints: vec![zero.clone()],
                scanned: zero,
                first_tab: None,
            };
            let mut chars = 0;
            for (column, g) in text.graphemes(true).enumerate() {
                chars += g.chars().count();
                layout.scan_to(chars, 0);
                assert_eq!(
                    layout.position(chars),
                    (
                        column + 1,
                        cell_column(text, CharIndex(chars), 3).unwrap().0
                    )
                );
            }
        }
    }
    #[test]
    fn parallel_index_preserves_global_tab_stops_and_cluster_boundaries() {
        let text = "e\u{301}\t界 🇻🇳 👩‍💻 A".repeat(60_000);
        let mut d = Document::from_text(&text);
        d.move_cursor(crate::Movement::Finish, false);
        assert_eq!(d.position().1, text.graphemes(true).count());
        for tab in [3, 4, 8] {
            let expected = cell_column(&text, CharIndex(text.chars().count()), tab)
                .unwrap()
                .0;
            assert_eq!(d.cursor_cell(tab), expected);
            for left in [100_003, 300_007, expected - 70] {
                let window = d.line_window(0, left, 50, tab).unwrap();
                assert!(d.is_grapheme_boundary(window.char_start));
                assert_eq!(
                    cell_column(&text, CharIndex(window.char_start), tab)
                        .unwrap()
                        .0,
                    window.cell_start
                );
            }
        }
    }
    #[test]
    fn windows_keep_absolute_tabs_and_clusters() {
        let d = Document::from_text("a\t界👩‍💻e\u{301}\r\nnext");
        let window = d.line_window(0, 3, 5, 4).unwrap();
        assert_eq!(window.text, "\t界👩‍💻");
        assert_eq!((window.char_start, window.cell_start), (1, 1));
        assert_eq!(window.text.graphemes(true).count(), 3);
        assert_eq!(d.line_bounds(0).unwrap(), 0..8);
        let d = Document::from_text(&"x".repeat(10 * 1024 * 1024));
        let window = d.line_window(0, 9 * 1024 * 1024, 100, 4).unwrap();
        assert_eq!(window.text.len(), 100);
    }

    #[test]
    fn sparse_index_matches_full_unicode_layout_after_edits_and_undo() {
        let text = "Việt e\u{301}\t界👩‍💻🇻🇳 ".repeat(1200);
        let mut d = Document::from_text(&text);
        let mut chars = 0;
        for (index, g) in text.graphemes(true).enumerate() {
            chars += g.chars().count();
            if index % 257 != 0 {
                continue;
            }
            d.select(chars, chars).unwrap();
            for tab in [4, 8] {
                let expected = cell_column(&text, CharIndex(chars), tab).unwrap().0;
                assert_eq!(d.cursor_cell(tab), expected);
                assert_eq!(d.position().1, index + 1);
                let window = d
                    .line_window(0, expected.saturating_sub(7), 40, tab)
                    .unwrap();
                assert!(d.is_grapheme_boundary(window.char_start));
                assert_eq!(
                    cell_column(&text, CharIndex(window.char_start), tab)
                        .unwrap()
                        .0,
                    window.cell_start
                );
            }
        }
        d.move_cursor(crate::Movement::Finish, false);
        d.insert("🐹").unwrap();
        assert_eq!(d.position().1, text.graphemes(true).count() + 1);
        d.undo().unwrap();
        assert_eq!(d.position().1, text.graphemes(true).count());
    }

    #[test]
    fn incremental_layout_rebuilds_joins_and_history_at_checkpoints() {
        let mut d = Document::from_text(&"a e\u{301} 界👩‍💻🇻🇳\t".repeat(800));
        for index in [0, 511, 1024, 4097, 7000] {
            d.move_cursor(crate::Movement::Start, false);
            for _ in 0..index {
                d.move_cursor(crate::Movement::Right, false);
            }
            for inserted in ["\u{301}", "🇺", "\u{600}", "\t", "\r\n"] {
                let _ = d.cursor_cell(4);
                d.insert(inserted).unwrap();
                for history in 0..3 {
                    if history == 1 {
                        d.undo().unwrap();
                    }
                    if history == 2 {
                        d.redo().unwrap();
                    }
                    let text = d.text();
                    let (line, col) = d.position();
                    let start = d.buffer().line_to_char(line).unwrap();
                    let prefix = d.buffer().slice(start..d.cursor()).unwrap();
                    assert_eq!(col, prefix.graphemes(true).count());
                    assert_eq!(
                        d.cursor_cell(4),
                        cell_column(&prefix, CharIndex(prefix.chars().count()), 4)
                            .unwrap()
                            .0
                    );
                    assert!(d.is_grapheme_boundary(d.cursor()), "{text}");
                }
            }
        }
    }
}
