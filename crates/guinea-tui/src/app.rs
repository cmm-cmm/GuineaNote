use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use guinea_core::{Document, Movement, TextBuffer};
use guinea_io::FileError;
use ratatui::layout::Rect;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::config::Config;
use crate::explorer::Explorer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Explorer,
}

#[derive(Default)]
pub struct HitRegions {
    pub tree: Rect,
    pub tabs: Vec<(Rect, usize)>,
    pub actions: Vec<(Rect, &'static str)>,
    pub editor: Rect,
    pub editor_text: Rect,
}

pub const HELP: &str = "GuineaNote — keyboard help\n\nWORKSPACE\nCtrl+K open folder  Ctrl+B show/hide explorer\nF4 switch pane     F5 refresh directory\nTree: ↑↓ select, → expand, ← collapse, Enter open\nTab / Esc returns from the tree to the editor\n\nEDITOR\nCtrl+N new    Ctrl+O open file/folder   Ctrl+S save\nCtrl+W close  Ctrl+Q quit    Ctrl+Shift+S save as\nCtrl+F find   F3 next match  Ctrl+H replace all\nCtrl+Z undo   Ctrl+Y redo    Ctrl+A select all\nCtrl+C/X/V copy/cut/paste (internal clipboard)\nArrows move, Shift selects, Ctrl+Home/End start/end\nF6 / Shift+F6 next / previous tab\n\nF2 COMMANDS\nnew, open, open-folder, save, save-as, close, quit,\nexplorer, focus, refresh, next, prev, find, find-next,\nreplace, undo, redo, select-all, copy, cut, paste,\nreload, help\n\nEnter submits · Esc cancels · Y confirms discard\nUTF-8 only · resident memory · no crash recovery\nF1 / Esc dismisses help";

pub struct Tab {
    pub document: Document,
    pub top: usize,
    pub left: usize,
}

impl Tab {
    fn new(document: Document) -> Self {
        Self {
            document,
            top: 0,
            left: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InputKind {
    Open,
    Folder,
    SaveAs,
    Find,
    ReplaceNeedle,
    ReplaceWith(String),
    Command,
}

impl InputKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Open => "Open file or folder",
            Self::Folder => "Open folder",
            Self::SaveAs => "Save to NEW path",
            Self::Find => "Find literal (case sensitive)",
            Self::ReplaceNeedle => "Replace all: find literal",
            Self::ReplaceWith(_) => "Replace all: replacement (Enter allows empty)",
            Self::Command => "Command (F1 lists commands)",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    Close,
    Quit,
    Reload,
    OpenLarge { path: PathBuf, replace: Option<u64> },
}

pub enum Modal {
    Input { kind: InputKind, value: String },
    Confirm { action: Action, message: String },
    Help,
}

#[derive(Debug, Clone)]
enum Job {
    Open { path: PathBuf, replace: Option<u64> },
    Save { id: u64 },
    Compute { id: u64 },
}

#[derive(Debug, thiserror::Error)]
enum WorkerError {
    #[error(transparent)]
    File(#[from] FileError),
    #[error(transparent)]
    Edit(#[from] guinea_core::EditError),
}

struct Completed {
    job: Job,
    result: Result<Document, WorkerError>,
    warning: Option<String>,
}

pub struct App {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub config: Config,
    pub modal: Option<Modal>,
    pub message: String,
    pub exit: bool,
    pub redraw: bool,
    pub page_height: usize,
    pub explorer: Explorer,
    pub explorer_visible: bool,
    pub focus: Focus,
    pub hits: HitRegions,
    readonly: bool,
    clipboard: String,
    search: String,
    pending: Option<Receiver<Completed>>,
    worker: Option<thread::JoinHandle<()>>,
    queued: VecDeque<PathBuf>,
}

impl App {
    pub fn new(files: Vec<PathBuf>, readonly: bool, config: Config) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let directory = files.iter().find(|path| path.is_dir()).cloned();
        let explorer_visible = directory.is_some() || files.is_empty();
        let root = directory
            .clone()
            .or_else(|| {
                files
                    .first()
                    .and_then(|p| p.parent())
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| cwd.clone());
        let root = if root.is_absolute() {
            root
        } else {
            cwd.join(root)
        };
        let files = files
            .into_iter()
            .filter(|path| !path.is_dir())
            .collect::<Vec<_>>();
        let mut document = Document::default();
        document.readonly = readonly;
        Self {
            tabs: vec![Tab::new(document)],
            active: 0,
            config,
            modal: None,
            message: "Ready — open a file or a folder to begin".into(),
            exit: false,
            redraw: true,
            page_height: 20,
            explorer: Explorer::new(root),
            explorer_visible,
            focus: if directory.is_some() && files.is_empty() {
                Focus::Explorer
            } else {
                Focus::Editor
            },
            hits: HitRegions::default(),
            readonly,
            clipboard: String::new(),
            search: String::new(),
            pending: None,
            worker: None,
            queued: files.into(),
        }
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn document(&self) -> &Document {
        &self.tabs[self.active].document
    }
    pub fn welcome(&self) -> bool {
        self.tabs.len() == 1
            && self.document().path.is_none()
            && self.document().buffer().len_chars() == 0
            && !self.document().is_dirty()
    }
    fn document_mut(&mut self) -> &mut Document {
        &mut self.tabs[self.active].document
    }

    fn prompt(&mut self, kind: InputKind) {
        self.modal = Some(Modal::Input {
            kind,
            value: String::new(),
        });
    }

    fn edit(&mut self, command: impl FnOnce(&mut Document) -> Result<(), guinea_core::EditError>) {
        if let Err(error) = command(self.document_mut()) {
            self.message = error.to_string();
        }
    }

    fn start_open(&mut self, path: PathBuf, replace: Option<u64>, limit: u64) {
        let readonly = self.readonly;
        let job = Job::Open {
            path: path.clone(),
            replace,
        };
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.message = format!("Opening {}… edits paused", path.display());
        self.worker = Some(thread::spawn(move || {
            let result = guinea_io::open(&path, readonly, limit).map_err(Into::into);
            let _ = sender.send(Completed {
                job,
                result,
                warning: None,
            });
        }));
    }

    fn start_save(&mut self, path: PathBuf) {
        if self.document().readonly {
            self.message = "Document is read-only".into();
            return;
        }
        let mut document = self.document().clone();
        let job = Job::Save { id: document.id };
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.message = format!("Saving {}… edits paused", path.display());
        self.worker = Some(thread::spawn(move || {
            let report = guinea_io::save(&mut document, &path);
            let (result, warning) = match report {
                Ok(report) => (Ok(document), report.warning),
                Err(error) => (Err(error.into()), None),
            };
            let _ = sender.send(Completed {
                job,
                result,
                warning,
            });
        }));
    }

    fn start_compute(
        &mut self,
        label: &str,
        task: impl FnOnce(&mut Document) -> Result<String, guinea_core::EditError> + Send + 'static,
    ) {
        let mut document = self.document().clone();
        let job = Job::Compute { id: document.id };
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.message = format!("{label}… edits paused");
        self.worker = Some(thread::spawn(move || {
            let (result, warning) = match task(&mut document) {
                Ok(message) => (Ok(document), Some(message)),
                Err(error) => (Err(error.into()), None),
            };
            let _ = sender.send(Completed {
                job,
                result,
                warning,
            });
        }));
    }

    fn join_worker(&mut self) {
        if let Some(worker) = self.worker.take()
            && let Err(panic) = worker.join()
        {
            std::panic::resume_unwind(panic);
        }
    }

    /// Called once per event-loop iteration, never blocking on pending work.
    pub fn poll_jobs(&mut self) {
        if (self.explorer_visible || self.explorer.scanning()) && self.explorer.poll() {
            self.redraw = true;
        }
        if let Some(receiver) = &self.pending {
            let result = receiver.try_recv();
            match result {
                Ok(completed) => {
                    self.pending = None;
                    self.join_worker();
                    self.redraw = true;
                    self.finish_job(completed);
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.join_worker();
                    self.redraw = true;
                    self.message =
                        "File worker failed; document retained. Retry or save to a new path."
                            .into();
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if !self.busy()
            && self.modal.is_none()
            && let Some(path) = self.queued.pop_front()
        {
            self.start_open(path, None, self.config.large_file_warning_bytes);
            self.redraw = true;
        }
    }

    fn finish_job(&mut self, completed: Completed) {
        match completed.result {
            Err(WorkerError::File(FileError::LargeFile { size, limit })) => {
                if let Job::Open { path, replace, .. } = completed.job {
                    self.modal = Some(Modal::Confirm {
                        action: Action::OpenLarge { path, replace },
                        message: format!(
                            "{size} bytes exceeds warning {limit}. Resident RAM may be much larger. Y open / N cancel"
                        ),
                    });
                }
            }
            Err(error) => {
                self.message = error.to_string();
            }
            Ok(mut document) => match completed.job {
                Job::Save { id } | Job::Compute { id } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.document.id == id) {
                        tab.document = document;
                    }
                    self.message = completed.warning.unwrap_or_else(|| "Saved".into());
                }
                Job::Open {
                    replace: Some(id), ..
                } => {
                    document.id = id;
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.document.id == id) {
                        *tab = Tab::new(document);
                    }
                    self.message = "Reloaded".into();
                    self.focus = Focus::Editor;
                }
                Job::Open { .. } => {
                    if let Some(idx) = self
                        .tabs
                        .iter()
                        .position(|t| t.document.path == document.path)
                    {
                        self.active = idx;
                        self.message = "Already open; switched to its tab".into();
                    } else if self.tabs.len() == 1
                        && self.document().path.is_none()
                        && self.document().text().is_empty()
                        && !self.document().is_dirty()
                    {
                        self.tabs[0] = Tab::new(document);
                        self.message = "Opened".into();
                    } else {
                        self.tabs.push(Tab::new(document));
                        self.active = self.tabs.len() - 1;
                        self.message = "Opened".into();
                    }
                    self.focus = Focus::Editor;
                    if let Some(path) = self.document().path.clone() {
                        self.explorer.select_path(&path);
                    }
                }
            },
        }
    }

    pub fn handle(&mut self, event: Event) {
        if matches!(event, Event::Resize(..)) {
            self.redraw = true;
            return;
        }
        if self.busy() {
            if matches!(event, Event::Key(_)) {
                self.message =
                    "Background operation in progress; edits paused until it completes".into();
                self.redraw = true;
            }
            return;
        }
        match event {
            Event::Paste(text) => {
                self.redraw = true;
                if let Some(Modal::Input { value, .. }) = self.modal.as_mut() {
                    value.push_str(&text.replace(['\r', '\n'], ""));
                } else if self.modal.is_none() && self.focus == Focus::Editor {
                    self.edit(|d| d.insert(&text));
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.redraw = true;
                if self.modal.is_some() {
                    self.modal_key(key);
                } else {
                    self.editor_key(key);
                }
            }
            Event::Mouse(mouse) if self.modal.is_none() && self.config.mouse => self.mouse(mouse),
            _ => {}
        }
    }

    fn modal_key(&mut self, key: KeyEvent) {
        let modal = self.modal.take().expect("modal exists");
        match modal {
            Modal::Help => {
                if !matches!(key.code, KeyCode::Esc | KeyCode::F(1)) {
                    self.modal = Some(Modal::Help);
                }
            }
            Modal::Confirm { action, message } => match key.code {
                KeyCode::Char('y' | 'Y') => self.confirm(action),
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                    self.message = "Cancelled".into();
                }
                _ => {
                    self.modal = Some(Modal::Confirm { action, message });
                }
            },
            Modal::Input { kind, mut value } => {
                match key.code {
                    KeyCode::Esc => {
                        self.message = "Cancelled".into();
                        return;
                    }
                    KeyCode::Enter => {
                        self.submit(kind, value);
                        return;
                    }
                    KeyCode::Backspace => {
                        if let Some((byte, _)) = value.grapheme_indices(true).next_back() {
                            value.truncate(byte);
                        }
                    }
                    KeyCode::Char(ch)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        value.push(ch)
                    }
                    _ => {}
                }
                self.modal = Some(Modal::Input { kind, value });
            }
        }
    }

    fn submit(&mut self, kind: InputKind, value: String) {
        match kind {
            InputKind::Open | InputKind::Folder
                if !value.is_empty() && PathBuf::from(&value).is_dir() =>
            {
                self.open_folder(PathBuf::from(value))
            }
            InputKind::Folder => self.message = "That path is not an accessible folder".into(),
            InputKind::Open if !value.is_empty() => self.start_open(
                PathBuf::from(value),
                None,
                self.config.large_file_warning_bytes,
            ),
            InputKind::SaveAs if !value.is_empty() => self.start_save(PathBuf::from(value)),
            InputKind::Find => {
                self.search = value;
                self.find_next();
            }
            InputKind::ReplaceNeedle if !value.is_empty() => {
                self.prompt(InputKind::ReplaceWith(value))
            }
            InputKind::ReplaceWith(needle) => {
                self.start_compute("Replacing", move |d| {
                    let count = d.replace_all(&needle, &value)?;
                    Ok(format!(
                        "Replaced {count} occurrence(s); Ctrl+Z undoes the transaction"
                    ))
                });
            }
            InputKind::Command => self.command(value.trim()),
            _ => {
                self.message = "Empty input; cancelled".into();
            }
        }
    }

    fn editor_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        if ctrl {
            match key.code {
                KeyCode::Char('s' | 'S') if shift => self.command("save-as"),
                KeyCode::Char('s') => self.command("save"),
                KeyCode::Char('o') => self.command("open"),
                KeyCode::Char('n') => self.command("new"),
                KeyCode::Char('b') => self.command("explorer"),
                KeyCode::Char('k') => self.command("open-folder"),
                KeyCode::Char('w') => self.command("close"),
                KeyCode::Char('q') => self.command("quit"),
                KeyCode::Char('f') => self.command("find"),
                KeyCode::Char('h') => self.command("replace"),
                KeyCode::Char('z') => self.command("undo"),
                KeyCode::Char('y') => self.command("redo"),
                KeyCode::Char('a') => self.command("select-all"),
                KeyCode::Char('c') => self.command("copy"),
                KeyCode::Char('x') => self.command("cut"),
                KeyCode::Char('v') => self.command("paste"),
                KeyCode::Tab | KeyCode::BackTab => {
                    self.command(if shift { "prev" } else { "next" })
                }
                KeyCode::Home => self.document_mut().move_cursor(Movement::Start, shift),
                KeyCode::End => self.document_mut().move_cursor(Movement::Finish, shift),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::F(4) => {
                self.command("focus");
                return;
            }
            KeyCode::F(5) => {
                self.command("refresh");
                return;
            }
            KeyCode::F(1) | KeyCode::F(2) | KeyCode::F(3) | KeyCode::F(6) => {}
            _ if self.focus == Focus::Explorer => {
                self.tree_key(key);
                return;
            }
            _ => {}
        }
        let movement = match key.code {
            KeyCode::Left => Some(Movement::Left),
            KeyCode::Right => Some(Movement::Right),
            KeyCode::Up => Some(Movement::Up),
            KeyCode::Down => Some(Movement::Down),
            KeyCode::Home => Some(Movement::Home),
            KeyCode::End => Some(Movement::End),
            _ => None,
        };
        if let Some(movement) = movement {
            self.document_mut().move_cursor(movement, shift);
            return;
        }
        match key.code {
            KeyCode::F(1) => self.command("help"),
            KeyCode::F(2) => self.prompt(InputKind::Command),
            KeyCode::F(3) => self.command("find-next"),
            KeyCode::F(6) => self.command(if shift { "prev" } else { "next" }),
            KeyCode::PageUp | KeyCode::PageDown => {
                let movement = if key.code == KeyCode::PageUp {
                    Movement::Up
                } else {
                    Movement::Down
                };
                for _ in 0..self.page_height.max(1) {
                    self.document_mut().move_cursor(movement, shift);
                }
            }
            KeyCode::Backspace => self.edit(Document::backspace),
            KeyCode::Delete => self.edit(Document::delete_forward),
            KeyCode::Enter => self.edit(Document::newline),
            KeyCode::Tab => self.edit(|d| d.insert("\t")),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::ALT) => {
                self.edit(|d| d.insert(&ch.to_string()))
            }
            _ => {}
        }
    }

    fn find_next(&mut self) {
        let needle = self.search.clone();
        self.start_compute("Finding", move |d| {
            Ok(if d.find(&needle) {
                "Match selected (F3 next)"
            } else {
                "No grapheme-aligned match"
            }
            .into())
        });
    }

    fn open_folder(&mut self, path: PathBuf) {
        self.explorer.open(path);
        self.explorer_visible = true;
        self.focus = Focus::Explorer;
        self.message = "Workspace opened — Enter opens a file · F4 switches panes".into();
    }

    fn tree_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.explorer.move_by(-1),
            KeyCode::Down => self.explorer.move_by(1),
            KeyCode::PageUp => self
                .explorer
                .move_by(-(self.explorer.viewport_height as isize)),
            KeyCode::PageDown => self
                .explorer
                .move_by(self.explorer.viewport_height as isize),
            KeyCode::Home => self.explorer.selected = 0,
            KeyCode::End => self.explorer.selected = self.explorer.rows.len().saturating_sub(1),
            KeyCode::Right => self.explorer.expand(),
            KeyCode::Left => self.explorer.collapse(),
            KeyCode::Enter => self.activate_tree(),
            KeyCode::Tab | KeyCode::Esc => self.focus = Focus::Editor,
            KeyCode::Char('r') => self.explorer.refresh(),
            _ => {}
        }
    }

    fn activate_tree(&mut self) {
        if let Some(row) = self.explorer.current().cloned() {
            if row.entry.directory {
                self.explorer.toggle();
            } else {
                self.start_open(row.entry.path, None, self.config.large_file_warning_bytes);
            }
        }
    }

    fn mouse(&mut self, mouse: MouseEvent) {
        let position = (mouse.column, mouse.row).into();
        if self.hits.tree.contains(position) {
            match mouse.kind {
                MouseEventKind::ScrollDown => self.explorer.move_by(3),
                MouseEventKind::ScrollUp => self.explorer.move_by(-3),
                MouseEventKind::Down(MouseButton::Left) => {
                    let index = self.explorer.top + (mouse.row - self.hits.tree.y) as usize;
                    if index < self.explorer.rows.len() {
                        self.explorer.selected = index;
                        self.focus = Focus::Explorer;
                        self.activate_tree();
                    }
                }
                _ => return,
            }
            self.redraw = true;
        } else if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some((_, index)) = self
                .hits
                .tabs
                .iter()
                .find(|(rect, _)| rect.contains(position))
            {
                self.active = *index;
                self.focus = Focus::Editor;
                self.redraw = true;
            } else if let Some((_, action)) = self
                .hits
                .actions
                .iter()
                .find(|(rect, _)| rect.contains(position))
            {
                self.command(action);
            } else if self.hits.editor.contains(position) {
                self.focus = Focus::Editor;
                self.place_cursor(mouse.column, mouse.row);
                self.redraw = true;
            }
        }
    }

