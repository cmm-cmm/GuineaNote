//! Read-only, lazy directory browsing. One background listing at a time; never
//! recurse into a symlink or scan the whole workspace on startup.
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

const ENTRY_LIMIT: usize = 10_000;

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub directory: bool,
    pub symlink: bool,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub entry: Entry,
    pub depth: usize,
    pub guides: Vec<bool>,
    pub last: bool,
}

struct Listing {
    path: PathBuf,
    entries: Vec<Entry>,
    truncated: bool,
}

fn list(path: &Path) -> io::Result<Listing> {
    list_with_limit(path, ENTRY_LIMIT)
}

fn list_with_limit(path: &Path, limit: usize) -> io::Result<Listing> {
    let path = path.canonicalize()?;
    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        if entries.len() == limit {
            truncated = true;
            break;
        }
        let kind = entry.file_type()?;
        entries.push(Entry {
            path: entry.path(),
            name: entry.file_name().to_string_lossy().into_owned(),
            directory: kind.is_dir() && !kind.is_symlink(),
            symlink: kind.is_symlink(),
        });
    }
    entries.sort_by(|a, b| {
        (!a.directory, a.name.to_lowercase(), &a.path).cmp(&(
            !b.directory,
            b.name.to_lowercase(),
            &b.path,
        ))
    });
    Ok(Listing {
        path,
        entries,
        truncated,
    })
}

struct Scan {
    generation: u64,
    path: PathBuf,
    root: bool,
    result: io::Result<Listing>,
}

