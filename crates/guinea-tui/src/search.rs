//! Bounded, cancellable search/replace work. UI handlers only edit small fields
//! and select cached scalar ranges; scanning and replacement stay on workers.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use guinea_core::{Document, SearchOptions, TextBuffer};
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
use unicode_segmentation::UnicodeSegmentation;

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
    /// Byte range of the matched text inside `preview` (between the brackets).
    pub highlight: Range<usize>,
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
    /// Alt+C toggles case-insensitive, Alt+W whole-word and Alt+R regex
    /// matching.
    pub options: SearchOptions,
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
    pub fn new(document: &Document, query: String, replacement: String) -> Self {
        let caret = [query.len(), replacement.len()];
        Self {
            query,
            replacement,
            options: SearchOptions::default(),
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
    pub fn with_options(mut self, options: SearchOptions) -> Self {
        self.options = options;
        self
    }
    /// Toggle an option and rescan; ignored while a replacement is running.
    pub fn toggle(&mut self, whole_word: bool) {
        if self.replacing() {
            return;
        }
        if whole_word {
            self.options.whole_word = !self.options.whole_word;
        } else {
            self.options.ignore_case = !self.options.ignore_case;
        }
        self.changed();
    }
    /// Toggle regular-expression mode (Notepad++ "Regular expression").
    pub fn toggle_regex(&mut self) {
        if self.replacing() {
            return;
        }
        self.options.regex = !self.options.regex;
        self.changed();
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
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char('h') if ctrl => {
                self.focus = 1;
                self.selected_field[1] = true;
            }
            KeyCode::Char('f') if ctrl => {
                self.focus = 0;
                self.selected_field[0] = true;
            }
            KeyCode::Char('a') if ctrl && self.focus < 2 => self.selected_field[self.focus] = true,
            KeyCode::Char('r') if ctrl && shift => return Command::ReplaceAll,
            KeyCode::Char('r') if ctrl => return Command::Replace,
            KeyCode::Char('a' | 'A') if alt => return Command::ReplaceAll,
            KeyCode::Char('c' | 'C') if alt && !ctrl => self.toggle(false),
            KeyCode::Char('w' | 'W') if alt && !ctrl => self.toggle(true),
            KeyCode::Char('r' | 'R') if alt && !ctrl => self.toggle_regex(),
            KeyCode::Enter if self.focus == 2 => return Command::Go,
            KeyCode::Enter | KeyCode::Char(' ') if self.focus >= 3 => {
                return match self.focus {
                    3 => Command::Previous,
                    4 => Command::Next,
                    5 => Command::Go,
                    6 => Command::Replace,
                    7 => Command::ReplaceAll,
                    _ => Command::None,
                };
            }
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
                let fields = &[0, 1, 2, 3, 4, 5, 6, 7];
                let at = fields.iter().position(|&f| f == self.focus).unwrap_or(0);
                self.focus = fields[(at
                    + if key.code == KeyCode::BackTab || shift {
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
        if !self.ready() {
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
        // A literal match is as long as the query; a regex match is not, so
        // only literal mode subtracts it (the core enforces the exact cap).
        let removed = if self.options.regex {
            0
        } else {
            self.query.len().saturating_mul(count)
        };
        let estimate = document
            .buffer()
            .len_bytes()
            .checked_sub(removed)
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
        let options = self.options;
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
                    .replace_all_with(&query, &replacement, options)
                    .map(|n| Reply::Edit(Box::new(snapshot), n))
            } else {
                snapshot
                    .replace_match(range, &query, &replacement, options)
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
            // A pattern that does not compile is shown, never scanned.
            if let Err(error) = guinea_core::search::validate(&self.query, self.options) {
                self.entries.clear();
                self.total = 0;
                self.dirty = false;
                self.error = Some(format!("Regex: {error}"));
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
                let options = self.options;
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
                    let result = scan(&snapshot.text(), &query, options, offset, &worker_cancel)
                        .map(Reply::Scan);
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

fn scan(
    text: &str,
    query: &str,
    options: SearchOptions,
    offset: usize,
    cancel: &AtomicBool,
) -> Result<Scan, String> {
    let mut entries = Vec::new();
    let mut total = 0;
    let mut scalar = 0;
    let mut previous = 0;
    let mut line = 0;
    for range in guinea_core::search::find_matches(text, query, options) {
        if cancel.load(Ordering::Acquire) {
            return Err("Search cancelled".into());
        }
        let byte = range.start;
        let matched = &text[range];
        let length = matched.chars().count();
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
            let before = before.replace(['\r', '\n', '\t'], " ");
            let shown = matched
                .chars()
                .take(32)
                .collect::<String>()
                .replace(['\r', '\n', '\t'], " ");
            let highlight = before.len() + 1..before.len() + 1 + shown.len();
            let preview = format!(
                "{before}[{shown}]{}",
                after.replace(['\r', '\n', '\t'], " ")
            );
            entries.push(Found {
                range: scalar..scalar + length,
                line,
                preview,
                highlight,
            });
        }
        total += 1;
    }
    Ok(Scan { entries, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn start_replacement(search: &mut Search, document: &Document, all: bool) {
        // Parallel review/search tests share the bounded worker permits. A
        // ready query does not guarantee a permit at this exact instant.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            search.replace(document, all);
            if search.replacing() {
                return;
            }
            assert!(search.ready(), "replacement rejected: {:?}", search.error);
            assert!(Instant::now() < deadline, "replacement admission timed out");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn unified_fields_buttons_and_portable_replace_all_are_keyboard_reachable() {
        let document = Document::from_text("Việt Việt");
        let mut search = Search::new(&document, "Việt".into(), "Vietnam".into());
        for focus in 1..8 {
            search.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
            assert_eq!(search.focus, focus);
        }
        assert!(matches!(
            search.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Command::ReplaceAll
        ));
        search.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(search.focus, 0);
        search.key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        assert_eq!(search.focus, 7);
        search.key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        search.paste("e\u{301}👩‍💻");
        assert_eq!(search.replacement, "e\u{301}👩‍💻");
        assert_eq!(search.query, "Việt");
        assert!(matches!(
            search.key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT)),
            Command::ReplaceAll
        ));
        assert_eq!(document.text(), "Việt Việt");
    }
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
        assert_eq!(
            scan("e\u{301} 👩‍💻", "e", SearchOptions::default(), 0, &cancelled)
                .unwrap()
                .total,
            0
        );
        assert_eq!(
            scan("e\u{301} 👩‍💻", "💻", SearchOptions::default(), 0, &cancelled)
                .unwrap()
                .total,
            0
        );
        let text = format!("# title\r\n{}", "Việt ".repeat(PAGE + 3));
        let page = scan(&text, "Việt", SearchOptions::default(), PAGE, &cancelled).unwrap();
        assert_eq!(page.total, PAGE + 3);
        assert_eq!(page.entries.len(), 3);
        let document = Document::from_text(&text);
        for found in page.entries {
            assert!(document.is_grapheme_boundary(found.range.start));
            assert!(document.is_grapheme_boundary(found.range.end));
            assert_eq!(found.line, 1);
        }
        assert!(
            scan(
                &text,
                "Việt",
                SearchOptions::default(),
                0,
                &AtomicBool::new(true)
            )
            .is_err()
        );
    }
    #[test]
    fn replacement_is_prepared_off_thread_and_commits_as_one_undo() {
        let original = "Việt 👩‍💻\r\nViệt e\u{301}";
        let mut document = Document::from_text(original);
        let mut search = Search::new(&document, "Việt".into(), "Vietnam".into());
        settle(&mut search, &document);
        assert_eq!(search.total, 2);
        start_replacement(&mut search, &document, true);
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
    fn case_and_whole_word_options_rescan_and_scope_replace_all() {
        let original = "Việt việt Việtnam VIỆT";
        let mut document = Document::from_text(original);
        let mut search = Search::new(&document, "việt".into(), "X".into());
        settle(&mut search, &document);
        assert_eq!(search.total, 1);
        let alt = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT);
        assert!(matches!(search.key(alt('c')), Command::None));
        assert!(search.options.ignore_case);
        settle(&mut search, &document);
        assert_eq!(search.total, 4);
        search.key(alt('w'));
        assert!(search.options.whole_word);
        settle(&mut search, &document);
        assert_eq!(search.total, 3);
        assert_eq!(search.entries[2].range, 18..22);
        start_replacement(&mut search, &document, true);
        let changed = settle(&mut search, &document).unwrap();
        assert_eq!(changed.count, 3);
        document
            .accept_prepared(&changed.document, changed.revision)
            .unwrap();
        assert_eq!(document.text(), "X X Việtnam X");
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), original);
    }
    #[test]
    fn regex_mode_reports_bad_patterns_and_replaces_with_groups() {
        // The pattern is longer than some documents it matches in: the size
        // estimate must not treat its length as the match length.
        let original = "x=1\r\ny=22\r\nz=x\r\n";
        let mut document = Document::from_text(original);
        let mut search = Search::new(
            &document,
            r"(\w)=(\d+)(?:)(?:)(?:)(?:)(?:)(?:)(?:)".into(),
            "$2:$1".into(),
        );
        settle(&mut search, &document);
        assert_eq!(search.total, 0, "literal mode: no such text");
        let alt = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT);
        search.key(alt('r'));
        assert!(search.options.regex);
        settle(&mut search, &document);
        assert_eq!(search.total, 2);
        assert_eq!(search.entries[1].line, 1);
        // Replace one expands the groups of the selected match only.
        search.selected = 1;
        start_replacement(&mut search, &document, false);
        let changed = settle(&mut search, &document).unwrap();
        document
            .accept_prepared(&changed.document, changed.revision)
            .unwrap();
        assert_eq!(document.text(), "x=1\r\n22:y\r\nz=x\r\n");
        search.applied(&document, changed.count);
        settle(&mut search, &document);
        start_replacement(&mut search, &document, true);
        let changed = settle(&mut search, &document).unwrap();
        document
            .accept_prepared(&changed.document, changed.revision)
            .unwrap();
        assert_eq!(document.text(), "1:x\r\n22:y\r\nz=x\r\n", "CRLF kept");
        // An invalid pattern is reported instead of scanned.
        search.query = "(open".into();
        search.changed();
        settle(&mut search, &document);
        assert_eq!(search.total, 0);
        assert!(
            search
                .error
                .as_deref()
                .is_some_and(|e| e.starts_with("Regex: "))
        );
        assert!(document.undo().unwrap() && document.undo().unwrap());
        assert_eq!(document.text(), original);
    }
    #[test]
    fn replace_one_advances_past_newly_inserted_matches() {
        let mut document = Document::from_text("foo foo");
        let mut search = Search::new(&document, "foo".into(), "foobar".into());
        settle(&mut search, &document);
        start_replacement(&mut search, &document, false);
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
        let mut search = Search::new(&document, "foo".into(), "bar".into());
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
        let mut search = Search::new(&document, "old".into(), String::new());
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
