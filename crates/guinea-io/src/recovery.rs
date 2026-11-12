//! Private, versioned checkpoints. Recovery never writes a user's source file.
use crate::platform;
use guinea_core::{ContentSnapshot, Document, TextBuffer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

const MAGIC: &[u8; 8] = b"GNOTE\0\x01\0";
pub const MAX_DRAFT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_STORE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: usize = 128;
const MAX_HEADER: usize = 16 * 1024;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
enum StoredPath {
    Unix(Vec<u8>),
    Windows(Vec<u16>),
}
impl StoredPath {
    fn capture(path: &Path) -> Self {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            Self::Windows(path.as_os_str().encode_wide().collect())
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Self::Unix(path.as_os_str().as_bytes().to_vec())
        }
    }
    fn path(self) -> io::Result<PathBuf> {
        match self {
            #[cfg(windows)]
            Self::Windows(wide) => {
                use std::os::windows::ffi::OsStringExt;
                Ok(std::ffi::OsString::from_wide(&wide).into())
            }
            #[cfg(unix)]
            Self::Unix(bytes) => {
                use std::os::unix::ffi::OsStringExt;
                Ok(std::ffi::OsString::from_vec(bytes).into())
            }
            _ => Err(invalid("Recovery path belongs to another platform")),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    version: u32,
    bytes: u64,
    bom: bool,
    source: Option<StoredPath>,
    source_sha256: Option<[u8; 32]>,
    anchor: usize,
    head: usize,
}
pub struct Checkpoint {
    pub id: u64,
    pub revision: u64,
    pub content: ContentSnapshot,
    header: Header,
}
impl Checkpoint {
    pub fn capture(document: &Document) -> Self {
        let selection = document.selection();
        Self {
            id: document.id,
            revision: document.revision(),
            content: document.snapshot_buffer(),
            header: Header {
                version: 1,
                bytes: document.buffer().len_bytes() as u64,
                bom: document.bom,
                source: document
                    .path
                    .as_deref()
                    .or(document.recovery_source.as_deref())
                    .map(StoredPath::capture),
                source_sha256: document.fingerprint.as_ref().map(|f| f.sha256),
                anchor: selection.anchor,
                head: selection.head,
            },
        }
    }
}
pub struct Recovered {
    pub document: Document,
    pub source: Option<PathBuf>,
    pub checkpoint: PathBuf,
}
struct Claim {
    path: PathBuf,
    _lock: Option<File>,
}
#[derive(PartialEq, Eq)]
struct Version {
    revision: u64,
    bom: bool,
    source: Option<StoredPath>,
    source_sha256: Option<[u8; 32]>,
}
pub struct Store {
    root: PathBuf,
    session: PathBuf,
    _lock: Option<File>,
    claimed: Vec<Claim>,
    origins: HashMap<u64, PathBuf>,
    versions: HashMap<u64, Version>,
    records: HashSet<u64>,
}
struct HashWrite<W> {
    inner: W,
    digest: Sha256,
}
impl<W: Write> Write for HashWrite<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(bytes)?;
        self.digest.update(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
struct HashRead<R> {
    inner: R,
    digest: Sha256,
}
impl<R: Read> Read for HashRead<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(out)?;
        self.digest.update(&out[..n]);
        Ok(n)
    }
}
fn lock_file(path: &Path, create: bool) -> io::Result<File> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if platform::special(&meta) || !meta.is_file() {
            return Err(invalid("Recovery lock must be a regular file"));
        }
    } else if !create {
        return Err(invalid("Recovery session has no lock"));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(create)
        .truncate(false)
        .open(path)?;
    platform::private(path, false)?;
    Ok(file)
}
fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
impl Store {
    /// Claim inactive sessions with OS locks; active editors are never recovered.
    pub fn open(root: &Path) -> io::Result<(Self, Vec<Recovered>, Vec<String>)> {
        for ancestor in root.ancestors() {
            if let Ok(meta) = fs::symlink_metadata(ancestor)
                && platform::special(&meta)
            {
                return Err(invalid("Recovery directory ancestors cannot be links"));
            }
        }
        fs::create_dir_all(root)?;
        platform::private(root, true)?;
        let root = root.canonicalize()?;
        let mut claimed = Vec::new();
        let mut drafts = Vec::new();
        let mut warnings = Vec::new();
        for (index, entry) in fs::read_dir(&root)?.enumerate() {
            if index >= MAX_ENTRIES {
                warnings
                    .push("Recovery scan limited to 128 sessions; remaining files retained".into());
                break;
            }
            let entry = entry?;
            let path = entry.path();
            if !entry.file_name().to_string_lossy().starts_with("session-") {
                continue;
            }
            if platform::special(&fs::symlink_metadata(&path)?) || !path.is_dir() {
                warnings.push("Unsafe recovery session was retained without opening".into());
                continue;
            }
            platform::private(&path, true)?;
            let lock = match lock_file(&path.join("session.lock"), false) {
                Ok(lock) => lock,
                Err(error) => {
                    warnings.push(error.to_string());
                    continue;
                }
            };
            match lock.try_lock() {
                Ok(()) => {}
                Err(fs::TryLockError::WouldBlock) => continue,
                Err(error) => {
                    warnings.push(error.to_string());
                    continue;
                }
            }
            for entry in fs::read_dir(&path)?.take(MAX_ENTRIES + 1) {
                let entry = entry?;
                let record = entry.path();
                if record.extension().is_none_or(|e| e != "gnr") {
                    continue;
                }
                if drafts.len() >= MAX_ENTRIES {
                    warnings.push(
                        "Recovery draft limit reached; remaining checkpoints retained".into(),
                    );
                    break;
                }
                match read_checkpoint(&record) {
                    Ok(draft) => drafts.push(draft),
                    Err(error) => warnings.push(format!("Recovery checkpoint retained: {error}")),
                }
            }
            claimed.push(Claim {
                path,
                _lock: Some(lock),
            });
        }
        let session = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(&root)?
            .keep();
        platform::private(&session, true)?;
        let lock = lock_file(&session.join("session.lock"), true)?;
        lock.try_lock().map_err(io::Error::from)?;
        let mut store = Self {
            root,
            session,
            _lock: Some(lock),
            claimed,
            origins: HashMap::new(),
            versions: HashMap::new(),
            records: HashSet::new(),
        };
        let drafts = store.drop_duplicates(drafts, &mut warnings);
        store.origins = drafts
            .iter()
            .map(|d| (d.document.id, d.checkpoint.clone()))
            .collect();
        Ok((store, drafts, warnings))
    }
    /// A recovered draft is handed off by committing it to the new session
    /// before its claimed origin is removed. A crash between the two leaves
    /// both checkpoints; they hold the same draft, so only one is offered and
    /// the identical copies are deleted (the survivor is still on disk).
    fn drop_duplicates(
        &self,
        drafts: Vec<Recovered>,
        warnings: &mut Vec<String>,
    ) -> Vec<Recovered> {
        let mut kept: Vec<Recovered> = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let duplicate = kept.iter().any(|k| {
                k.source == draft.source
                    && k.document.bom == draft.document.bom
                    && k.document.buffer().len_bytes() == draft.document.buffer().len_bytes()
                    && k.document.text() == draft.document.text()
            });
            if !duplicate {
                kept.push(draft);
                continue;
            }
            if let Err(error) = self.remove_claimed(&draft.checkpoint) {
                warnings.push(format!("Duplicate recovery checkpoint retained: {error}"));
            }
        }
        kept
    }
    pub fn checkpoint(&mut self, snapshots: Vec<Checkpoint>, open: &[u64]) -> io::Result<()> {
        let quota = lock_file(&self.root.join("quota.lock"), true)?;
        quota.try_lock().map_err(io::Error::from)?;
        let dirty: HashSet<u64> = snapshots.iter().map(|s| s.id).collect();
        let mut errors = Vec::new();
        // Reclaim only explicitly clean/closed records before evaluating quota.
        let completed: Vec<u64> = self
            .records
            .iter()
            .copied()
            .filter(|id| !dirty.contains(id))
            .collect();
        for id in completed {
            match self.remove_own(&self.session.join(format!("{id}.gnr"))) {
                Ok(()) => {
                    self.records.remove(&id);
                    self.versions.remove(&id);
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
        for snapshot in snapshots {
            // One oversized/unwritable draft must not starve other tabs.
            let result = (|| -> io::Result<()> {
                let version = Version {
                    revision: snapshot.revision,
                    bom: snapshot.header.bom,
                    source: snapshot.header.source.clone(),
                    source_sha256: snapshot.header.source_sha256,
                };
                if self.versions.get(&snapshot.id) == Some(&version) {
                    return Ok(());
                }
                let target = self.session.join(format!("{}.gnr", snapshot.id));
                let old = fs::metadata(&target).map_or(0, |m| m.len());
                let (bytes, entries) = self.usage()?;
                if snapshot.header.bytes > MAX_DRAFT_BYTES
                    || bytes
                        .saturating_sub(old)
                        .saturating_add(snapshot.header.bytes + MAX_HEADER as u64 + 44)
                        > MAX_STORE_BYTES
                    || (old == 0 && entries >= MAX_ENTRIES)
                {
                    return Err(invalid(
                        "Recovery quota exceeded; existing checkpoints retained. Save your changes.",
                    ));
                }
                let written = write_checkpoint(&target, &snapshot);
                // Persist may have succeeded before directory fsync reported an error.
                if target.exists() {
                    self.records.insert(snapshot.id);
                }
                written?;
                self.versions.insert(snapshot.id, version);
                if let Some(origin) = self.origins.get(&snapshot.id) {
                    self.remove_claimed(origin)?;
                    self.origins.remove(&snapshot.id);
                }
                Ok(())
            })();
            if let Err(error) = result {
                errors.push(error.to_string());
            }
        }
        let closed: Vec<u64> = self
            .origins
            .keys()
            .copied()
            .filter(|id| !open.contains(id) || !dirty.contains(id))
            .collect();
        for id in closed {
            if let Some(origin) = self.origins.get(&id) {
                match self.remove_claimed(origin) {
                    Ok(()) => {
                        self.origins.remove(&id);
                    }
                    Err(error) => errors.push(error.to_string()),
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(errors.join("; ")))
        }
    }
    fn usage(&self) -> io::Result<(u64, usize)> {
        let mut bytes = 0u64;
        let mut files = 0usize;
        for (index, entry) in fs::read_dir(&self.root)?.enumerate() {
            if index > MAX_ENTRIES {
                return Err(invalid(
                    "Recovery session quota exceeded; checkpoints retained",
                ));
            }
            let path = entry?.path();
            let meta = fs::symlink_metadata(&path)?;
            if platform::special(&meta) || !meta.is_dir() {
                continue;
            }
            for (index, entry) in fs::read_dir(path)?.enumerate() {
                if index > MAX_ENTRIES {
                    return Err(invalid(
                        "Recovery entry quota exceeded; checkpoints retained",
                    ));
                }
                let entry = entry?;
                let meta = fs::symlink_metadata(entry.path())?;
                if platform::special(&meta) {
                    return Err(invalid("Unsafe entry in recovery store"));
                }
                if meta.is_file() && entry.path().extension().is_some_and(|e| e == "gnr") {
                    bytes = bytes.saturating_add(meta.len());
                    files += 1;
                }
            }
        }
        Ok((bytes, files))
    }
    fn remove_own(&self, path: &Path) -> io::Result<()> {
        if path.parent() == Some(self.session.as_path()) && !path.try_exists()? {
            return Ok(());
        }
        if path.parent() != Some(self.session.as_path())
            || platform::special(&fs::symlink_metadata(path)?)
        {
            return Err(invalid("Unsafe recovery cleanup path"));
        }
        fs::remove_file(path)?;
        sync_directory(&self.session)
    }
    fn remove_claimed(&self, path: &Path) -> io::Result<()> {
        if !self
            .claimed
            .iter()
            .any(|c| Some(c.path.as_path()) == path.parent())
            || path.extension().is_none_or(|e| e != "gnr")
            || platform::special(&fs::symlink_metadata(path)?)
        {
            return Err(invalid("Unsafe claimed recovery path"));
        }
        fs::remove_file(path)?;
        sync_directory(path.parent().expect("claimed parent"))
    }
    /// Only invoke after a deliberate normal quit/discard, never from Drop.
    pub fn discard(&mut self, discard_recovered: bool) -> io::Result<()> {
        for id in &self.records {
            self.remove_own(&self.session.join(format!("{id}.gnr")))?;
        }
        if discard_recovered {
            for path in self.origins.values() {
                self.remove_claimed(path)?;
            }
        }
        self.versions.clear();
        self.records.clear();
        self.origins.clear();
        Ok(())
    }
}
fn write_checkpoint(path: &Path, snapshot: &Checkpoint) -> io::Result<()> {
    let header = serde_json::to_vec(&snapshot.header)?;
    if header.len() > MAX_HEADER {
        return Err(invalid("Recovery header too large"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Recovery parent missing"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    platform::private(file.path(), false)?;
    {
        let mut out = HashWrite {
            inner: BufWriter::new(file.as_file_mut()),
            digest: Sha256::new(),
        };
        out.write_all(MAGIC)?;
        out.write_all(&(header.len() as u32).to_le_bytes())?;
        out.write_all(&header)?;
        let mut result = Ok(());
        snapshot.content.visit_chunks(&mut |chunk| {
            result = out.write_all(chunk.as_bytes());
            result.is_ok()
        });
        result?;
        let digest = out.digest.finalize();
        out.inner.write_all(&digest)?;
        out.inner.flush()?;
    }
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    sync_directory(parent)
}
pub fn read_checkpoint(path: &Path) -> io::Result<Recovered> {
    platform::private(path, false)?;
    let file = platform::open_file(path)?;
    let size = file.metadata()?.len();
    if !(44..=MAX_DRAFT_BYTES + MAX_HEADER as u64 + 44).contains(&size) {
        return Err(invalid("Invalid recovery size"));
    }
    let mut input = HashRead {
        inner: BufReader::new(file),
        digest: Sha256::new(),
    };
    let mut magic = [0; 8];
    input.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(invalid("Unknown recovery format"));
    }
    let mut len = [0; 4];
    input.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_HEADER {
        return Err(invalid("Invalid recovery header length"));
    }
    let mut header = vec![0; len];
    input.read_exact(&mut header)?;
    let header: Header = serde_json::from_slice(&header)?;
    if header.version != 1
        || header.bytes > MAX_DRAFT_BYTES
        || header.bytes + len as u64 + 44 != size
    {
        return Err(invalid("Invalid recovery length/version"));
    }
    let mut document = Document::from_utf8_reader((&mut input).take(header.bytes))?;
    if document.buffer().len_bytes() as u64 != header.bytes {
        return Err(invalid("Truncated recovery payload"));
    }
    let digest: [u8; 32] = input.digest.finalize().into();
    let mut expected = [0; 32];
    input.inner.read_exact(&mut expected)?;
    if digest != expected {
        return Err(invalid("Recovery checksum mismatch"));
    }
    document.bom = header.bom;
    let _ = document.select(header.anchor, header.head);
    document.mark_recovered();
    let source = header.source.map(StoredPath::path).transpose()?;
    document.recovery_source = source.clone();
    Ok(Recovered {
        document,
        source,
        checkpoint: path.to_path_buf(),
    })
}

fn cleanup_empty(path: &Path) {
    let safe = path.is_absolute()
        && path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("session-"))
        && fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !platform::special(&m));
    if !safe {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let entries = entries.collect::<Result<Vec<_>, _>>();
    if let Ok(entries) = entries
        && entries.len() == 1
        && entries[0].file_name() == "session.lock"
        && fs::symlink_metadata(entries[0].path())
            .is_ok_and(|m| m.is_file() && !platform::special(&m))
    {
        let _ = fs::remove_file(entries[0].path());
        let _ = fs::remove_dir(path);
    }
}
impl Drop for Claim {
    fn drop(&mut self) {
        self._lock.take();
        cleanup_empty(&self.path);
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        self._lock.take();
        cleanup_empty(&self.session);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_during_handoff_offers_the_draft_once() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let mut document = Document::from_text("draft e\u{301} 👩‍💻\r\n");
        document.insert("x").unwrap();
        let (mut first, _, _) = Store::open(&root).unwrap();
        first
            .checkpoint(vec![Checkpoint::capture(&document)], &[document.id])
            .unwrap();
        drop(first); // Crash.
        let (second, mut found, _) = Store::open(&root).unwrap();
        let recovered = found.pop().unwrap();
        // Crash after the handoff commit but before the origin was removed.
        let target = second
            .session
            .join(format!("{}.gnr", recovered.document.id));
        write_checkpoint(&target, &Checkpoint::capture(&recovered.document)).unwrap();
        assert!(recovered.checkpoint.exists());
        drop(second);
        let (mut third, found, warnings) = Store::open(&root).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(found.len(), 1, "the same draft is offered once");
        assert_eq!(found[0].document.text(), document.text());
        let remaining = fs::read_dir(&root)
            .unwrap()
            .flat_map(|session| fs::read_dir(session.unwrap().path()).into_iter().flatten())
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == "gnr")
            })
            .count();
        assert_eq!(remaining, 1, "the identical copy is deleted");
        // A different draft of the same file is never merged.
        let mut other = Document::from_text(&found[0].document.text());
        other.insert("y").unwrap();
        third
            .checkpoint(
                vec![
                    Checkpoint::capture(&found[0].document),
                    Checkpoint::capture(&other),
                ],
                &[found[0].document.id, other.id],
            )
            .unwrap();
        drop(third);
        let (mut fourth, found, _) = Store::open(&root).unwrap();
        assert_eq!(found.len(), 2);
        fourth.discard(true).unwrap();
    }

    #[test]
    fn checkpoint_recovers_exact_unsaved_unicode_bom_and_source_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let source = dir.path().join("Việt.txt");
        fs::write(&source, "original on disk").unwrap();
        let mut document = Document::from_text("Việt e\u{301} 👩‍💻\r\nsecond\n\0draft");
        document.bom = true;
        document.path = Some(source.clone());
        document.insert("unsaved ").unwrap();
        let expected = document.text();
        let selection = document.selection();
        let (mut active, found, warnings) = Store::open(&root).unwrap();
        assert!(found.is_empty() && warnings.is_empty());
        active
            .checkpoint(vec![Checkpoint::capture(&document)], &[document.id])
            .unwrap();
        let (other, found, warnings) = Store::open(&root).unwrap();
        assert!(
            found.is_empty() && warnings.is_empty(),
            "must not claim a live editor"
        );
        drop(other);
        drop(active); // Simulate unclean shutdown; no discard.
        let (mut restarted, mut found, warnings) = Store::open(&root).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(found.len(), 1);
        let recovered = found.pop().unwrap();
        assert_eq!(recovered.document.text(), expected);
        assert!(recovered.document.bom && recovered.document.is_dirty());
        assert!(recovered.document.path.is_none() && recovered.document.fingerprint.is_none());
        assert_eq!(recovered.document.selection(), selection);
        assert_eq!(recovered.source.as_deref(), Some(source.as_path()));
        assert_ne!(recovered.document.id, document.id);
        // Even when the source changes, a recovered draft remains independent.
        fs::write(&source, "external changes after crash").unwrap();
        restarted
            .checkpoint(
                vec![Checkpoint::capture(&recovered.document)],
                &[recovered.document.id],
            )
            .unwrap();
        assert!(
            !recovered.checkpoint.exists(),
            "old checkpoint retired only after new commit"
        );
        drop(restarted); // A second crash must retain secret/scope provenance.
        let (mut restarted, found, warnings) = Store::open(&root).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].document.recovery_source.as_deref(),
            Some(source.as_path())
        );
        assert_eq!(found[0].document.text(), expected);
        restarted.discard(true).unwrap();
        drop(restarted);
        let (_, found, warnings) = Store::open(&root).unwrap();
        assert!(found.is_empty() && warnings.is_empty());
        assert_eq!(
            fs::read_to_string(source).unwrap(),
            "external changes after crash"
        );
    }

    #[test]
    fn corrupted_checkpoints_are_retained_and_quick_quit_does_not_discard_unseen_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let mut d = Document::from_text("original draft");
        d.insert("unsaved ").unwrap();
        let (mut store, _, _) = Store::open(&root).unwrap();
        store
            .checkpoint(vec![Checkpoint::capture(&d)], &[d.id])
            .unwrap();
        let path = store.session.join(format!("{}.gnr", d.id));
        drop(store);
        let (mut reopened, found, _) = Store::open(&root).unwrap();
        assert_eq!(found.len(), 1);
        reopened.discard(false).unwrap();
        drop(reopened);
        assert!(path.exists(), "user never saw this draft");
        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        fs::write(&path, bytes).unwrap();
        // A parallel test spawning a child (real_process_kill_*) can briefly
        // hold a forked copy of a lock descriptor on Unix; a locked session is
        // skipped silently, so retry while the session was not inspected.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let (found, warnings) = loop {
            let (_, found, warnings) = Store::open(&root).unwrap();
            if !found.is_empty() || !warnings.is_empty() || std::time::Instant::now() > deadline {
                break (found, warnings);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(found.is_empty());
        assert!(!warnings.is_empty());
        assert!(path.exists());
    }

    #[test]
    fn successful_save_or_explicit_close_removes_only_its_own_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let mut a = Document::default();
        a.insert("first").unwrap();
        let mut b = Document::default();
        b.insert("second").unwrap();
        let (mut store, _, _) = Store::open(&root).unwrap();
        store
            .checkpoint(
                vec![Checkpoint::capture(&a), Checkpoint::capture(&b)],
                &[a.id, b.id],
            )
            .unwrap();
        a.mark_saved();
        store
            .checkpoint(vec![Checkpoint::capture(&b)], &[a.id, b.id])
            .unwrap();
        assert!(!store.session.join(format!("{}.gnr", a.id)).exists());
        assert!(store.session.join(format!("{}.gnr", b.id)).exists());
        store.checkpoint(vec![], &[a.id]).unwrap();
        store.discard(true).unwrap();
        drop(store);
        let (_, found, warnings) = Store::open(&root).unwrap();
        assert!(found.is_empty() && warnings.is_empty());
    }

    #[test]
    fn save_as_provenance_change_updates_checkpoint_without_a_text_edit() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let mut document = Document::default();
        document.insert("secret after Save As").unwrap();
        document.path = Some(dir.path().join("public.txt"));
        let (mut store, _, _) = Store::open(&root).unwrap();
        store
            .checkpoint(vec![Checkpoint::capture(&document)], &[document.id])
            .unwrap();
        document.path = Some(dir.path().join(".env"));
        store
            .checkpoint(vec![Checkpoint::capture(&document)], &[document.id])
            .unwrap();
        drop(store);
        let (mut restarted, found, warnings) = Store::open(&root).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(found[0].document.recovery_source, document.path);
        restarted.discard(true).unwrap();
    }

    #[test]
    fn rejected_draft_does_not_starve_other_tabs_or_remove_prior_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _, _) = Store::open(dir.path()).unwrap();
        let mut a = Document::default();
        a.insert("retained").unwrap();
        let mut b = Document::default();
        b.insert("other tab").unwrap();
        store
            .checkpoint(vec![Checkpoint::capture(&a)], &[a.id, b.id])
            .unwrap();
        let mut rejected = Checkpoint::capture(&a);
        rejected.revision += 1;
        rejected.header.bytes = MAX_DRAFT_BYTES + 1;
        assert!(
            store
                .checkpoint(vec![rejected, Checkpoint::capture(&b)], &[a.id, b.id])
                .is_err()
        );
        drop(store);
        let (mut restarted, found, warnings) = Store::open(dir.path()).unwrap();
        assert!(warnings.is_empty());
        let mut texts: Vec<_> = found.iter().map(|d| d.document.text()).collect();
        texts.sort();
        assert_eq!(texts, ["other tab", "retained"]);
        restarted.discard(true).unwrap();
    }

    #[test]
    fn recovery_kill_probe() {
        let Some(root) = std::env::var_os("GUINEANOTE_RECOVERY_KILL_ROOT") else {
            return;
        };
        let marker = std::env::var_os("GUINEANOTE_RECOVERY_KILL_READY").unwrap();
        let (mut store, _, _) = Store::open(Path::new(&root)).unwrap();
        let mut document = Document::from_text("e\u{301} 👩‍💻\r\n");
        document.insert("unsaved ").unwrap();
        document.bom = true;
        store
            .checkpoint(vec![Checkpoint::capture(&document)], &[document.id])
            .unwrap();
        fs::write(marker, b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn real_process_kill_recovers_committed_checkpoint() {
        use std::{
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("recovery");
        let marker = dir.path().join("ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "recovery::tests::recovery_kill_probe",
                "--nocapture",
            ])
            .env("GUINEANOTE_RECOVERY_KILL_ROOT", &root)
            .env("GUINEANOTE_RECOVERY_KILL_READY", &marker)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while !marker.exists() {
            if Instant::now() > deadline || child.try_wait().unwrap().is_some() {
                let _ = child.kill();
                let _ = child.wait();
                panic!("checkpoint child did not become ready");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let (mut store, drafts, warnings) = Store::open(&root).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].document.text(), "unsaved e\u{301} 👩‍💻\r\n");
        assert!(drafts[0].document.bom && drafts[0].document.is_dirty());
        store.discard(true).unwrap();
    }
}