pub struct Explorer {
    pub root: PathBuf,
    pub rows: Vec<Row>,
    pub selected: usize,
    pub top: usize,
    pub status: String,
    pub viewport_height: usize,
    expanded: BTreeSet<PathBuf>,
    cache: BTreeMap<PathBuf, Vec<Entry>>,
    generation: u64,
    queued: VecDeque<(PathBuf, bool)>,
    pending: Option<Receiver<Scan>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Explorer {
    pub fn new(root: PathBuf) -> Self {
        let mut result = Self {
            root: PathBuf::new(),
            rows: Vec::new(),
            selected: 0,
            top: 0,
            status: String::new(),
            viewport_height: 20,
            expanded: BTreeSet::new(),
            cache: BTreeMap::new(),
            generation: 0,
            queued: VecDeque::new(),
            pending: None,
            worker: None,
        };
        result.open(root);
        result
    }

    pub fn open(&mut self, root: PathBuf) {
        self.generation += 1;
        self.root = root;
        self.rows.clear();
        self.cache.clear();
        self.expanded.clear();
        self.queued.clear();
        self.selected = 0;
        self.top = 0;
        self.status = "Loading workspace…".into();
        self.queued.push_back((self.root.clone(), true));
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Workspace".into())
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some() || !self.queued.is_empty()
    }
    pub fn scanning(&self) -> bool {
        self.pending.is_some()
    }
    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded.contains(path)
    }
    pub fn current(&self) -> Option<&Row> {
        self.rows.get(self.selected)
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(scan) => {
                    self.pending = None;
                    self.join();
                    if scan.generation == self.generation {
                        match scan.result {
                            Ok(listing) => {
                                if scan.root {
                                    self.root = listing.path.clone();
                                    self.expanded.insert(listing.path.clone());
                                }
                                self.cache.insert(listing.path, listing.entries);
                                self.status = if listing.truncated {
                                    "10,000-entry limit reached; open a narrower folder".into()
                                } else {
                                    "↑↓ browse · Enter open · F5 refresh".into()
                                };
                                self.rebuild();
                            }
                            Err(error) => {
                                self.status =
                                    format!("Cannot read {}: {error}", scan.path.display());
                            }
                        }
                        changed = true;
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.join();
                    self.status = "Directory worker stopped; F5 to retry".into();
                    changed = true;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_none()
            && let Some((path, root)) = self.queued.pop_front()
        {
            let generation = self.generation;
            let (sender, receiver) = mpsc::channel();
            self.pending = Some(receiver);
            self.worker = Some(thread::spawn(move || {
                let result = list(&path);
                let _ = sender.send(Scan {
                    generation,
                    path,
                    root,
                    result,
                });
            }));
            changed = true;
        }
        changed
    }

    fn join(&mut self) {
        if let Some(worker) = self.worker.take()
            && let Err(panic) = worker.join()
        {
            std::panic::resume_unwind(panic);
        }
    }

    fn append(&self, path: &Path, guides: &[bool], rows: &mut Vec<Row>) {
        let Some(children) = self.cache.get(path) else {
            return;
        };
        for (index, entry) in children.iter().enumerate() {
            let last = index + 1 == children.len();
            rows.push(Row {
                entry: entry.clone(),
                depth: guides.len(),
                guides: guides.to_vec(),
                last,
            });
            if entry.directory && self.expanded.contains(&entry.path) {
                let mut next = guides.to_vec();
                next.push(!last);
                self.append(&entry.path, &next, rows);
            }
        }
    }

    fn rebuild(&mut self) {
        let selected = self.current().map(|row| row.entry.path.clone());
        let mut rows = Vec::new();
        self.append(&self.root, &[], &mut rows);
        self.rows = rows;
        self.selected = selected
            .and_then(|path| self.rows.iter().position(|r| r.entry.path == path))
            .unwrap_or(self.selected)
            .min(self.rows.len().saturating_sub(1));
    }

    pub fn move_by(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(1));
    }

    pub fn expand(&mut self) {
        let Some(row) = self.current().cloned() else {
            return;
        };
        if !row.entry.directory {
            return;
        }
        if self.expanded.insert(row.entry.path.clone()) {
            if !self.cache.contains_key(&row.entry.path) {
                self.queued.push_back((row.entry.path, false));
            }
            self.rebuild();
        } else if self
            .rows
            .get(self.selected + 1)
            .is_some_and(|next| next.depth > row.depth)
        {
            self.move_by(1);
        }
    }

    pub fn collapse(&mut self) {
        let Some(row) = self.current().cloned() else {
            return;
        };
        if self.expanded.remove(&row.entry.path) {
            self.rebuild();
        } else if let Some(parent) = row.entry.path.parent()
            && let Some(index) = self.rows.iter().position(|r| r.entry.path == parent)
        {
            self.selected = index;
        }
    }

    pub fn toggle(&mut self) {
        if self
            .current()
            .is_some_and(|r| self.is_expanded(&r.entry.path))
        {
            self.collapse();
        } else {
            self.expand();
        }
    }

    pub fn refresh(&mut self) {
        let path = self
            .current()
            .filter(|r| r.entry.directory)
            .map(|r| r.entry.path.clone())
            .unwrap_or_else(|| self.root.clone());
        if !self.queued.iter().any(|(queued, _)| queued == &path) {
            self.queued.push_back((path.clone(), path == self.root));
            self.status = "Refreshing…".into();
        }
    }

    pub fn select_path(&mut self, path: &Path) {
        if let Some(index) = self.rows.iter().position(|row| row.entry.path == path) {
            self.selected = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drain(tree: &mut Explorer) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while tree.busy() {
            tree.poll();
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn listing_limit_is_explicit_and_file_content_is_never_read() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a", "b", "c", "d"] {
            fs::write(dir.path().join(name), [0xff, 0x00]).unwrap();
        }
        let listing = list_with_limit(dir.path(), 3).unwrap();
        assert!(listing.truncated);
        assert_eq!(listing.entries.len(), 3);
        assert!(listing.entries.iter().all(|entry| !entry.directory));
    }

    #[cfg(unix)]
    #[test]
    fn directory_symlinks_are_listed_without_following_them() {
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join("loop")).unwrap();
        let mut tree = Explorer::new(dir.path().into());
        drain(&mut tree);
        assert!(tree.rows[0].entry.symlink);
        assert!(!tree.rows[0].entry.directory);
        tree.expand();
        drain(&mut tree);
        assert_eq!(tree.rows.len(), 1);
    }

    #[test]
    fn lazy_tree_sorts_expands_collapses_and_refreshes() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/Việt.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("README.md"), "readme").unwrap();
        let mut tree = Explorer::new(dir.path().into());
        drain(&mut tree);
        assert_eq!(tree.rows.len(), 2);
        assert_eq!(tree.rows[0].entry.name, "src");
        tree.expand();
        drain(&mut tree);
        assert_eq!(tree.rows.len(), 3);
        assert_eq!(tree.rows[1].entry.name, "Việt.rs");
        tree.collapse();
        assert_eq!(tree.rows.len(), 2);
        fs::write(dir.path().join("new.txt"), "new").unwrap();
        tree.selected = 1;
        tree.refresh();
        drain(&mut tree);
        assert_eq!(tree.rows.len(), 3);
    }

    #[test]
    fn root_change_discards_old_results_and_errors_do_not_panic() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        fs::write(a.path().join("old"), "old").unwrap();
        fs::write(b.path().join("new"), "new").unwrap();
        let mut tree = Explorer::new(a.path().into());
        tree.poll();
        tree.open(b.path().into());
        drain(&mut tree);
        assert_eq!(tree.rows.len(), 1);
        assert_eq!(tree.rows[0].entry.name, "new");
        tree.open(b.path().join("missing"));
        drain(&mut tree);
        assert!(tree.status.contains("Cannot read"));
        assert!(tree.rows.is_empty());
    }
}
