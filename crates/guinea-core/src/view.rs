use crate::{
    Document, EditError, TextBuffer,
    coordinates::{CharIndex, cell_column, display_grapheme},
};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
struct Checkpoint {
    chars: usize,
    byte: usize,
    cells: usize,
    graphemes: usize,
}

/// Sparse display index for long Unicode lines. Printable ASCII takes the
/// allocation-free path below. The cached text and index are shared by clones.
#[derive(Debug, Clone)]
pub(crate) struct LineLayout {
    line: usize,
    revision: u64,
    tab: usize,
    start: usize,
    text: String,
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

impl LineLayout {
    fn scan_to(&mut self, through_char: usize, through_cell: usize) {
        while self.scanned.byte < self.text.len()
            && (self.scanned.chars < through_char || self.scanned.cells < through_cell)
        {
            let point = &mut self.scanned;
            let tail = &self.text[point.byte..];
            let ascii = tail.bytes().take_while(u8::is_ascii).count();
            // The last ASCII scalar can combine with the following Unicode
            // scalar (e + combining accent); leave it to segmentation.
            let ascii = if ascii < tail.len() {
                ascii.saturating_sub(1)
            } else {
                ascii
            };
            if ascii > 0 && !tail[..ascii].contains('\r') {
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
        let segments = std::thread::scope(|scope| {
            let jobs = boundaries
                .windows(2)
                .map(|range| {
                    let text = &self.text[range[0]..range[1]];
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
                            text: text.to_owned(),
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
        for segment in segments {
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

impl Document {
    pub(crate) fn prepare_layout_edit(
        &self,
        range: &std::ops::Range<usize>,
        text: &str,
    ) -> Option<(std::ops::Range<usize>, usize)> {
        if text.contains(['\r', '\n']) {
            return None;
        }
        let cache = self.layout_cache.borrow();
        let layout = cache.as_ref()?;
        if layout.revision != self.revision()
            || range.start < layout.start
            || range.end > self.line_bounds(layout.line).ok()?.end
        {
            return None;
        }
        let origin = self.buffer().char_to_byte(CharIndex(layout.start)).ok()?.0;
        Some((
            self.buffer().char_to_byte(CharIndex(range.start)).ok()?.0 - origin
                ..self.buffer().char_to_byte(CharIndex(range.end)).ok()?.0 - origin,
            range.start - layout.start,
        ))
    }
    pub(crate) fn apply_layout_edit(
        &self,
        edit: Option<(std::ops::Range<usize>, usize)>,
        text: &str,
    ) {
        let mut cache = self.layout_cache.borrow_mut();
        let Some((bytes, chars)) = edit else {
            *cache = None;
            return;
        };
        let Some(layout) = cache.as_mut() else {
            return;
        };
        let layout = Arc::make_mut(layout);
        layout.text.replace_range(bytes, text);
        // Resume before the edit so joining accents/ZWJ/RI clusters are rebuilt.
        let retain = layout
            .checkpoints
            .partition_point(|point| point.chars < chars)
            .max(1);
        layout.checkpoints.truncate(retain);
        layout.scanned = layout.checkpoints.last().unwrap().clone();
        layout.revision = self.revision();
    }
    pub(crate) fn indexed_line(
        &self,
        line: usize,
        tab: usize,
        through_char: usize,
        through_cell: usize,
    ) -> Option<Arc<LineLayout>> {
        let mut cache = self.layout_cache.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|c| c.line != line || c.revision != self.revision() || c.tab != tab)
        {
            let range = self.line_bounds(line).ok()?;
            if range.len() < 8192 {
                return None;
            }
            let bytes = self.buffer().char_to_byte(CharIndex(range.end)).ok()?.0
                - self.buffer().char_to_byte(CharIndex(range.start)).ok()?.0;
            if bytes == range.len() {
                return None;
            }
            *cache = Some(Arc::new(LineLayout {
                line,
                revision: self.revision(),
                tab,
                start: range.start,
                text: self.buffer().slice(range).ok()?,
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