    fn place_cursor(&mut self, column: u16, row: u16) {
        use guinea_core::coordinates::display_grapheme;
        let rect = self.hits.editor_text;
        if rect.height == 0 || row < rect.y || row >= rect.bottom() {
            return;
        }
        let tab = &self.tabs[self.active];
        let line = tab.top + (row - rect.y) as usize;
        let Some(raw) = tab.document.buffer().line(line) else {
            return;
        };
        let text = raw
            .strip_suffix("\r\n")
            .or_else(|| raw.strip_suffix('\n'))
            .unwrap_or(&raw);
        let start = tab
            .document
            .buffer()
            .line_to_char(line)
            .expect("visible line");
        let target = tab.left + column.saturating_sub(rect.x) as usize;
        let mut cell = 0;
        let mut offset = 0;
        for grapheme in text.graphemes(true) {
            let width = display_grapheme(grapheme, cell, self.config.tab_width).1;
            if target < cell + width {
                if target - cell > width / 2 {
                    offset += grapheme.chars().count();
                }
                break;
            }
            cell += width;
            offset += grapheme.chars().count();
        }
        let _ = self.document_mut().select(start + offset, start + offset);
    }

    fn confirm(&mut self, action: Action) {
        match action {
            Action::Quit => self.exit = true,
            Action::Close => self.close(),
            Action::Reload => {
                if let Some(path) = self.document().path.clone() {
                    self.start_open(
                        path,
                        Some(self.document().id),
                        self.config.large_file_warning_bytes,
                    );
                }
            }
            Action::OpenLarge { path, replace } => self.start_open(path, replace, u64::MAX),
        }
    }

