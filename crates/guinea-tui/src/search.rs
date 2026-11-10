//! Bounded, cancellable search/replace work. UI handlers only edit small fields
//! and select cached scalar ranges; scanning and replacement stay on workers.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use guinea_core::{Document, TextBuffer};
use ratatui::layout::Rect;
use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

pub const PAGE: usize = 200;
static WORKERS: AtomicUsize = AtomicUsize::new(0);
struct Worker {
    _permit: crate::work::Permit,
}
impl Drop for Worker {
    fn drop(&mut self) {
        WORKERS.fetch_sub(1, Ordering::Release);
    }
}

pub struct Found {
    pub range: Range<usize>,
    pub line: usize,
    pub preview: String,
}
struct Scan {
    entries: Vec<Found>,
    total: usize,
}
enum Reply {
    Scan(Scan),
    Edit(Box<Document>, usize),
}
struct Pending {
    receiver: Receiver<Result<Reply, String>>,
    cancel: Arc<AtomicBool>,
    replace: bool,
    revision: u64,
}
pub struct Change {
    pub document: Document,
    pub revision: u64,
    pub count: usize,
}
#[derive(Clone, Copy)]
pub enum Command {
    None,
    Close,
    Next,
    Previous,
    Go,
    Replace,
    ReplaceAll,
}

