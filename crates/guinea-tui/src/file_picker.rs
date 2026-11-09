//! Shallow asynchronous file picker: cached names, bounded lists, no recursive
//! scan, no filesystem work in keyboard/filter handlers.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
};
use unicode_segmentation::UnicodeSegmentation;

const LIMIT: usize = 10_000;
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    folded: String,
    pub directory: bool,
    pub symlink: bool,
}
enum Loaded {
    Directory {
        root: PathBuf,
        entries: Vec<Entry>,
        capped: bool,
    },
    File(PathBuf),
}
#[derive(Default)]
pub struct Hits {
    pub list: Rect,
    pub parent: Rect,
    pub open: Rect,
    pub cancel: Rect,
}
pub struct FilePicker {
    pub root: PathBuf,
    pub query: String,
    pub entries: Vec<Entry>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub top: usize,
    pub height: usize,
    pub status: String,
    pub capped: bool,
    pub hits: Hits,
    pending: Option<Receiver<Result<Loaded, String>>>,
    activate_when_ready: bool,
}
pub enum Action {
    Keep,
    Cancel,
    Open(PathBuf),
}

fn scan(path: &Path) -> io::Result<Loaded> {
    let root = path.canonicalize()?;
    let mut entries = Vec::new();
    let mut capped = false;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if entries.len() >= LIMIT {
            capped = true;
            break;
        }
        let kind = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(Entry {
            path: entry.path(),
            folded: name.to_lowercase(),
            name,
            directory: kind.is_dir() && !kind.is_symlink(),
            symlink: kind.is_symlink(),
        });
    }
    entries.sort_by(|a, b| {
        (!a.directory, &a.folded, &a.name).cmp(&(!b.directory, &b.folded, &b.name))
    });
    Ok(Loaded::Directory {
        root,
        entries,
        capped,
    })
}
impl FilePicker {
    pub fn new(root: PathBuf) -> Self {
        let mut picker = Self {
            root,
            query: String::new(),
            entries: vec![],
            filtered: vec![],
            selected: 0,
            top: 0,
            height: 12,
            status: String::new(),
            capped: false,
            hits: Hits::default(),
            pending: None,
            activate_when_ready: false,
        };
        picker.navigate(picker.root.clone());
        picker
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn navigate(&mut self, path: PathBuf) {
        if self.busy() {
            return;
        }
        self.query.clear();
        self.refilter();
        self.status = "Loading files…".into();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(scan(&path).map_err(|e| e.to_string()));
        });
    }
    pub fn poll(&mut self) -> (bool, Option<PathBuf>) {
        let result = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("File list worker stopped".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        let Some(result) = result else {
            return (false, None);
        };
        self.pending = None;
        match result {
            Ok(Loaded::Directory {
                root,
                entries,
                capped,
            }) => {
                self.root = root;
                self.entries = entries;
                self.capped = capped;
                self.refilter();
                self.status.clear();
            }
            Ok(Loaded::File(path)) => return (true, Some(path)),
            Err(error) => self.status = format!("Cannot read: {error}"),
        }
        if self.activate_when_ready {
            self.activate_when_ready = false;
            if let Action::Open(path) = self.activate() {
                return (true, Some(path));
            }
        }
        (true, None)
    }
    pub fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.folded.contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.top = 0;
    }
    pub fn paste(&mut self, text: &str) {
        self.activate_when_ready = false;
        self.query.push_str(&text.replace(['\r', '\n'], ""));
        self.refilter();
    }
    pub fn move_by(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.filtered.len().saturating_sub(1));
    }
    pub fn parent(&mut self) {
        if let Some(parent) = self.root.parent() {
            self.navigate(parent.to_path_buf());
        }
    }
    pub fn activate(&mut self) -> Action {
        if self.busy() {
            self.activate_when_ready = true;
            return Action::Keep;
        }
        if let Some(entry) = self
            .filtered
            .get(self.selected)
            .and_then(|&i| self.entries.get(i))
        {
            if entry.directory {
                self.navigate(entry.path.clone());
                return Action::Keep;
            }
            return Action::Open(entry.path.clone());
        }
        if self.query.is_empty() {
            return Action::Keep;
        }
        let path = self.root.join(&self.query);
        self.query.clear();
        self.refilter();
        self.status = "Opening path…".into();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        thread::spawn(move || {
            let result = match fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => scan(&path),
                Ok(_) => Ok(Loaded::File(path)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Loaded::File(path)),
                Err(e) => Err(e),
            };
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
        Action::Keep
    }
    pub fn key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Enter => return self.activate(),
            KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => self.parent(),
            KeyCode::Up => self.move_by(-1),
            KeyCode::Down => self.move_by(1),
            KeyCode::PageUp => self.move_by(-(self.height as isize)),
            KeyCode::PageDown => self.move_by(self.height as isize),
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.filtered.len().saturating_sub(1),
            KeyCode::Backspace if self.query.is_empty() => self.parent(),
            KeyCode::Backspace => {
                if let Some((byte, _)) = self.query.grapheme_indices(true).next_back() {
                    self.query.truncate(byte);
                    self.refilter();
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.clear();
                self.refilter();
            }
            KeyCode::Tab => {
                if let Some(entry) = self
                    .filtered
                    .get(self.selected)
                    .and_then(|&i| self.entries.get(i))
                {
                    if entry.directory {
                        self.navigate(entry.path.clone());
                    } else {
                        self.query = entry.name.clone();
                        self.refilter();
                    }
                }
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.activate_when_ready = false;
                self.query.push(ch);
                self.refilter();
            }
            _ => {}
        }
        Action::Keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drain(p: &mut FilePicker) -> Option<PathBuf> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while p.busy() {
            let (_, opened) = p.poll();
            if opened.is_some() {
                return opened;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(2));
        }
        None
    }
    #[test]
    fn browses_filters_paths_and_never_changes_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("Việt.md"), "# original").unwrap();
        let mut p = FilePicker::new(dir.path().into());
        drain(&mut p);
        assert!(p.entries[0].directory);
        p.paste("việt");
        assert_eq!(p.filtered.len(), 1);
        assert!(matches!(p.activate(), Action::Open(path) if path.ends_with("Việt.md")));
        p.query = dir.path().join("src").to_string_lossy().into_owned();
        p.refilter();
        p.activate();
        drain(&mut p);
        assert!(p.root.ends_with("src"));
        p.paste("new.txt");
        p.activate();
        assert!(drain(&mut p).unwrap().ends_with("new.txt"));
        assert!(!p.root.join("new.txt").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("Việt.md")).unwrap(),
            "# original"
        );
    }
}