    fn close(&mut self) {
        self.tabs.remove(self.active);
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
        if self.tabs.is_empty() {
            let mut document = Document::default();
            document.readonly = self.readonly;
            self.tabs.push(Tab::new(document));
        }
    }

    pub fn command(&mut self, command: &str) {
        match command {
            "new" => {
                let mut document = Document::default(); document.readonly = self.readonly;
                self.tabs.push(Tab::new(document)); self.active = self.tabs.len() - 1;
                self.focus = Focus::Editor;
            }
            "open-folder" => self.prompt(InputKind::Folder),
            "explorer" => { self.explorer_visible = !self.explorer_visible; self.focus = if self.explorer_visible { Focus::Explorer } else { Focus::Editor }; }
            "focus" => { self.explorer_visible = true; self.focus = if self.focus == Focus::Editor { Focus::Explorer } else { Focus::Editor }; }
            "refresh" => self.explorer.refresh(),
            "open" => self.prompt(InputKind::Open),
            "save-as" => self.prompt(InputKind::SaveAs),
            "save" => {
                if let Some(path) = self.document().path.clone() { self.start_save(path); } else { self.prompt(InputKind::SaveAs); }
            }
            "close" if self.document().is_dirty() => self.modal = Some(Modal::Confirm { action: Action::Close, message: "Discard unsaved changes in this tab? Y discard / N cancel (Ctrl+S saves after cancel)".into() }),
            "close" => self.close(),
            "quit" if self.tabs.iter().any(|t| t.document.is_dirty()) => self.modal = Some(Modal::Confirm { action: Action::Quit, message: "Discard ALL unsaved tabs and quit? Y discard / N cancel (save tabs after cancel)".into() }),
            "quit" => self.exit = true,
            "reload" if self.document().path.is_some() => self.modal = Some(Modal::Confirm { action: Action::Reload, message: "Reload from disk and discard this tab's edits/history? Y reload / N cancel".into() }),
            "reload" => self.message = "Untitled buffer has no file to reload".into(),
            "next" => { self.active = (self.active + 1) % self.tabs.len(); self.focus = Focus::Editor; }
            "prev" => { self.active = (self.active + self.tabs.len() - 1) % self.tabs.len(); self.focus = Focus::Editor; }
            "help" => self.modal = Some(Modal::Help),
            "find" => self.prompt(InputKind::Find),
            "find-next" => self.find_next(),
            "replace" => self.prompt(InputKind::ReplaceNeedle),
            "undo" => self.edit(|d| d.undo().map(|_| ())),
            "redo" => self.edit(|d| d.redo().map(|_| ())),
            "select-all" => self.document_mut().select_all(),
            "copy" => { self.clipboard = self.document().selected_text(); self.message = "Copied to internal clipboard".into(); }
            "cut" => {
                if self.document().readonly { self.message = "Document is read-only".into(); }
                else if !self.document().selection().is_empty() {
                    self.clipboard = self.document().selected_text(); self.edit(|d| d.insert(""));
                }
            }
            "paste" => { let text = self.clipboard.clone(); self.edit(|d| d.insert(&text)); }
            _ => self.message = format!("Unknown command: {command}. F1 lists commands."),
        }
        self.redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    fn app() -> App {
        App::new(vec![], false, Config::default())
    }

    fn drain_jobs(a: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            a.poll_jobs();
            if !a.busy() && a.queued.is_empty() {
                return;
            }
            assert!(std::time::Instant::now() < deadline, "worker timed out");
            thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    fn drain_workspace(a: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            a.poll_jobs();
            if !a.busy() && a.queued.is_empty() && !a.explorer.busy() {
                return;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn directory_navigation_opens_files_and_preserves_dirty_tabs() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("src")).unwrap();
        std::fs::write(directory.path().join("src/note.txt"), "original").unwrap();
        std::fs::write(directory.path().join("Việt.md"), "second").unwrap();
        let mut a = App::new(vec![directory.path().into()], false, Config::default());
        drain_workspace(&mut a);
        assert!(a.explorer_visible);
        assert_eq!(a.focus, Focus::Explorer);
        a.handle(key(KeyCode::Char('x')));
        a.handle(Event::Paste("must not edit from tree".into()));
        assert_eq!(a.document().text(), "");
        assert!(!a.document().is_dirty());
        a.handle(key(KeyCode::Right));
        drain_workspace(&mut a);
        assert_eq!(a.explorer.rows[1].entry.name, "note.txt");
        a.handle(key(KeyCode::Down));
        a.handle(key(KeyCode::Enter));
        drain_workspace(&mut a);
        assert_eq!(a.focus, Focus::Editor);
        assert_eq!(a.document().text(), "original");
        a.handle(Event::Paste("edit ".into()));
        a.command("focus");
        a.explorer.selected = a
            .explorer
            .rows
            .iter()
            .position(|r| r.entry.name == "Việt.md")
            .unwrap();
        a.handle(key(KeyCode::Enter));
        drain_workspace(&mut a);
        assert_eq!(a.tabs.len(), 2);
        assert_eq!(a.document().text(), "second");
        assert_eq!(a.tabs[0].document.text(), "edit original");
        assert!(a.tabs[0].document.is_dirty());
        a.command("quit");
        assert!(matches!(a.modal, Some(Modal::Confirm { .. })));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("src/note.txt")).unwrap(),
            "original"
        );
    }

