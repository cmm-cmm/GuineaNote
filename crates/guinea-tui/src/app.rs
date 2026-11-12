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
use crate::review::{Review, ViewMode};
#[path = "agent.rs"]
mod agent;
#[path = "project_app.rs"]
mod project_ui;
#[path = "search_app.rs"]
mod search_ui;
use crate::{
    file_info::FileInfo,
    file_picker::{Action as PickerAction, FilePicker},
    i18n::Locale,
    preferences::Preferences,
    tab_manager::TabManager,
};

pub(crate) fn document_language(document: &Document) -> guinea_syntax::Language {
    let detected = guinea_syntax::Language::detect(
        document
            .path
            .as_deref()
            .or(document.recovery_source.as_deref()),
    );
    if !matches!(
        detected,
        guinea_syntax::Language::Plain
            | guinea_syntax::Language::Generic
            | guinea_syntax::Language::Config
    ) {
        return detected;
    }
    let mut prefix = String::with_capacity(160);
    let _ =
        document
            .buffer()
            .visit_chunks(0..document.buffer().len_chars().min(160), &mut |chunk| {
                prefix.push_str(chunk);
                !prefix.contains('\n')
            });
    guinea_syntax::Language::detect_content(
        document
            .path
            .as_deref()
            .or(document.recovery_source.as_deref()),
        &prefix,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Explorer,
}

#[derive(Default)]
pub struct HitRegions {
    pub tree: Rect,
    pub tabs: Vec<(Rect, usize)>,
    pub tab_closes: Vec<(Rect, usize)>,
    pub tab_strip: Rect,
    pub actions: Vec<(Rect, &'static str)>,
    pub editor: Rect,
    pub editor_text: Rect,
    pub review: Rect,
    pub modal_actions: Vec<(Rect, &'static str)>,
    pub modal: Rect,
    pub outline: Rect,
    pub commands: Vec<(Rect, &'static str)>,
}

pub const HELP: &str = crate::keyboard::GUIDE_EN;

pub struct Tab {
    pub id: u64,
    pub ai_action: Option<String>,
    pub title_override: Option<String>,
    pub document: Document,
    pub untitled: usize,
    pub review_source: Option<u64>,
    pub top: usize,
    pub left: usize,
    pub follow_cursor: bool,
    pub review: Review,
}

impl Tab {
    fn new(document: Document) -> Self {
        static NEXT_TAB: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_TAB.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ai_action: None,
            title_override: None,
            document,
            untitled: 1,
            review_source: None,
            top: 0,
            left: 0,
            follow_cursor: true,
            review: Review::default(),
        }
    }
    pub fn title(&self, locale: Locale) -> String {
        if let Some(title) = &self.title_override {
            return title.clone();
        }
        let name = self
            .document
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("{} {}", locale.tr("Untitled"), self.untitled));
        if self.review_source.is_some() {
            format!("{name} [Review]")
        } else {
            name
        }
    }
}

#[derive(Debug, Clone)]
pub enum InputKind {
    Open,
    Folder,
    SaveAs,
    Command,
    GoToLine,
}

impl InputKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Open => "Open file or folder",
            Self::Folder => "Open folder",
            Self::SaveAs => "Save to NEW path",
            Self::Command => "Command (F1 lists commands)",
            Self::GoToLine => "Go to line[:column] (1-based)",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    Close,
    CloseTab(usize),
    Quit,
    Reload,
    OpenLarge { path: PathBuf, replace: Option<u64> },
}

#[derive(PartialEq, Eq)]
enum ModalClick {
    Tab(u64),
    File(PathBuf),
    Search(u64, usize),
}

pub enum Modal {
    Input {
        kind: InputKind,
        value: String,
    },
    Confirm {
        action: Action,
        message: String,
        selected: bool,
    },
    Help,
    Settings {
        selected: Locale,
    },
    ReviewOptions {
        selected: usize,
    },
    FilePicker(Box<FilePicker>),
    FileInfo(FileInfo),
    Tabs(TabManager),
    Search(crate::search::Search),
    Project(Box<crate::project::Project>),
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
    pub agent: Option<guinea_service::ipc::Bridge>,
    agent_leases: std::collections::HashMap<u64, agent::LeaseState>,
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
    pub preferences: Preferences,
    pub recovery: crate::recovery::Recovery,
    pub palette_index: usize,
    pub help_scroll: usize,
    pub help_height: usize,
    pub help_rows: usize,
    pub focus_mode: bool,
    pub layout: crate::layout_worker::LayoutWorker,
    project_jump: Option<crate::project::Found>,
    mouse_selecting: Option<u64>,
    last_modal_click: Option<(std::time::Instant, ModalClick)>,
    saving: bool,
    quit_after_settings: bool,
    readonly: bool,
    next_untitled: usize,
    pub clipboard: crate::clipboard::Clipboard,
    paste_target: Option<u64>,
    input_batch: bool,
    pending_indent: Option<(u64, u64, String)>,
    search: String,
    replacement: String,
    search_options: guinea_core::SearchOptions,
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
        let clipboard = crate::clipboard::Clipboard::new(config.clipboard);
        Self {
            agent: None,
            agent_leases: std::collections::HashMap::new(),
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
            preferences: Preferences::default(),
            recovery: crate::recovery::Recovery::default(),
            palette_index: 0,
            help_scroll: 0,
            help_height: 1,
            help_rows: 0,
            focus_mode: false,
            layout: crate::layout_worker::LayoutWorker::default(),
            project_jump: None,
            mouse_selecting: None,
            last_modal_click: None,
            readonly,
            next_untitled: 2,
            saving: false,
            quit_after_settings: false,
            clipboard,
            paste_target: None,
            input_batch: false,
            pending_indent: None,
            search: String::new(),
            replacement: String::new(),
            search_options: guinea_core::SearchOptions::default(),
            pending: None,
            worker: None,
            queued: files.into(),
        }
    }

    pub fn background_work(&self) -> bool {
        !self.agent_leases.is_empty()
            || self.layout.busy()
            || self.clipboard.busy()
            || (cfg!(not(windows)) && self.agent.is_some())
            || self.busy()
            || self.preferences.busy()
            || self.recovery.busy()
            || matches!(&self.modal, Some(Modal::FilePicker(p)) if p.busy())
            || matches!(&self.modal, Some(Modal::FileInfo(p)) if p.busy())
            || matches!(&self.modal, Some(Modal::Search(p)) if p.busy())
            || matches!(&self.modal, Some(Modal::Project(p)) if p.busy())
            || self.explorer.scanning()
            || (self.explorer_visible && self.explorer.busy())
            || self.tabs[self.active]
                .review
                .needs_poll(self.document(), self.config.language)
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn document(&self) -> &Document {
        &self.tabs[self.active].document
    }
    /// Whether the caret line's exact layout is being (or must be) built on
    /// the layout worker; render and mouse code then avoid exact queries.
    pub fn caret_layout_pending(&self) -> bool {
        let document = self.document();
        let line = document.cursor_line();
        let start = document.buffer().line_to_char(line).unwrap_or(0);
        document.layout_pending(
            line,
            self.config.tab_width,
            document.cursor().saturating_sub(start),
            0,
        )
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
        self.palette_index = 0;
        self.modal = Some(Modal::Input {
            kind,
            value: String::new(),
        });
    }

    fn edit(&mut self, command: impl FnOnce(&mut Document) -> Result<(), guinea_core::EditError>) {
        if self.tabs[self.active].ai_action.is_some() {
            self.message = "AI is working in this tab; read-only until it finishes".into();
            return;
        }
        if self.tabs[self.active].review.mode == ViewMode::Review {
            self.message = "Review is read-only; F8 or Esc returns to source".into();
            return;
        }
        let trimmed = self.document().history_status().trimmed;
        if let Err(error) = command(self.document_mut()) {
            self.message = error.to_string();
        } else if self.document().history_status().trimmed != trimmed {
            self.message = self
                .config
                .language
                .tr("Older Undo steps trimmed to keep history within its budget")
                .into_owned();
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
        let tab_width = self.config.tab_width;
        self.worker = Some(thread::spawn(move || {
            let result = guinea_io::open(&path, readonly, limit)
                .inspect(|document| {
                    document.prepare_view(tab_width);
                })
                .map_err(Into::into);
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
        let mut document = self.document().worker_snapshot();
        let job = Job::Save { id: document.id };
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.saving = true;
        self.message = format!("Saving {}… you can keep editing", path.display());
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
        let mut document = self.document().worker_snapshot();
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
        self.poll_recovery();
        self.poll_agent();
        self.poll_search();
        if self.layout.poll(&self.tabs) {
            self.redraw = true;
        }
        self.poll_clipboard();
        if let Some(Modal::Project(project)) = &mut self.modal {
            self.redraw |= project.poll();
        }
        let mut documents_changed = false;
        if let Some(result) = self.preferences.poll() {
            self.redraw = true;
            self.message = match result {
                Ok(()) => "Settings saved".into(),
                Err(error) => format!("Settings apply to this session; save failed: {error}"),
            };
        }
        let opened = match &mut self.modal {
            Some(Modal::FilePicker(picker)) => {
                let (changed, path) = picker.poll();
                self.redraw |= changed;
                path
            }
            Some(Modal::FileInfo(info)) => {
                self.redraw |= info.poll();
                None
            }
            _ => None,
        };
        if let Some(path) = opened {
            self.modal = None;
            self.start_open(path, None, self.config.large_file_warning_bytes);
            self.redraw = true;
        }
        if (self.explorer_visible || self.explorer.scanning()) && self.explorer.poll() {
            self.redraw = true;
        }
        if let Some(receiver) = &self.pending {
            let result = receiver.try_recv();
            match result {
                Ok(completed) => {
                    documents_changed = true;
                    self.pending = None;
                    self.saving = false;
                    self.join_worker();
                    self.redraw = true;
                    self.finish_job(completed);
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.saving = false;
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
        self.sync_review_tabs();
        let mut cached = self
            .tabs
            .iter()
            .map(|t| t.review.cache_bytes())
            .sum::<usize>();
        let mut layouts = self
            .tabs
            .iter()
            .map(|t| t.document.view_cache_bytes())
            .sum::<usize>();
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            if index == self.active {
                continue;
            }
            tab.review.suspend();
            if cached > 32 * 1024 * 1024 {
                let before = tab.review.cache_bytes();
                tab.review.release_cache();
                cached = cached.saturating_sub(before - tab.review.cache_bytes());
            }
            if layouts > 32 * 1024 * 1024 {
                let bytes = tab.document.view_cache_bytes();
                tab.document.clear_view_cache();
                layouts = layouts.saturating_sub(bytes);
            }
        }
        if documents_changed && let Some(Modal::Tabs(manager)) = &mut self.modal {
            manager.refresh(&self.tabs, self.active, self.config.language);
        }
        let tab = &mut self.tabs[self.active];
        if tab.review.poll(
            &tab.document,
            self.config.basic_colors,
            self.config.language,
        ) {
            self.redraw = true;
        }
        if self.quit_after_settings && !self.preferences.busy() {
            self.exit = true;
        }
    }

    fn poll_recovery(&mut self) {
        if let Some(outcome) = self.recovery.poll() {
            match outcome {
                crate::recovery::Outcome::Loaded(drafts, warnings) => {
                    let count = drafts.len();
                    for mut recovered in drafts {
                        recovered.document.readonly = self.readonly;
                        let name = recovered
                            .source
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Untitled".into());
                        let mut tab = Tab::new(recovered.document);
                        tab.title_override =
                            Some(format!("{} · {name}", self.config.language.tr("Recovered")));
                        self.tabs.push(tab);
                        self.active = self.tabs.len() - 1;
                    }
                    if count > 0 {
                        self.message = self
                            .config
                            .language
                            .tr("Recovered drafts opened; use Save As to keep them")
                            .into_owned();
                    }
                    if !warnings.is_empty() {
                        self.message = format!(
                            "{}: {}",
                            self.config.language.tr("Recovery warning"),
                            warnings.join("; ")
                        );
                    }
                }
                crate::recovery::Outcome::Error(error) => {
                    self.message =
                        format!("{}: {error}", self.config.language.tr("Recovery warning"));
                }
                crate::recovery::Outcome::Saved => {}
            }
            self.redraw = true;
        }
        self.recovery.checkpoint(&self.tabs);
    }

    fn finish_job(&mut self, completed: Completed) {
        let opened = matches!(&completed.job, Job::Open { .. });
        match completed.result {
            Err(WorkerError::File(FileError::LargeFile { size, limit })) => {
                if let Job::Open { path, replace, .. } = completed.job {
                    self.modal = Some(Modal::Confirm {
                        selected: false,
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
            Ok(mut document) => {
                match completed.job {
                    Job::Save { id } => {
                        if let Some(tab) = self
                            .tabs
                            .iter_mut()
                            .find(|t| t.review_source.is_none() && t.document.id == id)
                        {
                            let recovered = tab.document.recovery_source.is_some();
                            tab.document.accept_saved_snapshot(&document);
                            if recovered {
                                tab.title_override = None;
                            }
                        }
                        self.message = completed.warning.unwrap_or_else(|| "Saved".into());
                    }
                    Job::Compute { id } => {
                        if let Some(tab) = self
                            .tabs
                            .iter_mut()
                            .find(|t| t.review_source.is_none() && t.document.id == id)
                        {
                            if document.revision() == tab.document.revision() {
                                let selection = document.selection();
                                let _ = tab.document.select(selection.anchor, selection.head);
                            } else {
                                let revision = tab.document.revision();
                                let _ = tab.document.accept_prepared(&document, revision);
                            }
                        }
                        self.message = completed.warning.unwrap_or_else(|| "Saved".into());
                    }
                    Job::Open {
                        replace: Some(id), ..
                    } => {
                        if let Some(tab) = self
                            .tabs
                            .iter_mut()
                            .find(|t| t.review_source.is_none() && t.document.id == id)
                        {
                            document.continue_identity(&tab.document);
                            *tab = Tab::new(document);
                        }
                        self.message = "Reloaded".into();
                        self.focus = Focus::Editor;
                    }
                    Job::Open { .. } => {
                        if let Some(idx) = self.tabs.iter().position(|t| {
                            t.review_source.is_none() && t.document.path == document.path
                        }) {
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
                }
            }
        }
        if opened {
            self.apply_project_jump();
        }
    }

    pub fn handle(&mut self, event: Event) {
        if self.quit_after_settings {
            if matches!(event, Event::Key(key) if key.code == KeyCode::Esc) {
                self.quit_after_settings = false;
                self.message = "Cancelled".into();
                self.redraw = true;
            }
            return;
        }
        if matches!(event, Event::Resize(..)) {
            self.redraw = true;
            return;
        }
        if self.busy() && !self.saving {
            if matches!(event, Event::Key(_)) {
                self.message =
                    "Background operation in progress; edits paused until it completes".into();
                self.redraw = true;
            }
            return;
        }
        let before = (
            self.document().id,
            self.document().cursor(),
            self.document().revision(),
        );
        match event {
            Event::Paste(text) => {
                self.pending_indent = None;
                self.redraw = true;
                if let Some(Modal::Input { value, .. }) = self.modal.as_mut() {
                    if value.len().saturating_add(text.len()) <= 4096 {
                        value.extend(text.chars().filter(|c| !c.is_control()));
                    } else {
                        self.message = "Input is limited to 4096 UTF-8 bytes".into();
                    }
                } else if let Some(Modal::FilePicker(picker)) = self.modal.as_mut() {
                    picker.paste(&text);
                } else if let Some(Modal::Project(project)) = self.modal.as_mut() {
                    project.paste(&text);
                } else if let Some(Modal::Tabs(manager)) = self.modal.as_mut() {
                    manager.paste(&text);
                } else if let Some(Modal::Search(search)) = self.modal.as_mut() {
                    search.paste(&text);
                    self.search = search.query.clone();
                    self.replacement = search.replacement.clone();
                } else if self.modal.is_none()
                    && self.focus == Focus::Editor
                    && self.tabs[self.active].review.mode != ViewMode::Review
                {
                    self.edit(|d| d.insert(&text));
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                // More input in the same burst after Enter means a paste.
                self.pending_indent = None;
                self.last_modal_click = None;
                if self.modal.is_none()
                    && self.focus == Focus::Editor
                    && self.tabs[self.active].review.mode == ViewMode::Review
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !(key.modifiers == KeyModifiers::ALT
                        && matches!(key.code, KeyCode::Char('m' | 'M')))
                    && !matches!(
                        key.code,
                        KeyCode::Up
                            | KeyCode::Down
                            | KeyCode::Left
                            | KeyCode::Right
                            | KeyCode::PageUp
                            | KeyCode::PageDown
                            | KeyCode::Home
                            | KeyCode::End
                            | KeyCode::Esc
                            | KeyCode::Tab
                            | KeyCode::Enter
                            | KeyCode::F(1..=12)
                            | KeyCode::Char('j' | 'k' | ' ')
                    )
                {
                    let message = "Review is read-only; F8 or Esc returns to source";
                    if self.message != message {
                        self.message = message.into();
                        self.redraw = true;
                    }
                    return;
                }
                self.redraw = true;
                // Search/project panels can be dismissed safely by tab keys.
                // Confirmation and save-path dialogs retain their own keys.
                let tab_context = self.modal.is_none()
                    || matches!(&self.modal, Some(Modal::Project(_)))
                    || matches!(&self.modal, Some(Modal::Search(search)) if !search.replacing());
                if tab_context && let Some(command) = crate::commands::tab_command(key) {
                    self.modal = None;
                    self.command(command);
                    return;
                }
                if self.modal.is_some() {
                    self.modal_key(key);
                } else {
                    self.editor_key(key);
                }
            }
            Event::Mouse(mouse) if self.config.mouse => {
                if self.modal.is_none() {
                    self.mouse(mouse);
                } else {
                    self.modal_mouse(mouse);
                }
            }
            _ => {}
        }
        if self.document().id == before.0
            && (self.document().cursor() != before.1 || self.document().revision() != before.2)
        {
            self.tabs[self.active].follow_cursor = true;
        }
    }

    fn modal_key(&mut self, key: KeyEvent) {
        let modal = self.modal.take().expect("modal exists");
        match modal {
            Modal::Search(search) => self.search_key(search, key),
            Modal::Project(project) => self.project_key(project, key),
            Modal::Tabs(mut manager) => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                match key.code {
                    KeyCode::Esc => return,
                    KeyCode::Char('p') if ctrl => return,
                    KeyCode::F(12) => return,
                    KeyCode::Char('n' | 't') if ctrl => {
                        self.command("new");
                        return;
                    }
                    KeyCode::Char('s') if ctrl => {
                        if let Some(index) = manager.current() {
                            self.active = index;
                            self.focus = Focus::Editor;
                            self.command("save");
                            return;
                        }
                    }
                    KeyCode::Char('w') if ctrl => {
                        if let Some(index) = manager.current() {
                            self.request_close_tab(index);
                            return;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(index) = manager.current() {
                            self.active = index;
                            self.focus = Focus::Editor;
                            return;
                        }
                    }
                    KeyCode::Up => manager.move_by(-1),
                    KeyCode::Down => manager.move_by(1),
                    KeyCode::Tab | KeyCode::BackTab => manager.move_by(
                        if key.code == KeyCode::BackTab
                            || key.modifiers.contains(KeyModifiers::SHIFT)
                        {
                            -1
                        } else {
                            1
                        },
                    ),
                    _ if crate::keyboard::tab_command(key) == Some("next") => manager.move_by(1),
                    _ if crate::keyboard::tab_command(key) == Some("prev") => manager.move_by(-1),
                    KeyCode::PageUp => manager.move_by(-(manager.height as isize)),
                    KeyCode::PageDown => manager.move_by(manager.height as isize),
                    KeyCode::Home => manager.selected = 0,
                    KeyCode::End => manager.selected = manager.filtered.len().saturating_sub(1),
                    KeyCode::Backspace => manager.backspace(),
                    KeyCode::Char('u') if ctrl => {
                        manager.query.clear();
                        manager.filter();
                    }
                    KeyCode::Char(ch)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        manager.paste(&ch.to_string())
                    }
                    _ => {}
                }
                self.modal = Some(Modal::Tabs(manager));
            }
            Modal::Settings { mut selected } => match key.code {
                KeyCode::Esc | KeyCode::F(10) => self.message = "Cancelled".into(),
                KeyCode::Left
                | KeyCode::Up
                | KeyCode::Right
                | KeyCode::Down
                | KeyCode::Tab
                | KeyCode::BackTab => {
                    selected = selected.toggle();
                    self.modal = Some(Modal::Settings { selected });
                }
                KeyCode::Char('v' | 'V') => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::Vi,
                    })
                }
                KeyCode::Char('e' | 'E') => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::En,
                    })
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.apply_settings(selected),
                _ => self.modal = Some(Modal::Settings { selected }),
            },
            Modal::ReviewOptions { mut selected } => match key.code {
                KeyCode::Esc | KeyCode::F(8) => self.message = "Cancelled".into(),
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Tab
                | KeyCode::BackTab => {
                    selected = 1 - selected;
                    self.modal = Some(Modal::ReviewOptions { selected });
                }
                KeyCode::Char('1') => self.command("split"),
                KeyCode::Char('2') => self.command("review-tab"),
                KeyCode::Enter => self.command(if selected == 0 { "split" } else { "review-tab" }),
                _ => self.modal = Some(Modal::ReviewOptions { selected }),
            },
            Modal::FilePicker(mut picker) => {
                let action = picker.key(key);
                self.picker_action(action, picker);
            }
            Modal::FileInfo(mut info) => {
                match key.code {
                    KeyCode::Esc | KeyCode::F(9) | KeyCode::Enter => return,
                    KeyCode::Up => info.scroll(-1),
                    KeyCode::Down => info.scroll(1),
                    KeyCode::PageUp => info.scroll(-(info.height as isize)),
                    KeyCode::PageDown => info.scroll(info.height as isize),
                    KeyCode::Home => info.top = 0,
                    KeyCode::End => info.scroll(isize::MAX),
                    _ => {}
                }
                self.modal = Some(Modal::FileInfo(info));
            }
            Modal::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::F(1) | KeyCode::Enter => return,
                    KeyCode::Up => self.help_scroll = self.help_scroll.saturating_sub(1),
                    KeyCode::Down => self.help_scroll = self.help_scroll.saturating_add(1),
                    KeyCode::PageUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(self.help_height)
                    }
                    KeyCode::PageDown => {
                        self.help_scroll = self.help_scroll.saturating_add(self.help_height)
                    }
                    KeyCode::Home => self.help_scroll = 0,
                    KeyCode::End => self.help_scroll = self.help_rows,
                    _ => {}
                }
                self.help_scroll = self
                    .help_scroll
                    .min(self.help_rows.saturating_sub(self.help_height));
                self.modal = Some(Modal::Help);
            }
            Modal::Confirm {
                action,
                message,
                mut selected,
            } => match key.code {
                KeyCode::Char('y' | 'Y') => self.confirm(action),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if selected {
                        self.confirm(action);
                    } else {
                        self.message = "Cancelled".into();
                    }
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    selected = !selected;
                    self.modal = Some(Modal::Confirm {
                        action,
                        message,
                        selected,
                    });
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                    self.message = "Cancelled".into();
                }
                _ => {
                    self.modal = Some(Modal::Confirm {
                        action,
                        message,
                        selected,
                    });
                }
            },
            Modal::Input { kind, mut value } => {
                if matches!(kind, InputKind::Command) {
                    let commands = crate::commands::suggestions(&value, self.config.language);
                    match key.code {
                        KeyCode::Up => self.palette_index = self.palette_index.saturating_sub(1),
                        KeyCode::Down => {
                            self.palette_index =
                                (self.palette_index + 1).min(commands.len().saturating_sub(1))
                        }
                        KeyCode::Tab => {
                            if let Some(command) = commands.get(self.palette_index) {
                                value = command.id.into();
                            }
                            self.palette_index = 0;
                        }
                        KeyCode::Enter if !commands.iter().any(|c| c.id == value.trim()) => {
                            if let Some(command) = commands.get(self.palette_index) {
                                value = command.id.into();
                            }
                        }
                        KeyCode::Char(_) | KeyCode::Backspace => self.palette_index = 0,
                        _ => {}
                    }
                }
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
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        value.clear()
                    }
                    KeyCode::Char(ch)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        if value.len() + ch.len_utf8() <= 4096 {
                            value.push(ch);
                        } else {
                            self.message = "Input is limited to 4096 UTF-8 bytes".into();
                        }
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
            InputKind::Command => self.command(value.trim()),
            InputKind::GoToLine => {
                let coordinates = value
                    .split(':')
                    .map(str::trim)
                    .map(str::parse::<usize>)
                    .collect::<Result<Vec<_>, _>>();
                let Ok(coordinates) = coordinates else {
                    self.message = "Invalid line or column".into();
                    return;
                };
                if coordinates.is_empty() || coordinates.len() > 2 || coordinates.contains(&0) {
                    self.message = "Invalid line or column".into();
                    return;
                }
                let line = coordinates[0] - 1;
                let column = coordinates.get(1).copied().unwrap_or(1) - 1;
                self.start_compute("Go to line", move |document| {
                    let start = document.buffer().line_to_char(line)?;
                    let bounds = document.line_bounds(line)?;
                    let text = document.buffer().slice(bounds.clone())?;
                    let byte = text
                        .grapheme_indices(true)
                        .map(|(byte, _)| byte)
                        .chain(std::iter::once(text.len()))
                        .nth(column)
                        .ok_or(guinea_core::EditError::OutOfBounds)?;
                    let at = start + text[..byte].chars().count();
                    document.select(at, at)?;
                    Ok("Moved to line".into())
                });
            }
            _ => {
                self.message = "Empty input; cancelled".into();
            }
        }
    }

    fn editor_key(&mut self, key: KeyEvent) {
        if matches!(
            key.code,
            KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        ) {
            self.tabs[self.active].follow_cursor = true;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let review = self.tabs[self.active].review.mode == ViewMode::Review;
        if ctrl && review && matches!(key.code, KeyCode::Home | KeyCode::End) {
            if key.code == KeyCode::Home {
                self.tabs[self.active].review.top = 0;
            } else {
                let height = self.page_height;
                self.tabs[self.active].review.scroll(isize::MAX, height);
            }
            return;
        }
        // Keymap v1 (commands.rs) owns every shortcut; the rest is context input.
        let editor = self.focus == Focus::Editor && !review;
        if let Some(command) = crate::commands::lookup(key, editor, false) {
            self.command(command);
            return;
        }
        if self.focus == Focus::Explorer {
            self.tree_key(key);
            return;
        }
        if review {
            let height = self.page_height;
            let table = matches!(
                document_language(self.document()),
                guinea_syntax::Language::Csv | guinea_syntax::Language::Tsv
            );
            let review = &mut self.tabs[self.active].review;
            if self.hits.outline.width == 0 {
                review.outline_focus = false;
            }
            if key.code == KeyCode::Tab
                && self.hits.outline.width > 0
                && review.outline_visible
                && !review.outline().is_empty()
            {
                review.outline_focus = !review.outline_focus;
                return;
            }
            if review.outline_focus {
                match key.code {
                    KeyCode::Up => {
                        review.outline_selected = review.outline_selected.saturating_sub(1)
                    }
                    KeyCode::Down => {
                        review.outline_selected = (review.outline_selected + 1)
                            .min(review.outline().len().saturating_sub(1))
                    }
                    KeyCode::Home => review.outline_selected = 0,
                    KeyCode::End => {
                        review.outline_selected = review.outline().len().saturating_sub(1)
                    }
                    KeyCode::Enter => review.jump_heading(),
                    KeyCode::Esc => review.outline_focus = false,
                    _ => {}
                }
                return;
            }
            match key.code {
                KeyCode::Left if table => review.column = review.column.saturating_sub(1),
                KeyCode::Right if table => review.column = review.column.saturating_add(1),
                KeyCode::Up => review.scroll(-1, height),
                KeyCode::Down | KeyCode::Char('j') => review.scroll(1, height),
                KeyCode::Char('k') => review.scroll(-1, height),
                KeyCode::PageUp => review.scroll(-(height as isize), height),
                KeyCode::PageDown | KeyCode::Char(' ') => review.scroll(height as isize, height),
                KeyCode::Home => review.top = 0,
                KeyCode::End => review.scroll(isize::MAX, height),
                KeyCode::Esc => self.command("source"),
                _ => {}
            }
            return;
        }
        if ctrl {
            match key.code {
                KeyCode::Home => self.document_mut().move_cursor(Movement::Start, shift),
                KeyCode::End => self.document_mut().move_cursor(Movement::Finish, shift),
                _ => {}
            }
            return;
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
            if !key.modifiers.contains(KeyModifiers::ALT) {
                self.document_mut().move_cursor(movement, shift);
            }
            return;
        }
        match key.code {
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
            KeyCode::Backspace => self.edit(Document::type_backspace),
            KeyCode::Delete => self.edit(Document::type_delete),
            KeyCode::Enter => self.newline(),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::ALT) => {
                self.edit(|d| d.type_text(&ch.to_string()))
            }
            _ => {}
        }
    }

    /// Enter keeps the caret line's indentation. A newline followed by more
    /// input in the same burst is a paste delivered as key events (ConPTY);
    /// its own indentation arrives with it, so auto-indent is cancelled then.
    fn newline(&mut self) {
        let indent = self.document().caret_indentation();
        let newline = self.document().line_ending().newline();
        let revision = self.document().revision();
        self.edit(|d| d.type_text(newline));
        if indent.is_empty() || self.document().revision() == revision {
            return;
        }
        let pending = (self.document().id, self.document().revision(), indent);
        if self.input_batch {
            self.pending_indent = Some(pending);
        } else {
            self.pending_indent = Some(pending);
            self.end_input(true);
        }
    }

    /// Called by the event loop before handling a burst of input events.
    pub fn begin_input(&mut self) {
        self.input_batch = true;
    }

    /// Called after a burst. `drained` is true when no further input was queued.
    pub fn end_input(&mut self, drained: bool) {
        self.input_batch = false;
        if !drained {
            return;
        }
        if let Some((id, revision, indent)) = self.pending_indent.take()
            && self.document().id == id
            && self.document().revision() == revision
        {
            self.edit(|d| d.type_text(&indent));
            self.redraw = true;
        }
    }

    /// Copy the selection, or the whole caret line when nothing is selected.
    fn copy_selection(&mut self) {
        let text = if self.document().selection().is_empty() {
            self.document().current_line_text()
        } else {
            self.document().selected_text()
        };
        self.copy_text(text);
    }

    fn copy_text(&mut self, text: String) {
        use crate::clipboard::Copied;
        self.message = match self.clipboard.copy(text) {
            Copied::System => "Copied to clipboard",
            Copied::Osc52 => "Copied through the terminal (OSC 52)",
            Copied::Internal => "Copied to internal clipboard",
            Copied::TooLargeForOsc52 => {
                "Selection too large for OSC 52; copied to internal clipboard"
            }
        }
        .into();
    }

    fn paste(&mut self) {
        match self.clipboard.paste() {
            crate::clipboard::Paste::Ready(text) => self.edit(|d| d.insert(&text)),
            crate::clipboard::Paste::Pending => {
                self.paste_target = Some(self.document().id);
                self.message = "Pasting from system clipboard…".into();
            }
        }
    }

    fn poll_clipboard(&mut self) {
        for event in self.clipboard.poll() {
            self.redraw = true;
            let (text, fallback) = match event {
                crate::clipboard::Event::CopyFailed(error) => {
                    self.message =
                        format!("System clipboard unavailable; copied internally: {error}");
                    continue;
                }
                crate::clipboard::Event::Pasted(text) => (text, None),
                crate::clipboard::Event::PasteFallback(text, error) => (text, Some(error)),
            };
            let target = self.paste_target.take();
            if target != Some(self.document().id) || self.modal.is_some() {
                self.message = "Paste cancelled: the active tab changed".into();
            } else if text.is_empty() {
                self.message = "Clipboard is empty".into();
            } else {
                self.edit(|d| d.insert(&text));
                self.tabs[self.active].follow_cursor = true;
                if let Some(error) = fallback {
                    self.message =
                        format!("System clipboard unavailable; pasted internal clipboard: {error}");
                }
            }
        }
    }

    fn find_next(&mut self) {
        let needle = self.search.clone();
        let options = self.search_options;
        self.start_compute("Finding", move |d| {
            Ok(if d.find_with(&needle, options) {
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
        if mouse.kind == MouseEventKind::Up(MouseButton::Left) {
            self.mouse_selecting = None;
            return;
        }
        if mouse.kind == MouseEventKind::Drag(MouseButton::Left) {
            if self.mouse_selecting == Some(self.document().id)
                && self.hits.editor.contains(position)
            {
                self.place_cursor(mouse.column, mouse.row, true);
                self.redraw = true;
            }
            return;
        }
        if matches!(mouse.kind, MouseEventKind::Down(_)) {
            self.mouse_selecting = None;
            self.last_modal_click = None;
        }
        if self.hits.tab_strip.contains(position) {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Middle) => {
                    if let Some((_, index)) =
                        self.hits.tabs.iter().find(|(r, _)| r.contains(position))
                    {
                        self.request_close_tab(*index);
                        self.redraw = true;
                    }
                    return;
                }
                MouseEventKind::ScrollDown => {
                    self.command("next");
                    return;
                }
                MouseEventKind::ScrollUp => {
                    self.command("prev");
                    return;
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((_, index)) = self
                        .hits
                        .tab_closes
                        .iter()
                        .find(|(rect, _)| rect.contains(position))
                    {
                        self.request_close_tab(*index);
                        self.redraw = true;
                        return;
                    }
                }
                _ => {}
            }
        }
        if self.hits.outline.contains(position) {
            let rect = self.hits.outline;
            let review = &mut self.tabs[self.active].review;
            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                let top = review
                    .outline_selected
                    .saturating_sub(rect.height as usize / 2)
                    .min(review.outline().len().saturating_sub(rect.height as usize));
                review.outline_selected = (top + mouse.row.saturating_sub(rect.y) as usize)
                    .min(review.outline().len().saturating_sub(1));
                review.jump_heading();
                self.redraw = true;
            }
            return;
        }
        if self.hits.review.contains(position) {
            let height = self.page_height;
            match mouse.kind {
                MouseEventKind::ScrollDown => self.tabs[self.active].review.scroll(3, height),
                MouseEventKind::ScrollUp => self.tabs[self.active].review.scroll(-3, height),
                _ => {}
            }
            self.redraw = true;
        } else if self.hits.tree.contains(position) {
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
        } else if self.hits.editor.contains(position)
            && matches!(
                mouse.kind,
                MouseEventKind::ScrollUp
                    | MouseEventKind::ScrollDown
                    | MouseEventKind::ScrollLeft
                    | MouseEventKind::ScrollRight
            )
        {
            let tab = &mut self.tabs[self.active];
            tab.follow_cursor = false;
            match mouse.kind {
                MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                    tab.left = tab.left.saturating_sub(12)
                }
                MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                    tab.left = tab.left.saturating_add(12)
                }
                MouseEventKind::ScrollLeft => tab.left = tab.left.saturating_sub(12),
                MouseEventKind::ScrollRight => tab.left = tab.left.saturating_add(12),
                MouseEventKind::ScrollUp => tab.top = tab.top.saturating_sub(3),
                MouseEventKind::ScrollDown => {
                    tab.top = tab
                        .top
                        .saturating_add(3)
                        .min(tab.document.buffer().len_lines().saturating_sub(1))
                }
                _ => {}
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
                self.place_cursor(
                    mouse.column,
                    mouse.row,
                    mouse.modifiers.contains(KeyModifiers::SHIFT),
                );
                self.mouse_selecting = Some(self.document().id);
                self.redraw = true;
            }
        }
    }

    fn modal_mouse(&mut self, mouse: MouseEvent) {
        self.redraw = true;
        let point = (mouse.column, mouse.row).into();
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && self
                .hits
                .modal_actions
                .iter()
                .any(|(r, a)| *a == "cancel" && r.contains(point))
        {
            let search = matches!(&self.modal, Some(Modal::Search(_)));
            self.modal = None;
            self.last_modal_click = None;
            self.message = if search { "Search closed" } else { "Cancelled" }.into();
            return;
        }
        if matches!(&self.modal, Some(Modal::Project(_))) {
            self.project_mouse(mouse);
            return;
        }
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && matches!(
                &self.modal,
                Some(Modal::Input {
                    kind: InputKind::Command,
                    ..
                })
            )
            && let Some(command) = self
                .hits
                .commands
                .iter()
                .find(|(r, _)| r.contains(point))
                .map(|(_, c)| *c)
        {
            self.modal = None;
            self.command(command);
            return;
        }
        if matches!(&self.modal, Some(Modal::Search(_))) {
            let Some(Modal::Search(mut search)) = self.modal.take() else {
                unreachable!()
            };
            let mut go = false;
            if search.list.contains(point) && search.ready() {
                match mouse.kind {
                    MouseEventKind::ScrollUp => search.navigate(true),
                    MouseEventKind::ScrollDown => search.navigate(false),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let at = search.top() + mouse.row.saturating_sub(search.list.y) as usize;
                        if let Some(found) = search.entries.get(at) {
                            go = self.double_click(ModalClick::Search(
                                self.document().id,
                                found.range.start,
                            ));
                            search.selected = at;
                            search.focus = 2;
                        }
                    }
                    _ => {}
                }
            }
            self.modal = Some(Modal::Search(search));
            self.search_action(if go { "search-go" } else { "search-select" });
            if go {
                return;
            }
        }
        if matches!(&self.modal, Some(Modal::Tabs(_))) {
            let Some(Modal::Tabs(mut manager)) = self.modal.take() else {
                unreachable!()
            };
            match mouse.kind {
                MouseEventKind::ScrollUp if manager.list.contains(point) => manager.move_by(-3),
                MouseEventKind::ScrollDown if manager.list.contains(point) => manager.move_by(3),
                MouseEventKind::Down(MouseButton::Left) if manager.list.contains(point) => {
                    let row = manager.top + (mouse.row - manager.list.y) as usize;
                    if let Some(&entry) = manager.filtered.get(row) {
                        manager.selected = row;
                        let index = manager.entries[entry].index;
                        if self.double_click(ModalClick::Tab(self.tabs[index].id)) {
                            self.active = index;
                            self.focus = Focus::Editor;
                            return;
                        }
                    }
                }
                _ => {}
            }
            self.modal = Some(Modal::Tabs(manager));
        }
        if matches!(&self.modal, Some(Modal::FilePicker(_))) {
            let Some(Modal::FilePicker(mut picker)) = self.modal.take() else {
                unreachable!()
            };
            let point = (mouse.column, mouse.row).into();
            let mut action = PickerAction::Keep;
            match mouse.kind {
                MouseEventKind::ScrollUp if picker.hits.list.contains(point) => picker.move_by(-3),
                MouseEventKind::ScrollDown if picker.hits.list.contains(point) => picker.move_by(3),
                MouseEventKind::Down(MouseButton::Left) if picker.hits.list.contains(point) => {
                    let row = picker.top + mouse.row.saturating_sub(picker.hits.list.y) as usize;
                    if let Some(&entry) = picker.filtered.get(row) {
                        picker.selected = row;
                        if self.double_click(ModalClick::File(picker.entries[entry].path.clone())) {
                            action = picker.activate();
                        }
                    }
                }
                MouseEventKind::Down(MouseButton::Left) if picker.hits.parent.contains(point) => {
                    picker.parent()
                }
                MouseEventKind::Down(MouseButton::Left) if picker.hits.open.contains(point) => {
                    action = picker.activate()
                }
                MouseEventKind::Down(MouseButton::Left) if picker.hits.cancel.contains(point) => {
                    action = PickerAction::Cancel
                }
                _ => {}
            }
            self.picker_action(action, picker);
            return;
        }
        if let Some(Modal::FileInfo(info)) = &mut self.modal {
            match mouse.kind {
                MouseEventKind::ScrollDown if self.hits.modal.contains(point) => info.scroll(3),
                MouseEventKind::ScrollUp if self.hits.modal.contains(point) => info.scroll(-3),
                _ => {}
            }
        }
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some(action) = self
                .hits
                .modal_actions
                .iter()
                .find(|(r, _)| r.contains((mouse.column, mouse.row).into()))
                .map(|(_, a)| *a)
        {
            match action {
                action if action.starts_with("search-") => self.search_action(action),
                "confirm-accept" => {
                    if let Some(Modal::Confirm { action, .. }) = self.modal.take() {
                        self.confirm(action);
                    }
                }
                "input-submit" => {
                    if let Some(Modal::Input { kind, value }) = self.modal.take() {
                        self.submit(kind, value);
                    }
                }
                "tabs-new" => {
                    self.modal = None;
                    self.command("new");
                }
                "tabs-switch" | "tabs-save" | "tabs-close" => {
                    if let Some(Modal::Tabs(manager)) = self.modal.take() {
                        if let Some(index) = manager.current() {
                            if action == "tabs-close" {
                                self.request_close_tab(index);
                            } else {
                                self.active = index;
                                self.focus = Focus::Editor;
                                if action == "tabs-save" {
                                    self.command("save");
                                }
                            }
                        } else {
                            self.modal = Some(Modal::Tabs(manager));
                        }
                    }
                }
                "cancel" => {
                    self.modal = None;
                    self.message = "Cancelled".into();
                }
                "settings-vi" => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::Vi,
                    })
                }
                "settings-en" => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::En,
                    })
                }
                "settings-apply" => {
                    if let Some(Modal::Settings { selected }) = self.modal.take() {
                        self.apply_settings(selected);
                    }
                }
                "review-split" => {
                    self.modal = None;
                    self.command("split");
                }
                "review-tab" => {
                    self.modal = None;
                    self.command("review-tab");
                }
                _ => {}
            }
        }
    }

    fn double_click(&mut self, target: ModalClick) -> bool {
        let now = std::time::Instant::now();
        let double = self
            .last_modal_click
            .take()
            .is_some_and(|(time, previous)| {
                previous == target
                    && now.duration_since(time) < std::time::Duration::from_millis(450)
            });
        if !double {
            self.last_modal_click = Some((now, target));
        }
        double
    }

    fn place_cursor(&mut self, column: u16, row: u16, extend: bool) {
        use guinea_core::coordinates::display_grapheme;
        let rect = self.hits.editor_text;
        if rect.height == 0 || row < rect.y || row >= rect.bottom() {
            return;
        }
        if self.caret_layout_pending() {
            // The visible caret line is an approximation until indexed.
            self.message = "Indexing long line… click again when it finishes".into();
            return;
        }
        let tab = &self.tabs[self.active];
        let line = tab.top + (row - rect.y) as usize;
        if tab.document.layout_pending(
            line,
            self.config.tab_width,
            0,
            tab.left.saturating_add(rect.width as usize),
        ) {
            self.message = "Indexing long line… click again when it finishes".into();
            return;
        }
        let Ok(window) =
            tab.document
                .line_window(line, tab.left, rect.width as usize, self.config.tab_width)
        else {
            return;
        };
        let text = window.text.as_str();
        let start = window.char_start;
        let target = tab.left + column.saturating_sub(rect.x) as usize;
        let mut cell = window.cell_start;
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
        let anchor = if extend {
            self.document().selection().anchor
        } else {
            start + offset
        };
        let _ = self.document_mut().select(anchor, start + offset);
        self.tabs[self.active].follow_cursor = true;
    }

    fn confirm(&mut self, action: Action) {
        match action {
            Action::Quit => self.request_quit(),
            Action::Close => self.close(),
            Action::CloseTab(index) => self.close_at(index),
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

    fn request_quit(&mut self) {
        if self.preferences.busy() {
            self.quit_after_settings = true;
            self.message = "Saving settings… Esc cancels exit".into();
        } else {
            self.exit = true;
        }
    }
    fn close(&mut self) {
        self.close_at(self.active);
    }
    fn new_blank(&mut self) {
        let mut document = Document::default();
        document.readonly = self.readonly;
        let mut tab = Tab::new(document);
        tab.untitled = self.next_untitled;
        self.next_untitled += 1;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.focus = Focus::Editor;
    }
    fn request_close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.tabs[index].ai_action.is_some() {
            self.message = "AI is working in this tab; read-only until it finishes".into();
            return;
        }
        if self.busy() {
            self.message = "Background operation in progress; retry when complete".into();
        } else if self.tabs[index].review_source.is_none() && self.tabs[index].document.is_dirty() {
            self.modal = Some(Modal::Confirm {
                selected: false,
                action: Action::CloseTab(index),
                message: format!(
                    "{}\n{}",
                    self.tabs[index].title(self.config.language),
                    self.config.language.tr("Discard unsaved changes in this tab? Y discard / N cancel (Ctrl+S saves after cancel)")
                ),
            });
        } else {
            self.close_at(index);
        }
    }
    fn close_at(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        let active_key = (
            self.document().id,
            self.tabs[self.active].review_source.is_some(),
        );
        let review_source = self.tabs[index].review_source;
        let source = self.tabs[index]
            .review_source
            .is_none()
            .then_some(self.tabs[index].document.id);
        self.tabs.remove(index);
        if let Some(id) = source {
            self.tabs.retain(|tab| tab.review_source != Some(id));
        }
        self.active = self
            .tabs
            .iter()
            .position(|t| (t.document.id, t.review_source.is_some()) == active_key)
            .or_else(|| {
                review_source.and_then(|id| {
                    self.tabs
                        .iter()
                        .position(|tab| tab.review_source.is_none() && tab.document.id == id)
                })
            })
            .unwrap_or_else(|| index.min(self.tabs.len().saturating_sub(1)));
        if self.tabs.is_empty() {
            self.new_blank();
        }
        self.focus = Focus::Editor;
    }

    fn sync_review_tabs(&mut self) {
        for index in 0..self.tabs.len() {
            let Some(id) = self.tabs[index].review_source else {
                continue;
            };
            let source = self
                .tabs
                .iter()
                .find(|tab| tab.review_source.is_none() && tab.document.id == id);
            if let Some(source) = source {
                let old = &self.tabs[index].document;
                if old.revision() != source.document.revision()
                    || old.path != source.document.path
                    || old.is_dirty() != source.document.is_dirty()
                    || old.bom != source.document.bom
                {
                    self.tabs[index].document = source.document.review_snapshot();
                    self.redraw = true;
                }
            }
        }
    }
    fn source_tab(&mut self) {
        if let Some(id) = self.tabs[self.active].review_source
            && let Some(index) = self
                .tabs
                .iter()
                .position(|t| t.review_source.is_none() && t.document.id == id)
        {
            self.active = index;
        }
        self.focus = Focus::Editor;
    }
    fn open_review_tab(&mut self) {
        self.source_tab();
        let id = self.document().id;
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.review_source == Some(id))
        {
            self.active = index;
        } else {
            let mut tab = Tab::new(self.document().review_snapshot());
            tab.ai_action = self.tabs[self.active].ai_action.clone();
            tab.untitled = self.tabs[self.active].untitled;
            tab.review_source = Some(id);
            tab.review.outline_visible = true;
            tab.review.mode = ViewMode::Review;
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
        }
        self.focus = Focus::Editor;
    }
    fn apply_settings(&mut self, selected: Locale) {
        self.config.language = selected;
        self.preferences.request(selected);
        self.message = if self.preferences.path.is_some() {
            "Saving settings…"
        } else {
            "Settings applied for this session"
        }
        .into();
    }
    fn picker_action(&mut self, action: PickerAction, picker: Box<FilePicker>) {
        match action {
            PickerAction::Keep => self.modal = Some(Modal::FilePicker(picker)),
            PickerAction::Cancel => self.message = "Cancelled".into(),
            PickerAction::Open(path) => {
                self.start_open(path, None, self.config.large_file_warning_bytes)
            }
        }
    }

    /// Why an action cannot run now, shared by the dispatcher and F2.
    pub fn unavailable(&self, command: &str) -> Option<&'static str> {
        const CHANGES: &[&str] = &[
            "save",
            "save-as",
            "close",
            "reload",
            "undo",
            "redo",
            "cut",
            "paste",
            "replace",
            "find",
            "find-next",
            "duplicate",
            "move-line-up",
            "move-line-down",
            "indent",
            "outdent",
            "delete-word-left",
            "delete-word-right",
        ];
        const REVIEW_BLOCKED: &[&str] = &[
            "save",
            "save-as",
            "reload",
            "find",
            "find-next",
            "replace",
            "undo",
            "redo",
            "cut",
            "paste",
            "duplicate",
            "move-line-up",
            "move-line-down",
            "indent",
            "outdent",
            "delete-word-left",
            "delete-word-right",
        ];
        const BUSY_BLOCKED: &[&str] = &[
            "save",
            "save-as",
            "open",
            "reload",
            "close",
            "quit",
            "find",
            "find-next",
            "replace",
        ];
        const EDITS: &[&str] = &[
            "undo",
            "redo",
            "cut",
            "paste",
            "duplicate",
            "move-line-up",
            "move-line-down",
            "indent",
            "outdent",
            "delete-word-left",
            "delete-word-right",
        ];
        let tab = &self.tabs[self.active];
        if (command == "quit" && !self.agent_leases.is_empty())
            || (tab.ai_action.is_some() && CHANGES.contains(&command))
        {
            return Some("AI is working in this tab; read-only until it finishes");
        }
        if tab.review_source.is_some() && REVIEW_BLOCKED.contains(&command) {
            return Some("Review tab is read-only; return to source to edit or save");
        }
        if self.busy() && BUSY_BLOCKED.contains(&command) {
            return Some("Background operation in progress; retry when complete");
        }
        if tab.document.readonly && EDITS.contains(&command) {
            return Some("Document is read-only");
        }
        None
    }

    pub fn command(&mut self, command: &str) {
        let command = crate::commands::canonical(command);
        if let Some(reason) = self.unavailable(command) {
            self.message = reason.into();
            self.redraw = true;
            return;
        }
        match command {
            "language" | "language-vi" | "language-en" => {
                self.config.language = match command { "language-vi" => crate::i18n::Locale::Vi, "language-en" => crate::i18n::Locale::En, _ => self.config.language.toggle() };
            }
            "files" => self.open_project(crate::project::Mode::Files),
            "project-search" => self.open_project(crate::project::Mode::Contents),
            "goto" => { self.source_tab(); self.tabs[self.active].review.mode = ViewMode::Source; self.prompt(InputKind::GoToLine); }
            "focus-mode" => { self.focus_mode = !self.focus_mode; self.focus = Focus::Editor; }
            "settings" => self.modal = Some(Modal::Settings { selected: self.config.language }),
            "tabs" => self.modal = Some(Modal::Tabs(TabManager::new(&self.tabs, self.active, self.config.language))),
            "info" | "file-info" => self.modal = Some(Modal::FileInfo(FileInfo::new(self.document(), self.config.language))),
            "preview" => self.modal = Some(Modal::ReviewOptions { selected: 0 }),
            "review" | "review-tab" => self.open_review_tab(),
            "source" => { self.source_tab(); self.tabs[self.active].review.mode = ViewMode::Source; },
            "split" => { self.source_tab(); self.tabs[self.active].review.mode = ViewMode::Split; self.focus = Focus::Editor; },
            "new" => self.new_blank(),
            "open-folder" => self.prompt(InputKind::Folder),
            "explorer" => { if self.focus_mode { self.focus_mode = false; self.explorer_visible = true; } else { self.explorer_visible = !self.explorer_visible; } self.focus = if self.explorer_visible { Focus::Explorer } else { Focus::Editor }; }
            "focus" => { self.focus_mode = false; self.explorer_visible = true; self.focus = if self.focus == Focus::Editor { Focus::Explorer } else { Focus::Editor }; }
            "refresh" => self.explorer.refresh(),
            "open" => {
                let root = self.document().path.as_ref().and_then(|p| p.parent()).filter(|p| !p.as_os_str().is_empty()).map(PathBuf::from).unwrap_or_else(|| self.explorer.root.clone());
                self.modal = Some(Modal::FilePicker(Box::new(FilePicker::new(root))));
            }
            "open-path" => self.prompt(InputKind::Open),
            "save-as" => self.prompt(InputKind::SaveAs),
            "save" => {
                if let Some(path) = self.document().path.clone() { self.start_save(path); } else { self.prompt(InputKind::SaveAs); }
            }
            "close" if self.tabs[self.active].review_source.is_none() && self.document().is_dirty() => self.modal = Some(Modal::Confirm { selected: false, action: Action::Close, message: "Discard unsaved changes in this tab? Y discard / N cancel (Ctrl+S saves after cancel)".into() }),
            "close" => self.close(),
            "quit" if self.tabs.iter().any(|t| t.review_source.is_none() && t.document.is_dirty()) => self.modal = Some(Modal::Confirm { selected: false, action: Action::Quit, message: "Discard ALL unsaved tabs and quit? Y discard / N cancel (save tabs after cancel)".into() }),
            "quit" => self.request_quit(),
            "reload" if self.document().path.is_some() => self.modal = Some(Modal::Confirm { selected: false, action: Action::Reload, message: "Reload from disk and discard this tab's edits/history? Y reload / N cancel".into() }),
            "reload" => self.message = "Untitled buffer has no file to reload".into(),
            "next" => { self.active = (self.active + 1) % self.tabs.len(); self.focus = Focus::Editor; }
            "prev" => { self.active = (self.active + self.tabs.len() - 1) % self.tabs.len(); self.focus = Focus::Editor; }
            "help" => {
                self.help_scroll = 0;
                self.modal = Some(Modal::Help);
            }
            "find" => self.open_search(),
            "find-next" => self.find_next(),
            "replace" => self.open_search(),
            "undo" => self.edit(|d| d.undo().map(|_| ())),
            "redo" => self.edit(|d| d.redo().map(|_| ())),
            "select-all" => self.document_mut().select_all(),
            "copy" => self.copy_selection(),
            "cut" if self.document().selection().is_empty() => {
                let mut text = None;
                self.edit(|d| d.cut_line().map(|cut| text = Some(cut)));
                if let Some(text) = text { self.copy_text(text); }
            }
            "cut" => { self.copy_selection(); self.edit(|d| d.insert("")); }
            "paste" => self.paste(),
            "commands" => self.prompt(InputKind::Command),
            "outline" => {
                let review = &mut self.tabs[self.active].review;
                review.outline_visible = !review.outline_visible;
                review.outline_focus = false;
            }
            "duplicate" => self.edit(Document::duplicate),
            "move-line-up" => self.edit(|d| d.move_lines(false)),
            "move-line-down" => self.edit(|d| d.move_lines(true)),
            "indent" | "outdent" => {
                let tab = self.config.tab_width;
                let unit = self.document().indent_unit(tab);
                if command == "indent" && !self.document().selection_spans_lines() {
                    self.edit(|d| d.type_text(&unit));
                } else {
                    self.edit(|d| d.indent_lines(&unit, tab, command == "outdent"));
                }
            }
            "word-left" | "word-right" | "select-word-left" | "select-word-right" => {
                let movement = if command.ends_with("left") { Movement::WordLeft } else { Movement::WordRight };
                self.document_mut().move_cursor(movement, command.starts_with("select"));
                self.tabs[self.active].follow_cursor = true;
            }
            "delete-word-left" => self.edit(Document::delete_word_left),
            "delete-word-right" => self.edit(Document::delete_word_right),
            _ => self.message = format!("Unknown command: {command}. F1 lists commands."),
        }
        self.redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn function_keys_follow_previous_next_review_settings_and_outline_mapping() {
        let mut app = app();
        app.command("new");
        app.command("new");
        assert_eq!(app.active, 2);
        app.handle(key(KeyCode::F(6)));
        assert_eq!(app.active, 1);
        app.handle(key(KeyCode::F(7)));
        assert_eq!(app.active, 2);
        app.handle(key(KeyCode::F(8)));
        assert!(matches!(app.modal, Some(Modal::ReviewOptions { .. })));
        app.handle(key(KeyCode::F(8)));
        assert!(app.modal.is_none());
        app.handle(key(KeyCode::F(10)));
        assert!(matches!(app.modal, Some(Modal::Settings { .. })));
        app.handle(key(KeyCode::F(10)));
        assert!(app.modal.is_none());
        app.tabs[app.active].review.mode = ViewMode::Review;
        let before = app.tabs[app.active].review.outline_visible;
        app.handle(Event::Key(KeyEvent::new(
            KeyCode::Char('m'),
            KeyModifiers::ALT,
        )));
        assert_eq!(app.tabs[app.active].review.outline_visible, !before);
        assert_eq!(app.active, 2);
    }
    #[test]
    fn project_navigation_keeps_dirty_buffer_and_stale_disk_results_safe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.txt");
        std::fs::write(&path, "first\nneedle e\u{301} 👩‍💻\n").unwrap();
        let document = guinea_io::open(&path, false, u64::MAX).unwrap();
        let found = crate::project::Found {
            path: document.path.clone().unwrap(),
            label: "source.txt".into(),
            line: 1,
            range: Some(6..12),
            hash: document.fingerprint.as_ref().map(|f| f.sha256),
            preview: "needle".into(),
        };
        let mut a = app();
        a.tabs[0] = Tab::new(document);
        a.open_project_result(found.clone());
        assert_eq!(a.document().selected_text(), "needle");
        a.document_mut().insert("changed").unwrap();
        let text = a.document().text();
        let selection = a.document().selection();
        a.open_project_result(found.clone());
        assert_eq!(a.document().text(), text);
        assert_eq!(a.document().selection(), selection);
        assert!(a.message.contains("Search result changed"));
        let mut stale = found;
        stale.hash = Some([0; 32]);
        a.document_mut().undo().unwrap();
        a.open_project_result(stale);
        assert!(a.message.contains("Search result changed"));
    }
    #[test]
    fn goto_uses_grapheme_columns_and_rejects_out_of_range_without_moving() {
        let mut a = app();
        a.handle(Event::Paste("e\u{301}👩‍💻\r\nnext".into()));
        a.submit(InputKind::GoToLine, "1:2".into());
        drain_jobs(&mut a);
        assert_eq!(a.document().cursor(), 2);
        let original = a.document().text();
        for value in ["0", "1:0", "999:1", "1:99", "1:2:3", "invalid"] {
            a.submit(InputKind::GoToLine, value.into());
            drain_jobs(&mut a);
            assert_eq!(a.document().cursor(), 2, "{value}");
            assert_eq!(a.document().text(), original);
        }
        a.submit(InputKind::GoToLine, "1:3".into());
        drain_jobs(&mut a);
        assert_eq!(a.document().cursor(), 5);
    }

    #[test]
    fn project_shortcuts_palette_completion_and_focus_mode_preserve_text() {
        let mut a = app();
        a.handle(Event::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::CONTROL,
        )));
        assert!(matches!(a.modal, Some(Modal::Project(_))));
        a.handle(key(KeyCode::Esc));
        a.handle(Event::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        )));
        assert!(matches!(a.modal, Some(Modal::Tabs(_))));
        a.handle(key(KeyCode::Esc));
        a.handle(key(KeyCode::F(2)));
        a.handle(Event::Paste("project-se".into()));
        a.handle(key(KeyCode::Tab));
        a.handle(key(KeyCode::Enter));
        assert!(
            matches!(&a.modal,Some(Modal::Project(p)) if p.mode==crate::project::Mode::Contents)
        );
        a.handle(key(KeyCode::Esc));
        a.handle(Event::Paste("draft".into()));
        a.handle(key(KeyCode::F(11)));
        assert!(a.focus_mode);
        a.handle(key(KeyCode::F(11)));
        assert!(!a.focus_mode);
        assert_eq!(a.document().text(), "draft");
        a.command("focus-mode");
        a.command("focus");
        assert!(!a.focus_mode);
        assert!(a.explorer_visible);
        assert_eq!(a.focus, Focus::Explorer);
    }

    #[test]
    fn tab_keys_leave_search_safely_and_do_not_bypass_dirty_confirmation() {
        let mut a = app();
        a.document_mut().insert("first").unwrap();
        a.command("new");
        a.document_mut().insert("second").unwrap();
        a.command("find");
        a.handle(Event::Paste("query".into()));
        a.handle(Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT)));
        assert!(a.modal.is_none());
        assert_eq!(a.active, 0);
        assert_eq!(a.document().text(), "first");
        a.handle(key(KeyCode::F(12)));
        assert!(matches!(a.modal, Some(Modal::Tabs(_))));
        a.handle(key(KeyCode::Tab));
        a.handle(key(KeyCode::Enter));
        assert_eq!(a.active, 1);
        assert_eq!(a.document().text(), "second");
        a.command("close");
        assert!(matches!(a.modal, Some(Modal::Confirm { .. })));
        a.handle(Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT)));
        a.handle(key(KeyCode::F(12)));
        assert_eq!(a.active, 1);
        assert!(matches!(a.modal, Some(Modal::Confirm { .. })));
        a.handle(key(KeyCode::Enter)); // Safe default: keep working.
        assert_eq!(a.tabs.len(), 2);
        assert_eq!(a.document().text(), "second");
    }

    #[test]
    fn closing_inactive_dirty_and_linked_tabs_preserves_selection_and_source() {
        let mut a = app();
        a.handle(Event::Paste("# draft".into()));
        a.command("review-tab");
        a.command("new");
        let active = a.document().id;
        let title = a.tabs[a.active].title(Locale::En);
        a.request_close_tab(0);
        assert!(matches!(
            a.modal,
            Some(Modal::Confirm {
                action: Action::CloseTab(0),
                ..
            })
        ));
        a.handle(key(KeyCode::Char('n')));
        assert_eq!(a.tabs.len(), 3);
        assert_eq!(a.document().id, active);
        assert_eq!(a.tabs[0].document.text(), "# draft");
        a.request_close_tab(1);
        assert_eq!(a.tabs.len(), 2);
        assert_eq!(a.document().id, active);
        assert_eq!(a.tabs[0].document.text(), "# draft");
        a.request_close_tab(0);
        a.handle(key(KeyCode::Char('y')));
        assert_eq!(a.tabs.len(), 1);
        assert_eq!(a.document().id, active);
        a.request_close_tab(0);
        assert_eq!(a.tabs.len(), 1);
        assert_eq!(a.document().text(), "");
        assert_ne!(a.tabs[0].title(Locale::En), title);
    }

    #[test]
    fn tab_manager_filters_duplicate_paths_and_cancels_without_editing() {
        let mut a = app();
        a.document_mut().path = Some("alpha/Việt.md".into());
        a.handle(Event::Paste("source".into()));
        a.command("new");
        a.document_mut().path = Some("beta/Việt.md".into());
        let active = a.document().id;
        a.handle(Event::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        )));
        a.handle(Event::Paste("alpha/Việt".into()));
        let Some(Modal::Tabs(manager)) = &a.modal else {
            panic!("missing tab manager")
        };
        assert_eq!(manager.current(), Some(0));
        a.handle(key(KeyCode::Esc));
        assert_eq!(a.document().id, active);
        assert_eq!(a.tabs[0].document.text(), "source");
        a.command("tabs");
        a.handle(Event::Paste("no match".into()));
        a.handle(key(KeyCode::Enter));
        assert!(matches!(a.modal, Some(Modal::Tabs(_))));
        a.handle(Event::Key(KeyEvent::new(
            KeyCode::Char('u'),
            KeyModifiers::CONTROL,
        )));
        a.handle(key(KeyCode::Home));
        a.handle(key(KeyCode::Enter));
        assert_eq!(a.active, 0);
        a.command("undo");
        assert_eq!(a.document().text(), "");
        a.handle(Event::Key(KeyEvent::new(
            KeyCode::PageDown,
            KeyModifiers::CONTROL,
        )));
        assert_eq!(a.active, 1);
        a.handle(Event::Key(KeyEvent::new(
            KeyCode::Char('t'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(a.tabs.len(), 3);
        assert!(a.document().path.is_none());
    }

    #[test]
    fn tab_manager_refreshes_metadata_after_pending_save_without_losing_filter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("draft.txt");
        std::fs::write(&path, "original").unwrap();
        let mut a = App::new(vec![path.clone()], false, Config::default());
        drain_jobs(&mut a);
        a.handle(Event::Paste("new ".into()));
        a.command("save");
        a.command("tabs");
        a.handle(Event::Paste("draft".into()));
        drain_jobs(&mut a);
        let Some(Modal::Tabs(manager)) = &a.modal else {
            panic!("missing manager after save")
        };
        assert_eq!(manager.query, "draft");
        assert_eq!(manager.current(), Some(0));
        assert!(!manager.entries[0].dirty);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new original");
    }

    #[test]
    fn f8_opens_settings_cancel_does_not_change_language_and_apply_is_explicit() {
        let mut a = app();
        let initial = a.config.language;
        a.handle(key(KeyCode::F(10)));
        assert!(matches!(a.modal, Some(Modal::Settings { .. })));
        a.handle(key(KeyCode::Right));
        assert_eq!(a.config.language, initial);
        a.handle(key(KeyCode::Esc));
        assert_eq!(a.config.language, initial);
        a.handle(key(KeyCode::F(10)));
        a.handle(key(KeyCode::Char('v')));
        a.handle(key(KeyCode::Enter));
        assert_eq!(a.config.language, Locale::Vi);
        assert!(a.modal.is_none());
    }

    #[test]
    fn review_tab_tracks_source_and_cannot_replace_editable_history() {
        let mut a = app();
        a.document_mut().path = Some("note.md".into());
        a.handle(Event::Paste("# Source".into()));
        a.command("review-tab");
        assert_eq!(a.tabs.len(), 2);
        assert_eq!(a.tabs[a.active].review_source, Some(a.tabs[0].document.id));
        let original = a.tabs[0].document.text();
        for cmd in [
            "save",
            "find-next",
            "replace",
            "undo",
            "redo",
            "paste",
            "cut",
            "reload",
        ] {
            a.command(cmd);
            assert!(!a.busy());
        }
        a.handle(Event::Paste("never edit".into()));
        assert_eq!(a.tabs[0].document.text(), original);
        a.command("source");
        a.handle(Event::Paste(" updated".into()));
        a.poll_jobs();
        assert_eq!(a.tabs[1].document.text(), a.tabs[0].document.text());
        a.command("review-tab");
        assert_eq!(a.tabs.len(), 2);
        a.command("close");
        assert_eq!(a.tabs.len(), 1);
        assert!(a.document().is_dirty());
        a.command("undo");
        assert_eq!(a.document().text(), original);
    }

    #[test]
    fn review_chooser_split_and_dirty_source_close_keep_document_safe() {
        let mut a = app();
        a.handle(Event::Paste("source".into()));
        a.handle(key(KeyCode::F(8)));
        assert!(matches!(a.modal, Some(Modal::ReviewOptions { .. })));
        a.handle(key(KeyCode::Char('1')));
        assert_eq!(a.tabs[a.active].review.mode, ViewMode::Split);
        a.handle(key(KeyCode::F(8)));
        a.handle(key(KeyCode::Char('2')));
        assert_eq!(a.tabs.len(), 2);
        a.command("source");
        a.command("close");
        assert!(matches!(a.modal, Some(Modal::Confirm { .. })));
        a.handle(key(KeyCode::Char('n')));
        assert_eq!(a.tabs.len(), 2);
        a.command("close");
        a.handle(key(KeyCode::Char('y')));
        assert_eq!(a.tabs.len(), 1);
        assert!(a.tabs[0].review_source.is_none());
        assert_eq!(a.document().text(), "");
    }

    #[test]
    fn reload_advances_preview_revision_even_for_saved_unedited_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "# Original").unwrap();
        let mut a = App::new(vec![path.clone()], false, Config::default());
        drain_jobs(&mut a);
        a.command("review-tab");
        let old = a.document().revision();
        a.command("source");
        std::fs::write(&path, "# Changed").unwrap();
        a.command("reload");
        a.handle(key(KeyCode::Char('y')));
        drain_jobs(&mut a);
        assert!(a.document().revision() > old);
        assert_eq!(a.tabs[1].document.text(), "# Changed");
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    fn keym(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn keymap_v1_editing_commands_are_single_undo_steps() {
        let mut a = app();
        a.handle(Event::Paste("    alpha beta\r\ngamma".into()));
        a.command("undo");
        a.handle(Event::Paste("    alpha beta\r\ngamma".into()));
        let _ = a.document_mut().select(14, 14);
        // Enter keeps indentation; newline + indent are one Undo step. A new
        // buffer inserts LF; pasted CRLF bytes stay as they are.
        a.handle(key(KeyCode::Enter));
        assert_eq!(a.document().text(), "    alpha beta\n    \r\ngamma");
        a.command("undo");
        assert_eq!(a.document().text(), "    alpha beta\r\ngamma");
        // A newline followed by more keys in one burst is a paste: no indent.
        let _ = a.document_mut().select(14, 14);
        a.begin_input();
        a.handle(key(KeyCode::Enter));
        for c in "  z".chars() {
            a.handle(key(KeyCode::Char(c)));
        }
        a.end_input(true);
        assert_eq!(a.document().text(), "    alpha beta\n  z\r\ngamma");
        a.command("undo");
        assert_eq!(
            a.document().text(),
            "    alpha beta\r\ngamma",
            "burst is one step"
        );
        // Word motion and word deletion.
        let _ = a.document_mut().select(14, 14);
        a.handle(keym(KeyCode::Left, KeyModifiers::CONTROL));
        assert_eq!(a.document().cursor(), 10);
        a.handle(keym(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(a.document().text(), "    beta\r\ngamma");
        a.command("undo");
        // Ctrl+X without a selection cuts the line; Ctrl+V restores it.
        let end = a.document().buffer().len_chars();
        let _ = a.document_mut().select(end, end);
        a.handle(keym(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(a.document().text(), "    alpha beta");
        a.handle(keym(KeyCode::Char('v'), KeyModifiers::CONTROL));
        assert_eq!(a.document().text(), "    alpha betagamma\n");
        a.command("undo");
        a.command("undo");
        assert_eq!(a.document().text(), "    alpha beta\r\ngamma");
        // Duplicate, move and block indent.
        let _ = a.document_mut().select(0, 0);
        a.handle(keym(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(
            a.document().text(),
            "    alpha beta\n    alpha beta\r\ngamma"
        );
        a.command("undo");
        let end = a.document().buffer().len_chars();
        let _ = a.document_mut().select(end, end);
        a.handle(keym(KeyCode::Up, KeyModifiers::ALT));
        assert_eq!(a.document().text(), "gamma\r\n    alpha beta");
        a.command("undo");
        a.command("select-all");
        a.handle(key(KeyCode::BackTab));
        assert_eq!(a.document().text(), "alpha beta\r\ngamma");
        a.handle(key(KeyCode::Tab));
        // No indented line remains to detect spaces from, so Tab uses \t.
        assert_eq!(a.document().text(), "\talpha beta\r\n\tgamma");
        assert!(a.unavailable("duplicate").is_none());
        a.tabs[a.active].document.readonly = true;
        assert_eq!(a.unavailable("duplicate"), Some("Document is read-only"));
        a.handle(keym(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(a.message, "Document is read-only");
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
        a.handle(Event::Paste("typed during save ".into()));
        drain_jobs(&mut a);
        assert_eq!(std::fs::read_to_string(&path_b).unwrap(), "changed 👩‍💻\n");
        a.command("prev");
        assert_eq!(a.document().text(), "Việt\r\n");
        a.search = "Việt".into();
        a.find_next();
        drain_jobs(&mut a);
        assert_eq!(a.document().selected_text(), "Việt");
        a.start_compute("Replacing", |d| {
            d.replace_all("Việt", "Vietnam").map(|_| "Replaced".into())
        });
        drain_jobs(&mut a);
        assert_eq!(a.document().text(), "Vietnam\r\n");
        std::fs::write(&path_a, "external").unwrap();
        a.command("save");
        drain_jobs(&mut a);
        assert!(a.document().is_dirty());
        assert!(a.message.contains("changed outside"));
        assert_eq!(std::fs::read_to_string(path_a).unwrap(), "external");
        assert_eq!(a.tabs[1].document.text(), "changed typed during save 👩‍💻\n");
        assert!(a.tabs[1].document.is_dirty());
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
        a.handle(key(KeyCode::F(7)));
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