pub struct Search {
    pub query: String,
    pub replacement: String,
    pub replace_mode: bool,
    pub focus: usize,
    pub entries: Vec<Found>,
    pub total: usize,
    pub offset: usize,
    pub selected: usize,
    pub error: Option<String>,
    pub replaced: Option<usize>,
    pub list: Rect,
    pub height: usize,
    caret: [usize; 2],
    selected_field: [bool; 2],
    document: u64,
    revision: u64,
    dirty: bool,
    changed: Instant,
    pending: Option<Pending>,
    select_last: bool,
    advance: bool,
    resume: Option<usize>,
}
impl Drop for Search {
    fn drop(&mut self) {
        if let Some(p) = &self.pending {
            p.cancel.store(true, Ordering::Release);
        }
    }
}
impl Search {
    pub fn new(document: &Document, query: String, replacement: String, replace: bool) -> Self {
        let caret = [query.len(), replacement.len()];
        Self {
            query,
            replacement,
            replace_mode: replace,
            focus: 0,
            entries: vec![],
            total: 0,
            offset: 0,
            selected: 0,
            error: None,
            replaced: None,
            list: Rect::default(),
            height: 5,
            caret,
            selected_field: [true, true],
            document: document.id,
            revision: document.revision(),
            dirty: true,
            changed: Instant::now() - Duration::from_millis(40),
            pending: None,
            select_last: false,
            advance: false,
            resume: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.dirty || self.pending.is_some()
    }
    pub fn replacing(&self) -> bool {
        self.pending.as_ref().is_some_and(|p| p.replace)
    }
    pub fn ready(&self) -> bool {
        !self.busy() && self.error.is_none() && !self.entries.is_empty()
    }
    pub fn current(&self) -> Option<Range<usize>> {
        self.ready()
            .then(|| self.entries[self.selected].range.clone())
    }
    pub fn top(&self) -> usize {
        self.selected
            .saturating_sub(self.height / 2)
            .min(self.entries.len().saturating_sub(self.height))
    }
    fn changed(&mut self) {
        if let Some(p) = &self.pending {
            p.cancel.store(true, Ordering::Release);
        }
        self.dirty = true;
        self.changed = Instant::now();
        self.offset = 0;
        self.selected = 0;
        self.error = None;
        self.replaced = None;
        self.resume = None;
        self.select_last = false;
    }
    pub fn paste(&mut self, text: &str) {
        if self.focus > 1 || self.replacing() {
            return;
        }
        let text = text.replace(['\r', '\n', '\0'], "");
        let field = if self.focus == 0 {
            &mut self.query
        } else {
            &mut self.replacement
        };
        let selected = self.selected_field[self.focus];
        if (if selected { 0 } else { field.len() }).saturating_add(text.len()) > 4096 {
            self.error = Some("Input is limited to 4096 UTF-8 bytes".into());
            return;
        }
        if selected {
            field.clear();
            self.caret[self.focus] = 0;
            self.selected_field[self.focus] = false;
        }
        field.insert_str(self.caret[self.focus], &text);
        self.caret[self.focus] += text.len();
        self.error = None;
        // Keep the field caret on a complete extended grapheme after joins.
        self.caret[self.focus] = field
            .grapheme_indices(true)
            .map(|(b, _)| b)
            .find(|&b| b >= self.caret[self.focus])
            .unwrap_or(field.len());
        if self.focus == 0 {
            self.changed();
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> Command {
        if key.code == KeyCode::Esc {
            return Command::Close;
        }
        if self.replacing() {
            return Command::None;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Char('h') if ctrl => {
                self.replace_mode = true;
                self.focus = 1;
            }
            KeyCode::Char('f') if ctrl => self.focus = 0,
            KeyCode::Char('a') if ctrl && self.focus < 2 => self.selected_field[self.focus] = true,
            KeyCode::Char('r') if ctrl && shift => return Command::ReplaceAll,
            KeyCode::Char('r') if ctrl => return Command::Replace,
            KeyCode::Enter if self.focus == 2 => return Command::Go,
            KeyCode::Enter | KeyCode::F(3) => {
                return if shift {
                    Command::Previous
                } else {
                    Command::Next
                };
            }
            KeyCode::Up => return Command::Previous,
            KeyCode::Down => return Command::Next,
            KeyCode::Tab | KeyCode::BackTab => {
                let fields = if self.replace_mode {
                    &[0, 1, 2][..]
                } else {
                    &[0, 2][..]
                };
                let at = fields.iter().position(|&f| f == self.focus).unwrap_or(0);
                self.focus = fields[(at
                    + if key.code == KeyCode::BackTab {
                        fields.len() - 1
                    } else {
                        1
                    })
                    % fields.len()];
                if self.focus < 2 {
                    self.selected_field[self.focus] = true;
                }
            }
            KeyCode::Char('u') if ctrl && self.focus < 2 => {
                if self.focus == 0 {
                    self.query.clear();
                    self.changed();
                } else {
                    self.replacement.clear();
                }
                self.caret[self.focus] = 0;
                self.selected_field[self.focus] = false;
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Backspace
            | KeyCode::Delete
                if self.focus < 2 =>
            {
                let i = self.focus;
                let field = if i == 0 {
                    &mut self.query
                } else {
                    &mut self.replacement
                };
                if self.selected_field[i]
                    && matches!(key.code, KeyCode::Backspace | KeyCode::Delete)
                {
                    field.clear();
                    self.caret[i] = 0;
                    self.selected_field[i] = false;
                    if i == 0 {
                        self.changed();
                    }
                    return Command::None;
                }
                self.selected_field[i] = false;
                let at = self.caret[i].min(field.len());
                let prev = field[..at]
                    .grapheme_indices(true)
                    .next_back()
                    .map_or(0, |(b, _)| b);
                let next = at + field[at..].graphemes(true).next().map_or(0, str::len);
                match key.code {
                    KeyCode::Left => self.caret[i] = prev,
                    KeyCode::Right => self.caret[i] = next,
                    KeyCode::Home => self.caret[i] = 0,
                    KeyCode::End => self.caret[i] = field.len(),
                    KeyCode::Backspace => {
                        field.replace_range(prev..at, "");
                        self.caret[i] = prev;
                    }
                    KeyCode::Delete => {
                        field.replace_range(at..next, "");
                        self.caret[i] = at;
                    }
                    _ => {}
                }
                if i == 0 && matches!(key.code, KeyCode::Backspace | KeyCode::Delete) {
                    self.changed();
                }
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.paste(&ch.to_string())
            }
            _ => {}
        }
        Command::None
    }
    pub fn field(&self, index: usize, width: usize) -> (String, usize) {
        let value = if index == 0 {
            &self.query
        } else {
            &self.replacement
        };
        let at = self.caret[index].min(value.len());
        let left = crate::workspace_render::input_tail(&value[..at], width.saturating_sub(1));
        let cell = ratatui::text::Line::from(left.as_str()).width();
        let right = crate::workspace_render::clipped(&value[at..], width.saturating_sub(cell + 1));
        (format!("{left}│{right}"), cell)
    }
    pub fn field_selected(&self, index: usize) -> bool {
        self.selected_field[index]
    }
    pub fn navigate(&mut self, previous: bool) {
        if !self.ready() {
            return;
        }
        if self.total <= PAGE {
            self.selected = (self.selected + if previous { self.entries.len() - 1 } else { 1 })
                % self.entries.len();
            return;
        }
        if previous && self.selected == 0 {
            self.offset = if self.offset == 0 {
                (self.total.saturating_sub(1) / PAGE) * PAGE
            } else {
                self.offset.saturating_sub(PAGE)
            };
            self.select_last = true;
            self.dirty = true;
        } else if !previous && self.selected + 1 >= self.entries.len() {
            self.offset = if self.offset + self.entries.len() >= self.total {
                0
            } else {
                self.offset + PAGE
            };
            self.select_last = false;
            self.dirty = true;
        } else {
            self.selected = self
                .selected
                .saturating_add_signed(if previous { -1 } else { 1 });
        }
        self.changed = Instant::now() - Duration::from_millis(40);
    }
    pub fn replace(&mut self, document: &Document, all: bool) {
        if !self.replace_mode || !self.ready() {
            return;
        }
        if document.readonly {
            self.error = Some("Document is read-only".into());
            return;
        }
        if document.id != self.document || document.revision() != self.revision {
            self.changed();
            return;
        }
        let count = if all { self.total } else { 1 };
        let estimate = document
            .buffer()
            .len_bytes()
            .checked_sub(self.query.len().saturating_mul(count))
            .and_then(|n| n.checked_add(self.replacement.len().saturating_mul(count)));
        if estimate.is_none_or(|n| n > 128 * 1024 * 1024) {
            self.error = Some(
                "Result exceeds 128 MiB; use a shorter replacement or replace one match".into(),
            );
            return;
        }
        let Some(permit) = crate::work::Permit::acquire() else {
            return;
        };
        if WORKERS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 2).then_some(n + 1)
            })
            .is_err()
        {
            return;
        }
        let guard = Worker { _permit: permit };
        let mut snapshot = document.review_snapshot();
        snapshot.readonly = false;
        let query = self.query.clone();
        let replacement = self.replacement.clone();
        let range = self.entries[self.selected].range.clone();
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        self.pending = Some(Pending {
            receiver: rx,
            cancel,
            replace: true,
            revision: document.revision(),
        });
        self.advance = !all;
        std::thread::spawn(move || {
            let _guard = guard;
            let result = if all {
                snapshot
                    .replace_all(&query, &replacement)
                    .map(|n| Reply::Edit(Box::new(snapshot), n))
            } else {
                snapshot
                    .replace_range(range, &replacement)
                    .map(|()| Reply::Edit(Box::new(snapshot), 1))
            };
            if !worker_cancel.load(Ordering::Acquire) {
                let _ = tx.send(result.map_err(|e| e.to_string()));
            }
        });
    }
    pub fn poll(&mut self, document: &Document) -> (bool, Option<Change>) {
        let mut changed = false;
        if let Some(pending) = &self.pending {
            match pending.receiver.try_recv() {
                Ok(reply) => {
                    let replace = pending.replace;
                    let revision = pending.revision;
                    self.pending = None;
                    changed = true;
                    if replace {
                        match reply {
                            Ok(Reply::Edit(prepared, count)) => {
                                return (
                                    true,
                                    Some(Change {
                                        document: *prepared,
                                        revision,
                                        count,
                                    }),
                                );
                            }
                            Err(e) => self.error = Some(e),
                            _ => {}
                        }
                    } else if !self.dirty {
                        match reply {
                            Ok(Reply::Scan(scan)) => {
                                self.entries = scan.entries;
                                self.total = scan.total;
                                self.selected = if self.select_last {
                                    self.entries.len().saturating_sub(1)
                                } else {
                                    0
                                };
                                if let Some(resume) = self.resume {
                                    if let Some(index) =
                                        self.entries.iter().position(|m| m.range.start >= resume)
                                    {
                                        self.selected = index;
                                        self.resume = None;
                                    } else if self.total > 0
                                        && self.offset + self.entries.len() < self.total
                                    {
                                        self.offset += PAGE;
                                        self.dirty = true;
                                    } else {
                                        self.resume = None;
                                        if self.offset > 0 {
                                            self.offset = 0;
                                            self.dirty = true;
                                        }
                                        self.selected = 0;
                                    }
                                }
                            }
                            Err(e) => self.error = Some(e),
                            _ => {}
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    changed = true;
                    if !self.dirty {
                        self.entries.clear();
                        self.total = 0;
                        self.error = Some("Search worker stopped; edit the query to retry".into());
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if document.id != self.document {
            self.error = Some("Document is no longer active".into());
            self.dirty = false;
            return (true, None);
        }
        if document.revision() != self.revision {
            self.revision = document.revision();
            self.changed();
        }
        if self.dirty
            && self.pending.is_none()
            && self.changed.elapsed() >= Duration::from_millis(35)
        {
            if self.query.is_empty() {
                self.entries.clear();
                self.total = 0;
                self.dirty = false;
                return (true, None);
            }
            let Some(permit) = crate::work::Permit::acquire() else {
                return (changed, None);
            };
            if WORKERS
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    (n < 2).then_some(n + 1)
                })
                .is_ok()
            {
                let guard = Worker { _permit: permit };
                let snapshot = document.worker_snapshot();
                let query = self.query.clone();
                let offset = self.offset;
                let (tx, rx) = mpsc::channel();
                let cancel = Arc::new(AtomicBool::new(false));
                let worker_cancel = cancel.clone();
                self.pending = Some(Pending {
                    receiver: rx,
                    cancel,
                    replace: false,
                    revision: document.revision(),
                });
                self.dirty = false;
                changed = true;
                std::thread::spawn(move || {
                    let _guard = guard;
                    let result =
                        scan(&snapshot.text(), &query, offset, &worker_cancel).map(Reply::Scan);
                    let _ = tx.send(result);
                });
            }
        }
        (changed, None)
    }
    pub fn applied(&mut self, document: &Document, count: usize) {
        self.revision = document.revision();
        let offset = self.offset;
        self.changed();
        // An explicit replacement already completed. Refresh its results now;
        // the typing debounce only coalesces query edits.
        self.changed = Instant::now() - Duration::from_millis(40);
        if self.advance {
            self.offset = offset;
            self.resume = Some(document.cursor());
        }
        self.replaced = Some(count);
    }
}

fn scan(text: &str, query: &str, offset: usize, cancel: &AtomicBool) -> Result<Scan, String> {
    let mut entries = Vec::new();
    let mut total = 0;
    let mut scalar = 0;
    let mut previous = 0;
    let mut line = 0;
    let length = query.chars().count();
    for (byte, matched) in text.match_indices(query) {
        if cancel.load(Ordering::Acquire) {
            return Err("Search cancelled".into());
        }
        if !GraphemeCursor::new(byte, text.len(), true)
            .is_boundary(text, 0)
            .unwrap_or(false)
            || !GraphemeCursor::new(byte + matched.len(), text.len(), true)
                .is_boundary(text, 0)
                .unwrap_or(false)
        {
            continue;
        }
        scalar += text[previous..byte].chars().count();
        line += text[previous..byte].bytes().filter(|&b| b == b'\n').count();
        previous = byte;
        if total >= offset && entries.len() < PAGE {
            let before = text[..byte]
                .chars()
                .rev()
                .take(24)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>();
            let after = text[byte + matched.len()..]
                .chars()
                .take(32)
                .collect::<String>();
            let preview = format!(
                "{}[{}]{}",
                before.replace(['\r', '\n', '\t'], " "),
                matched.chars().take(32).collect::<String>(),
                after.replace(['\r', '\n', '\t'], " ")
            );
            entries.push(Found {
                range: scalar..scalar + length,
                line,
                preview,
            });
        }
        total += 1;
    }
    Ok(Scan { entries, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settle(search: &mut Search, document: &Document) -> Option<Change> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let (_, change) = search.poll(document);
            if change.is_some() || !search.busy() {
                return change;
            }
            assert!(Instant::now() < deadline, "Search did not complete");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn unicode_matches_are_paged_and_never_split_clusters() {
        let cancelled = AtomicBool::new(false);
        assert_eq!(scan("e\u{301} 👩‍💻", "e", 0, &cancelled).unwrap().total, 0);
        assert_eq!(scan("e\u{301} 👩‍💻", "💻", 0, &cancelled).unwrap().total, 0);
        let text = format!("# title\r\n{}", "Việt ".repeat(PAGE + 3));
        let page = scan(&text, "Việt", PAGE, &cancelled).unwrap();
        assert_eq!(page.total, PAGE + 3);
        assert_eq!(page.entries.len(), 3);
        let document = Document::from_text(&text);
        for found in page.entries {
            assert!(document.is_grapheme_boundary(found.range.start));
            assert!(document.is_grapheme_boundary(found.range.end));
            assert_eq!(found.line, 1);
        }
        assert!(scan(&text, "Việt", 0, &AtomicBool::new(true)).is_err());
    }
    #[test]
    fn replacement_is_prepared_off_thread_and_commits_as_one_undo() {
        let original = "Việt 👩‍💻\r\nViệt e\u{301}";
        let mut document = Document::from_text(original);
        let mut search = Search::new(&document, "Việt".into(), "Vietnam".into(), true);
        settle(&mut search, &document);
        assert_eq!(search.total, 2);
        search.replace(&document, true);
        assert_eq!(document.text(), original);
        let changed = settle(&mut search, &document).unwrap();
        assert_eq!(changed.count, 2);
        document
            .accept_prepared(&changed.document, changed.revision)
            .unwrap();
        assert_eq!(document.text(), "Vietnam 👩‍💻\r\nVietnam e\u{301}");
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), original);
        document.readonly = true;
        search.applied(&document, 0);
        settle(&mut search, &document);
        search.replace(&document, true);
        assert!(
            search
                .error
                .as_deref()
                .is_some_and(|s| s.contains("read-only"))
        );
    }
    #[test]
    fn replace_one_advances_past_newly_inserted_matches() {
        let mut document = Document::from_text("foo foo");
        let mut search = Search::new(&document, "foo".into(), "foobar".into(), true);
        settle(&mut search, &document);
        search.replace(&document, false);
        let change = settle(&mut search, &document).unwrap();
        document
            .accept_prepared(&change.document, change.revision)
            .unwrap();
        search.applied(&document, 1);
        assert!(search.changed.elapsed() >= Duration::from_millis(35));
        settle(&mut search, &document);
        assert_eq!(document.text(), "foobar foo");
        assert_eq!(search.current().unwrap(), 7..10);
    }
    #[test]
    fn replacement_receipt_keeps_its_original_revision() {
        let mut document = Document::from_text("foo foo");
        let mut search = Search::new(&document, "foo".into(), "bar".into(), true);
        let mut prepared = document.review_snapshot();
        prepared.readonly = false;
        prepared.replace_all("foo", "bar").unwrap();
        document.insert("user ").unwrap();
        document.insert("later ").unwrap();
        let original = document.text();
        search.revision = document.revision();
        let (tx, receiver) = mpsc::channel();
        search.pending = Some(Pending {
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
            replace: true,
            revision: 0,
        });
        tx.send(Ok(Reply::Edit(Box::new(prepared), 2)))
            .ok()
            .unwrap();
        let (_, change) = search.poll(&document);
        let change = change.unwrap();
        assert_eq!(change.revision, 0);
        assert!(
            document
                .accept_prepared(&change.document, change.revision)
                .is_err()
        );
        assert_eq!(document.text(), original);
    }
    #[test]
    fn changed_queries_discard_stale_results_and_inputs_edit_graphemes() {
        let document = Document::from_text("old new new 👩‍💻");
        let mut search = Search::new(&document, "old".into(), String::new(), false);
        search.poll(&document);
        search.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        search.paste("new");
        settle(&mut search, &document);
        assert_eq!(search.total, 2);
        assert!(search.entries.iter().all(|m| m.preview.contains("[new]")));
        search.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        search.paste("e\u{301}👩‍💻");
        search.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        search.key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(search.query, "👩‍💻");
    }
}