    #[test]
    fn standalone_file_mode_and_open_prompt_accept_folder() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.txt");
        std::fs::write(&path, "file").unwrap();
        let mut a = App::new(vec![path], false, Config::default());
        drain_jobs(&mut a);
        assert!(!a.explorer_visible);
        a.submit(
            InputKind::Open,
            directory.path().to_string_lossy().into_owned(),
        );
        drain_workspace(&mut a);
        assert!(a.explorer_visible);
        assert_eq!(a.focus, Focus::Explorer);
        assert_eq!(a.document().text(), "file");
    }

    #[test]
    fn mouse_hit_regions_activate_tabs_and_workspace_actions() {
        let mut a = app();
        a.command("new");
        a.hits.tabs.push((Rect::new(40, 3, 20, 1), 0));
        a.handle(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 42,
            row: 3,
            modifiers: KeyModifiers::NONE,
        }));
        assert_eq!(a.active, 0);
        a.hits.actions.push((Rect::new(1, 1, 16, 1), "open-folder"));
        a.handle(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: 1,
            modifiers: KeyModifiers::NONE,
        }));
        assert!(matches!(
            a.modal,
            Some(Modal::Input {
                kind: InputKind::Folder,
                ..
            })
        ));
    }

    #[test]
    fn mouse_cursor_placement_uses_cells_and_keeps_graphemes_whole() {
        let mut a = app();
        a.handle(Event::Paste("e\u{301}\t界👩‍💻\r\nnext".into()));
        a.hits.editor = Rect::new(30, 6, 60, 20);
        a.hits.editor_text = Rect::new(34, 6, 56, 20);
        let click = |column, row| {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };
        a.handle(click(34, 6));
        assert_eq!(a.document().cursor(), 0);
        a.handle(click(38, 6));
        assert_eq!(a.document().cursor(), 3);
        a.handle(click(40, 6));
        assert_eq!(a.document().cursor(), 4);
        assert!(a.document().is_grapheme_boundary(a.document().cursor()));
        a.handle(click(34, 7));
        assert_eq!(a.document().position(), (1, 0));
    }

    #[test]
    fn worker_open_save_find_replace_and_conflict_keep_tabs_separate() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.txt");
        let path_b = dir.path().join("b.txt");
        std::fs::write(&path_a, "Việt\r\n").unwrap();
        std::fs::write(&path_b, "👩‍💻\n").unwrap();
        let mut a = App::new(
            vec![path_a.clone(), path_b.clone()],
            false,
            Config::default(),
        );
        drain_jobs(&mut a);
        assert_eq!(a.tabs.len(), 2);
        a.handle(Event::Paste("changed ".into()));
        a.command("save");
        a.handle(Event::Paste("must not arrive while busy".into()));
        drain_jobs(&mut a);
        assert_eq!(std::fs::read_to_string(&path_b).unwrap(), "changed 👩‍💻\n");
        a.command("prev");
        assert_eq!(a.document().text(), "Việt\r\n");
        a.submit(InputKind::Find, "Việt".into());
        drain_jobs(&mut a);
        assert_eq!(a.document().selected_text(), "Việt");
        a.submit(InputKind::ReplaceWith("Việt".into()), "Vietnam".into());
        drain_jobs(&mut a);
        assert_eq!(a.document().text(), "Vietnam\r\n");
        std::fs::write(&path_a, "external").unwrap();
        a.command("save");
        drain_jobs(&mut a);
        assert!(a.document().is_dirty());
        assert!(a.message.contains("changed outside"));
        assert_eq!(std::fs::read_to_string(path_a).unwrap(), "external");
        assert_eq!(a.tabs[1].document.text(), "changed 👩‍💻\n");
    }

    #[test]
    fn idle_does_not_request_redraw_and_large_open_needs_yes() {
        let mut a = app();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while a.explorer.busy() {
            a.poll_jobs();
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(2));
        }
        a.redraw = false;
        for _ in 0..100 {
            a.poll_jobs();
        }
        assert!(!a.redraw);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large");
        std::fs::write(&path, "larger than warning").unwrap();
        a.start_open(path, None, 1);
        drain_jobs(&mut a);
        assert!(matches!(
            a.modal,
            Some(Modal::Confirm {
                action: Action::OpenLarge { .. },
                ..
            })
        ));
        a.handle(key(KeyCode::Char('n')));
        assert_eq!(a.document().text(), "");
    }

    #[test]
    fn dirty_close_and_quit_require_explicit_yes() {
        let mut a = app();
        a.handle(key(KeyCode::Char('ế')));
        a.command("close");
        assert!(matches!(a.modal, Some(Modal::Confirm { .. })));
        a.handle(key(KeyCode::Enter));
        assert_eq!(a.document().text(), "ế");
        a.handle(key(KeyCode::Esc));
        assert!(a.document().is_dirty());
        a.command("quit");
        a.handle(key(KeyCode::Char('n')));
        assert!(!a.exit);
        a.command("quit");
        a.handle(key(KeyCode::Char('y')));
        assert!(a.exit);
    }

    #[test]
    fn tabs_clipboard_paste_history_and_fallback() {
        let mut a = app();
        a.handle(Event::Paste("e\u{301} 👩‍💻\r\n".into()));
        a.command("select-all");
        a.command("copy");
        a.command("new");
        a.command("paste");
        assert_eq!(a.document().text(), "e\u{301} 👩‍💻\r\n");
        a.command("undo");
        assert_eq!(a.document().text(), "");
        a.handle(key(KeyCode::F(6)));
        assert_eq!(a.document().text(), "e\u{301} 👩‍💻\r\n");
        a.handle(key(KeyCode::F(2)));
        a.handle(Event::Paste("new".into()));
        a.handle(key(KeyCode::Enter));
        assert_eq!(a.tabs.len(), 3);
        assert_eq!(a.document().text(), "");
    }

    #[test]
    fn windows_release_events_do_not_duplicate_text() {
        let mut a = app();
        let mut k = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        a.handle(Event::Key(k));
        k.kind = KeyEventKind::Release;
        a.handle(Event::Key(k));
        assert_eq!(a.document().text(), "x");
    }

    #[test]
    fn readonly_applies_to_new_tabs_and_paste() {
        let mut a = App::new(vec![], true, Config::default());
        a.command("new");
        a.handle(Event::Paste("cannot insert".into()));
        assert_eq!(a.document().text(), "");
        assert!(a.document().readonly);
    }
}
