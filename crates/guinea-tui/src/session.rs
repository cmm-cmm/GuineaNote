//! Notepad++-style "remember current session": the open file tabs, each
//! caret position and the active tab are written on a normal exit and reopened
//! by a later launch without file arguments. Only paths and positions are
//! stored; unsaved text is never written here (recovery drafts own that).
use crate::app::Tab;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Upper bound on restored tabs, so a damaged file cannot queue unbounded work.
const MAX_FILES: usize = 64;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
struct Stored {
    version: u32,
    /// Index into `files` of the active tab, if it was a file tab.
    active: Option<usize>,
    files: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Entry {
    pub path: PathBuf,
    /// 0-based line.
    pub line: usize,
    /// 0-based grapheme column, matching Go to line.
    pub column: usize,
}

/// Where the session lives: `GUINEANOTE_SESSION`; next to explicit
/// preferences (PTY fixtures stay self-contained); else the per-user state
/// directory used by recovery drafts.
pub fn path(preferences: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GUINEANOTE_SESSION") {
        return Some(path.into());
    }
    if std::env::var_os("GUINEANOTE_PREFERENCES").is_some() {
        return preferences.and_then(|p| p.parent().map(|p| p.join("session.toml")));
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")));
    base.map(|p| p.join("GuineaNote/session.toml"))
}

/// Files to reopen and the positions to restore once each has opened.
#[derive(Debug, Default)]
pub struct Restore {
    positions: HashMap<PathBuf, (usize, usize)>,
    active: Option<PathBuf>,
    /// Files not yet opened (or failed); `active` is applied when it empties.
    outstanding: usize,
    /// Files queued for reopening.
    pub total: usize,
    /// Session files skipped because they no longer exist or failed to open.
    pub missing: usize,
}

impl Restore {
    /// Reads the session; missing or unreadable sessions restore nothing.
    /// Returns the existing files to queue and how many were skipped.
    pub fn load(path: &Path) -> (Vec<PathBuf>, Self, usize) {
        let stored = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str::<Stored>(&text).ok())
            .unwrap_or_default();
        Self::from_stored(stored)
    }

    fn from_stored(stored: Stored) -> (Vec<PathBuf>, Self, usize) {
        let mut restore = Self::default();
        let mut files = Vec::new();
        let mut missing = 0;
        for (index, entry) in stored.files.into_iter().take(MAX_FILES).enumerate() {
            let normalized = key(&entry.path);
            if !entry.path.is_file() || restore.positions.contains_key(&normalized) {
                missing += usize::from(!entry.path.is_file());
                continue;
            }
            if stored.active == Some(index) {
                restore.active = Some(normalized.clone());
            }
            restore
                .positions
                .insert(normalized, (entry.line, entry.column));
            files.push(entry.path);
        }
        restore.outstanding = files.len();
        restore.total = files.len();
        restore.missing = missing;
        (files, restore, missing)
    }

    pub fn is_empty(&self) -> bool {
        self.outstanding == 0
    }

    /// Position to apply to a just-opened file, if it was part of the session.
    pub fn opened(&mut self, path: &Path) -> Option<(usize, usize)> {
        let position = self.positions.remove(&key(path))?;
        self.outstanding = self.outstanding.saturating_sub(1);
        Some(position)
    }

    /// A queued session file failed to open: it is reported as skipped and
    /// no longer blocks the active-tab choice.
    pub fn failed(&mut self, path: &Path) {
        if self.positions.remove(&key(path)).is_some() {
            self.outstanding = self.outstanding.saturating_sub(1);
            self.total = self.total.saturating_sub(1);
            self.missing += 1;
        }
    }

    /// A queued session file waits for a large-file confirmation; it opens
    /// (at its start) if confirmed, so it is neither restored nor skipped.
    pub fn deferred(&mut self, path: &Path) {
        if self.positions.remove(&key(path)).is_some() {
            self.outstanding = self.outstanding.saturating_sub(1);
        }
    }

    /// The tab to activate once every session file has been handled.
    pub fn take_active(&mut self) -> Option<PathBuf> {
        if self.outstanding == 0 {
            self.active.take()
        } else {
            None
        }
    }
}

/// The comparable form of a file path: absolute and, on Windows, without the
/// verbatim `\\?\` prefix that canonicalized document paths carry, so a
/// session entry `C:\a.txt` matches the opened document `\\?\C:\a.txt`.
pub(crate) fn key(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(windows)]
    if let Some(text) = path.to_str() {
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\")
            && rest.as_bytes().get(1) == Some(&b':')
        {
            return PathBuf::from(rest);
        }
    }
    path
}

