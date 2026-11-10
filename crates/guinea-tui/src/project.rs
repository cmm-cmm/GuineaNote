//! Bounded project discovery and literal disk search; no filesystem work on input.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use guinea_core::TextBuffer;
use ratatui::layout::Rect;
use std::{
    ops::Range,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

const MAX_FILES: usize = 20_000;
const MAX_INDEX_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESULTS: usize = 200;
const MAX_FILE: u64 = 10 * 1024 * 1024;
const MAX_SCAN: u64 = 256 * 1024 * 1024;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Files,
    Contents,
}
#[derive(Clone)]
pub struct Found {
    pub path: PathBuf,
    pub label: String,
    pub line: usize,
    pub range: Option<Range<usize>>,
    pub hash: Option<[u8; 32]>,
    pub preview: String,
}
struct File {
    path: PathBuf,
    label: String,
    folded: String,
}
struct Index {
    files: Vec<File>,
    limited: bool,
    skipped: usize,
}
struct Report {
    epoch: u64,
    index: Option<Arc<Index>>,
    rows: Vec<Found>,
    matches: usize,
    scanned: usize,
    skipped: usize,
    limited: bool,
    error: Option<String>,
}
struct Pending {
    receiver: Receiver<Report>,
    cancel: Arc<AtomicBool>,
}
pub enum Action {
    None,
    Close,
    Open,
}
pub struct Project {
    pub root: PathBuf,
    pub mode: Mode,
    pub query: String,
    pub rows: Vec<Found>,
    pub selected: usize,
    pub height: usize,
    pub list: Rect,
    pub scanned: usize,
    pub skipped: usize,
    pub matches: usize,
    pub limited: bool,
    pub error: Option<String>,
    epoch: u64,
    dirty: bool,
    changed: Instant,
    pending: Option<Pending>,
    index: Option<Arc<Index>>,
}
impl Drop for Project {
    fn drop(&mut self) {
        if let Some(p) = &self.pending {
            p.cancel.store(true, Ordering::Release);
        }
    }
}
impl Project {
    pub fn new(root: PathBuf, mode: Mode) -> Self {
        Self {
            root,
            mode,
            query: String::new(),
            rows: Vec::new(),
            selected: 0,
            height: 10,
            list: Rect::default(),
            scanned: 0,
            skipped: 0,
            matches: 0,
            limited: false,
            error: None,
            epoch: 0,
            dirty: true,
            changed: Instant::now() - Duration::from_secs(1),
            pending: None,
            index: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.dirty
    }
    pub fn ready(&self) -> bool {
        !self.busy() && self.error.is_none()
    }
    fn invalidate(&mut self) {
        if let Some(p) = &self.pending {
            p.cancel.store(true, Ordering::Release);
        }
        self.epoch += 1;
        self.dirty = true;
        self.changed = Instant::now();
        self.rows.clear();
        self.selected = 0;
        self.matches = 0;
        self.error = None;
    }
    pub fn paste(&mut self, text: &str) {
        for ch in text.chars().filter(|c| !c.is_control()) {
            if self.query.len() + ch.len_utf8() > 1024 {
                break;
            }
            self.query.push(ch);
        }
        self.invalidate();
    }
    pub fn toggle(&mut self) {
        self.mode = if self.mode == Mode::Files {
            Mode::Contents
        } else {
            Mode::Files
        };
        self.invalidate();
    }
    pub fn move_by(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(1));
    }
    pub fn top(&self) -> usize {
        self.selected.saturating_sub(self.height.saturating_sub(1))
    }
    pub fn key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Action::Close,
            KeyCode::Enter if self.ready() && !self.rows.is_empty() => return Action::Open,
            KeyCode::Tab | KeyCode::BackTab => self.toggle(),
            KeyCode::F(5) => {
                self.index = None;
                self.invalidate();
            }
            KeyCode::Up => self.move_by(-1),
            KeyCode::Down => self.move_by(1),
            KeyCode::PageUp => self.move_by(-(self.height as isize)),
            KeyCode::PageDown => self.move_by(self.height as isize),
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.rows.len().saturating_sub(1),
            KeyCode::Char('u') if ctrl => {
                self.query.clear();
                self.invalidate();
            }
            KeyCode::Backspace => {
                if let Some((byte, _)) = self.query.grapheme_indices(true).next_back() {
                    self.query.truncate(byte);
                    self.invalidate();
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
        Action::None
    }
    pub fn poll(&mut self) -> bool {
        let mut redraw = false;
        if let Some(pending) = &self.pending {
            match pending.receiver.try_recv() {
                Ok(report) => {
                    self.pending = None;
                    // A completed index is independent of the filter query.
                    if report.index.is_some() && report.epoch == self.epoch {
                        self.index = report.index;
                    }
                    if report.epoch == self.epoch {
                        self.rows = report.rows;
                        self.matches = report.matches;
                        self.scanned = report.scanned;
                        self.skipped = report.skipped;
                        self.limited = report.limited;
                        self.error = report.error;
                        redraw = true;
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    if !self.dirty {
                        self.error = Some("Project worker stopped".into());
                    }
                    redraw = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.dirty
            && self.pending.is_none()
            && self.changed.elapsed() >= Duration::from_millis(40)
        {
            let Some(permit) = crate::work::Permit::acquire() else {
                return redraw;
            };
            let root = self.root.clone();
            let query = self.query.clone();
            let mode = self.mode;
            let epoch = self.epoch;
            let index = self.index.clone();
            let cancel = Arc::new(AtomicBool::new(false));
            let stop = cancel.clone();
            let (tx, receiver) = mpsc::sync_channel(0);
            self.pending = Some(Pending { receiver, cancel });
            self.dirty = false;
            std::thread::spawn(move || {
                let _permit = permit;
                let report = scan(root, query, mode, epoch, index, &stop);
                if !stop.load(Ordering::Acquire) {
                    let _ = tx.send(report);
                }
            });
        }
        redraw
    }
}
fn scan(
    root: PathBuf,
    query: String,
    mode: Mode,
    epoch: u64,
    cached: Option<Arc<Index>>,
    stop: &AtomicBool,
) -> Report {
    let mut report = Report {
        epoch,
        index: None,
        rows: Vec::new(),
        matches: 0,
        scanned: 0,
        skipped: 0,
        limited: false,
        error: None,
    };
    let index = match cached {
        Some(index) => index,
        None => match build_index(&root, stop) {
            Ok(index) => Arc::new(index),
            Err(error) => {
                report.error = Some(error);
                return report;
            }
        },
    };
    report.limited = index.limited;
    report.skipped = index.skipped;
    report.index = Some(index.clone());
    let folded = query.to_lowercase();
    let words = folded.split_whitespace().collect::<Vec<_>>();
    let mut bytes = 0u64;
    if mode == Mode::Contents && query.is_empty() {
        return report;
    }
    for file in &index.files {
        if stop.load(Ordering::Acquire) {
            return report;
        }
        if mode == Mode::Files {
            if words.iter().all(|word| file.folded.contains(word)) {
                report.matches += 1;
                if report.rows.len() < MAX_RESULTS {
                    report.rows.push(Found {
                        path: file.path.clone(),
                        label: file.label.clone(),
                        line: 0,
                        range: None,
                        hash: None,
                        preview: String::new(),
                    });
                } else {
                    report.limited = true;
                }
            }
            continue;
        }
        let size = match std::fs::symlink_metadata(&file.path) {
            Ok(m) if !guinea_service::policy::special(&m) && m.is_file() && m.len() <= MAX_FILE => {
                m.len()
            }
            _ => {
                report.skipped += 1;
                continue;
            }
        };
        if bytes.saturating_add(size) > MAX_SCAN {
            report.limited = true;
            break;
        }
        bytes += size;
        let document = match guinea_io::open(&file.path, true, MAX_FILE) {
            Ok(d) => d,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        report.scanned += 1;
        if stop.load(Ordering::Acquire) {
            return report;
        }
        let text = document.text();
        let mut previous_byte = 0;
        let mut previous_scalar = 0;
        for (byte, _) in text.match_indices(&query) {
            if stop.load(Ordering::Acquire) {
                return report;
            }
            let start = previous_scalar + text[previous_byte..byte].chars().count();
            let end = start + query.chars().count();
            previous_byte = byte;
            previous_scalar = start;
            if !document.is_grapheme_boundary(start) || !document.is_grapheme_boundary(end) {
                continue;
            }
            let line = document.buffer().char_to_line(start).unwrap_or(0);
            let origin = document.buffer().line_to_char(line).unwrap_or(start);
            // Preview is bounded even for a single enormous source line.
            let preview_start = start.saturating_sub(32).max(origin);
            let line_end = document.line_bounds(line).map_or(end, |r| r.end);
            let preview = document
                .buffer()
                .slice(preview_start..(end + 80).min(line_end))
                .unwrap_or_default()
                .replace('\t', "  ");
            report.rows.push(Found {
                path: file.path.clone(),
                label: file.label.clone(),
                line,
                range: Some(start..end),
                hash: document.fingerprint.as_ref().map(|f| f.sha256),
                preview,
            });
            report.matches += 1;
            if report.rows.len() == MAX_RESULTS {
                report.limited = true;
                return report;
            }
        }
    }
    report
}
fn build_index(root: &std::path::Path, stop: &AtomicBool) -> Result<Index, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if !root.is_dir() {
        return Err("Project root must be a directory".into());
    }
    let mut builder = ignore::WalkBuilder::new(&root);
    builder
        .hidden(false)
        .follow_links(false)
        .require_git(false)
        .git_global(false)
        .parents(false)
        .max_depth(Some(32))
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if entry.file_type().is_some_and(|t| t.is_dir())
                && matches!(
                    entry.file_name().to_str(),
                    Some(".git" | "target" | "node_modules" | ".venv" | "__pycache__")
                )
            {
                return false;
            }
            entry
                .metadata()
                .is_ok_and(|m| !guinea_service::policy::special(&m))
        });
    let mut index = Index {
        files: Vec::new(),
        limited: false,
        skipped: 0,
    };
    let mut bytes = 0usize;
    for (visited, item) in builder.build().enumerate() {
        if stop.load(Ordering::Acquire) {
            return Err("Project scan cancelled".into());
        }
        if visited >= 40_000 {
            index.limited = true;
            break;
        }
        let entry = match item {
            Ok(e) => e,
            Err(_) => {
                index.skipped += 1;
                continue;
            }
        };
        if entry.depth() == 0 || !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.into_path();
        let label = path
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if label.len() > 4096 {
            index.skipped += 1;
            continue;
        }
        let folded = label.to_lowercase();
        bytes +=
            path.as_os_str().len() * 2 + label.len() + folded.len() + std::mem::size_of::<File>();
        if bytes > MAX_INDEX_BYTES || index.files.len() >= MAX_FILES {
            index.limited = true;
            break;
        }
        index.files.push(File {
            path,
            label,
            folded,
        });
    }
    index.files.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_ignores_generated_files_and_preserves_unicode_match_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::create_dir(dir.path().join("target")).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(dir.path().join("ignored.txt"), "needle").unwrap();
        std::fs::write(dir.path().join("target/build.txt"), "needle").unwrap();
        std::fs::write(
            dir.path().join("src/Việt.rs"),
            "e\u{301} 👩‍💻\r\nneedle needle",
        )
        .unwrap();
        std::fs::write(dir.path().join("binary"), b"needle\0").unwrap();
        let stop = AtomicBool::new(false);
        let result = scan(
            dir.path().into(),
            "src việt".into(),
            Mode::Files,
            0,
            None,
            &stop,
        );
        assert_eq!(result.rows.len(), 1);
        let result = scan(
            dir.path().into(),
            "needle".into(),
            Mode::Contents,
            1,
            result.index,
            &stop,
        );
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.scanned, 2); // .gitignore and source; binary refused.
        let mut source = guinea_io::open(&result.rows[0].path, false, MAX_FILE).unwrap();
        let range = result.rows[0].range.clone().unwrap();
        source.select(range.start, range.end).unwrap();
        assert_eq!(source.selected_text(), "needle");
        assert_eq!(result.rows[0].line, 1);
        assert!(
            scan(
                dir.path().into(),
                "💻".into(),
                Mode::Contents,
                2,
                result.index,
                &stop
            )
            .rows
            .is_empty()
        );
    }
    #[test]
    fn results_and_query_are_bounded_and_cancellation_is_observed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("many.txt"), "needle ".repeat(500)).unwrap();
        let stop = AtomicBool::new(false);
        let result = scan(
            dir.path().into(),
            "needle".into(),
            Mode::Contents,
            0,
            None,
            &stop,
        );
        assert_eq!(result.rows.len(), MAX_RESULTS);
        assert!(result.limited);
        let mut browser = Project::new(dir.path().into(), Mode::Files);
        browser.paste(&"👩‍💻\n".repeat(1000));
        assert!(browser.query.len() <= 1024 && !browser.query.contains('\n'));
        stop.store(true, Ordering::Release);
        assert!(
            scan(
                dir.path().into(),
                "needle".into(),
                Mode::Contents,
                1,
                result.index,
                &stop
            )
            .rows
            .is_empty()
        );
    }
}