/// Snapshot of the open file tabs. `position` maps a tab to its caret line and
/// grapheme column. Untitled, recovered and review tabs are not part of it.
fn capture(tabs: &[Tab], active: usize, position: impl Fn(&Tab) -> (usize, usize)) -> Stored {
    let mut stored = Stored {
        version: 1,
        ..Stored::default()
    };
    for (index, tab) in tabs.iter().enumerate() {
        let Some(path) = tab.document.path.as_deref() else {
            continue;
        };
        if tab.review_source.is_some() || stored.files.len() == MAX_FILES {
            continue;
        }
        let path = key(path);
        // TOML strings are UTF-8; a non-UTF-8 path cannot round-trip.
        if path.to_str().is_none() || stored.files.iter().any(|e| e.path == path) {
            continue;
        }
        if index == active {
            stored.active = Some(stored.files.len());
        }
        let (line, column) = position(tab);
        stored.files.push(Entry { path, line, column });
    }
    stored
}

/// Writes the session atomically (temp file + rename in the same folder).
pub fn save(path: &Path, tabs: &[Tab], active: usize) -> io::Result<()> {
    let stored = capture(tabs, active, |tab| tab.document.position());
    let text = toml::to_string(&stored).map_err(io::Error::other)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use guinea_core::Document;

    fn tab(path: Option<&Path>, text: &str) -> Tab {
        let mut document = Document::from_text(text);
        document.path = path.map(Path::to_path_buf);
        Tab::new(document)
    }

    #[test]
    fn saves_file_tabs_with_positions_and_restores_existing_ones() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("Việt b.txt");
        std::fs::write(&a, "one\ntwo\n").unwrap();
        std::fs::write(&b, "e\u{301}x\n").unwrap();
        let mut first = tab(Some(&a), "one\ntwo\n");
        first.document.select(6, 6).unwrap();
        let mut second = tab(Some(&b), "e\u{301}x\n");
        second.document.select(2, 2).unwrap();
        let mut review = tab(Some(&a), "one\ntwo\n");
        review.review_source = Some(first.document.id);
        let tabs = vec![first, tab(None, "draft"), second, review];
        let session = dir.path().join("state/session.toml");
        save(&session, &tabs, 2).unwrap();

        let (files, mut restore, missing) = Restore::load(&session);
        assert_eq!(files, vec![a.clone(), b.clone()]);
        assert_eq!(missing, 0);
        assert_eq!(restore.opened(&a), Some((1, 2)));
        assert_eq!(restore.take_active(), None, "b is still outstanding");
        assert_eq!(restore.opened(&b), Some((0, 1)), "grapheme column");
        assert_eq!(restore.take_active(), Some(b));
        assert!(restore.is_empty());
    }

    #[test]
    fn missing_files_and_damaged_sessions_restore_nothing_harmful() {
        let dir = tempfile::tempdir().unwrap();
        let kept = dir.path().join("kept.txt");
        std::fs::write(&kept, "x").unwrap();
        let stored = Stored {
            version: 1,
            active: Some(0),
            files: vec![
                Entry {
                    path: dir.path().join("gone.txt"),
                    line: 3,
                    column: 0,
                },
                Entry {
                    path: kept.clone(),
                    line: 0,
                    column: 0,
                },
                Entry {
                    path: kept.clone(),
                    line: 9,
                    column: 9,
                },
            ],
        };
        let (files, mut restore, missing) = Restore::from_stored(stored);
        assert_eq!(files, vec![kept.clone()]);
        assert_eq!(missing, 1);
        restore.failed(&kept);
        assert!(restore.is_empty());
        assert_eq!(
            (restore.total, restore.missing),
            (0, 2),
            "failures are skipped"
        );
        assert_eq!(restore.take_active(), None, "the active file was missing");

        let damaged = dir.path().join("session.toml");
        std::fs::write(&damaged, "files = 3\n[[[").unwrap();
        let (files, restore, missing) = Restore::load(&damaged);
        assert!(files.is_empty() && restore.is_empty() && missing == 0);
        let (files, _, _) = Restore::load(&dir.path().join("absent.toml"));
        assert!(files.is_empty());
    }

    #[test]
    fn saving_replaces_the_previous_session_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        std::fs::write(&a, "a").unwrap();
        let session = dir.path().join("session.toml");
        save(&session, &[tab(Some(&a), "a")], 0).unwrap();
        save(&session, &[tab(None, "")], 0).unwrap();
        let (files, _, _) = Restore::load(&session);
        assert!(
            files.is_empty(),
            "closing every file tab clears the session"
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[cfg(windows)]
    #[test]
    fn verbatim_document_paths_match_session_entries() {
        let plain = Path::new(r"C:\notes\Việt.txt");
        assert_eq!(key(Path::new(r"\\?\C:\notes\Việt.txt")), plain);
        assert_eq!(key(plain), plain);
        assert_eq!(
            key(Path::new(r"\\?\UNC\server\share\a.txt")),
            Path::new(r"\\server\share\a.txt")
        );
        // Other verbatim forms (device paths) are left alone.
        assert_eq!(
            key(Path::new(r"\\?\Volume{0}\a.txt")),
            Path::new(r"\\?\Volume{0}\a.txt")
        );
        // A real canonicalized file matches its plain spelling.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(key(&file.canonicalize().unwrap()), key(&file));
    }
}
