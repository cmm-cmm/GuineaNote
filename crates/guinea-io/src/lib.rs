//! Strict UTF-8, exact newline bytes, and conservative same-directory saves.
#[cfg(windows)]
use std::io::Seek;
use std::{
    fs::{self, File, Metadata},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use guinea_core::{ContentSnapshot, Document, Fingerprint};
mod platform;
pub mod recovery;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

const BOM: &[u8] = b"\xef\xbb\xbf";

#[derive(Debug, Error)]
pub enum FileError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("unsupported encoding: only strict UTF-8 (optionally BOM) can be edited")]
    Encoding,
    #[error("NUL/binary input is not supported; original file was not modified")]
    Binary,
    #[error("file changed outside GuineaNote; reload, save to a new path, or cancel")]
    Conflict,
    #[error("document or destination is read-only")]
    ReadOnly,
    #[error("symbolic links and non-regular files are refused; open the real file path")]
    SpecialFile,
    #[error(
        "file is {size} bytes; opening may consume substantially more RAM (limit {limit}); explicit confirmation required"
    )]
    LargeFile { size: u64, limit: u64 },
    #[error("destination already exists; open it in a tab before editing instead of overwriting")]
    DestinationExists,
    #[error("file changed while it was being read; retry opening")]
    UnstableRead,
    #[error("hard-linked targets cannot be replaced safely; use Save As")]
    HardLinks,
}

#[derive(Debug, Default)]
pub struct SaveReport {
    /// Replacement succeeded, but a post-commit durability operation failed.
    pub warning: Option<String>,
}

/// Resolve the parent, but not the final component: final symlinks must remain
/// visible to the safety checks. Parent directory must already exist.
pub fn absolute_path(path: &Path) -> Result<PathBuf, FileError> {
    let name = path.file_name().ok_or(FileError::SpecialFile)?;
    #[cfg(windows)]
    if name.to_string_lossy().contains(':') {
        return Err(FileError::SpecialFile);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize()?.join(name))
}

fn metadata(path: &Path) -> Result<Option<Metadata>, FileError> {
    match fs::symlink_metadata(path) {
        Ok(m) if platform::special(&m) || !m.is_file() => Err(FileError::SpecialFile),
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

struct CheckedReader<R> {
    input: R,
    digest: Sha256,
    size: u64,
    nul: bool,
}
impl<R: Read> Read for CheckedReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.input.read(out)?;
        self.digest.update(&out[..n]);
        self.size += n as u64;
        self.nul |= out[..n].contains(&0);
        if self.nul {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "NUL input"));
        }
        Ok(n)
    }
}
fn same_metadata(a: &Metadata, b: &Metadata) -> bool {
    a.len() == b.len() && a.modified().ok() == b.modified().ok()
}

pub fn open(path: &Path, readonly: bool, limit: u64) -> Result<Document, FileError> {
    let path = absolute_path(path)?;
    let Some(before) = metadata(&path)? else {
        let mut document = Document::default();
        document.path = Some(path);
        document.readonly = readonly;
        return Ok(document);
    };
    if before.len() > limit {
        return Err(FileError::LargeFile {
            size: before.len(),
            limit,
        });
    }
    let file = platform::open_file(&path)?;
    let opened = file.metadata()?;
    let identity = platform::identity(&file)?;
    if !same_metadata(&before, &opened) {
        return Err(FileError::UnstableRead);
    }
    let mut input = BufReader::with_capacity(64 * 1024, (&file).take(limit.saturating_add(1)));
    let prefix = input.fill_buf()?;
    if prefix.starts_with(b"\xff\xfe") || prefix.starts_with(b"\xfe\xff") {
        return Err(FileError::Encoding);
    }
    let bom = prefix.starts_with(BOM);
    let mut reader = CheckedReader {
        input,
        digest: Sha256::new(),
        size: 0,
        nul: false,
    };
    if bom {
        let mut prefix = [0; 3];
        reader.read_exact(&mut prefix)?;
    }
    let result = Document::from_utf8_reader(&mut reader);
    if reader.size > limit {
        return Err(FileError::LargeFile {
            size: reader.size,
            limit,
        });
    }
    if reader.nul {
        return Err(FileError::Binary);
    }
    let mut document = result.map_err(|e| {
        if e.kind() == io::ErrorKind::InvalidData {
            FileError::Encoding
        } else {
            e.into()
        }
    })?;
    let after = file.metadata()?;
    if !same_metadata(&opened, &after)
        || after.len() != reader.size
        || !platform::same_object(&path, identity).unwrap_or(false)
    {
        return Err(FileError::UnstableRead);
    }
    document.path = Some(path);
    document.bom = bom;
    document.readonly = readonly || after.permissions().readonly();
    document.fingerprint = Some(Fingerprint {
        size: reader.size,
        modified: after.modified().ok(),
        sha256: reader.digest.finalize().into(),
        identity: Some(identity),
    });
    document.mark_saved();
    Ok(document)
}

fn validate_destination(
    path: &Path,
    expected: Option<&Fingerprint>,
) -> Result<Option<File>, FileError> {
    let current = metadata(path)?;
    match (current, expected) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(FileError::Conflict),
        (Some(_), None) => Err(FileError::DestinationExists),
        (Some(meta), Some(expected)) => {
            if meta.permissions().readonly() {
                return Err(FileError::ReadOnly);
            }
            let file = platform::open_file(path)?;
            let identity = platform::identity(&file)?;
            if expected.identity.is_some_and(|id| id != identity) {
                return Err(FileError::Conflict);
            }
            if platform::links(&file)? > 1 {
                return Err(FileError::HardLinks);
            }
            let before = file.metadata()?;
            if before.len() != expected.size || before.modified().ok() != expected.modified {
                return Err(FileError::Conflict);
            }
            let mut input = (&file).take(expected.size.saturating_add(1));
            let mut digest = Sha256::new();
            let mut chunk = [0; 64 * 1024];
            let mut size = 0;
            loop {
                let n = input.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                digest.update(&chunk[..n]);
                size += n as u64;
            }
            let after = file.metadata()?;
            if !same_metadata(&before, &after)
                || size != expected.size
                || <[u8; 32]>::from(digest.finalize()) != expected.sha256
                || !platform::same_object(path, identity).unwrap_or(false)
            {
                return Err(FileError::Conflict);
            }
            Ok(Some(file))
        }
    }
}

/// Stream immutable UTF-8 bytes without a full String or output Vec.
pub fn write_content(
    snapshot: &ContentSnapshot,
    bom: bool,
    out: &mut impl Write,
) -> Result<(u64, [u8; 32]), FileError> {
    let mut digest = Sha256::new();
    let mut size = 0u64;
    if bom {
        out.write_all(BOM)?;
        digest.update(BOM);
        size += 3;
    }
    let mut result: Result<(), FileError> = Ok(());
    snapshot.visit_chunks(&mut |chunk| {
        if chunk.contains('\0') {
            result = Err(FileError::Binary);
            return false;
        }
        if let Err(error) = out.write_all(chunk.as_bytes()) {
            result = Err(error.into());
            return false;
        }
        digest.update(chunk.as_bytes());
        size += chunk.len() as u64;
        true
    });
    result?;
    Ok((size, digest.finalize().into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Create,
    Write,
    Flush,
    Sync,
    Replace,
}

pub fn save(document: &mut Document, target: &Path) -> Result<SaveReport, FileError> {
    save_with_hook(document, target, |_| Ok(()))
}

fn save_with_hook(
    document: &mut Document,
    target: &Path,
    mut hook: impl FnMut(Stage) -> io::Result<()>,
) -> Result<SaveReport, FileError> {
    if document.readonly {
        return Err(FileError::ReadOnly);
    }
    let target = absolute_path(target)?;
    let same_path = document.path.as_ref() == Some(&target);
    let expected = if same_path {
        document.fingerprint.as_ref()
    } else {
        None
    };
    let original = validate_destination(&target, expected)?;
    let parent = target.parent().ok_or(FileError::SpecialFile)?;
    hook(Stage::Create)?;
    let mut temp = NamedTempFile::new_in(parent)?;
    // Draft bytes must never inherit a broader ACL than the original file.
    // Newly created files start owner-only; replacement metadata is applied
    // before any original or edited bytes are copied into the temporary file.
    platform::private(temp.path(), false)?;
    if let Some(original) = &original {
        platform::preserve(original, &target, temp.as_file(), temp.path())?;
    }
    #[cfg(windows)]
    if original.is_some() {
        // std CopyFileEx copies NTFS alternate streams. Only truncate the main
        // stream afterwards; original bytes remain untouched until persist.
        fs::copy(&target, temp.path())
            .map_err(|e| io::Error::new(e.kind(), format!("Preserving file streams: {e}")))?;
        temp.as_file().set_len(0).map_err(|e| {
            io::Error::new(e.kind(), format!("Resetting temporary main stream: {e}"))
        })?;
        temp.as_file_mut().rewind()?;
    }
    let snapshot = document.snapshot_buffer();
    let midpoint = (snapshot.len_bytes() + usize::from(document.bom) * 3) / 2;
    let mut called = false;
    let (size, sha256) = {
        struct Progress<'a, W, H> {
            out: &'a mut W,
            hook: &'a mut H,
            written: usize,
            midpoint: usize,
            called: &'a mut bool,
        }
        impl<W: Write, H: FnMut(Stage) -> io::Result<()>> Write for Progress<'_, W, H> {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if !*self.called && self.written >= self.midpoint {
                    (self.hook)(Stage::Write)?;
                    *self.called = true;
                }
                let len = if !*self.called {
                    bytes.len().min(self.midpoint - self.written)
                } else {
                    bytes.len()
                };
                let n = self.out.write(&bytes[..len])?;
                self.written += n;
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                self.out.flush()
            }
        }
        let mut out = BufWriter::with_capacity(64 * 1024, temp.as_file_mut());
        let result = write_content(
            &snapshot,
            document.bom,
            &mut Progress {
                out: &mut out,
                hook: &mut hook,
                written: 0,
                midpoint,
                called: &mut called,
            },
        )?;
        if !called {
            hook(Stage::Write)?;
        }
        hook(Stage::Flush)?;
        out.flush()?;
        result
    };
    if let Some(original) = &original {
        platform::preserve(original, &target, temp.as_file(), temp.path())
            .map_err(|e| io::Error::new(e.kind(), format!("Preserving security metadata: {e}")))?;
    }
    hook(Stage::Sync)?;
    temp.as_file()
        .sync_all()
        .map_err(|e| io::Error::new(e.kind(), format!("Syncing temporary file: {e}")))?;
    let saved_fingerprint = Fingerprint {
        size,
        sha256,
        modified: temp.as_file().metadata()?.modified().ok(),
        identity: Some(platform::identity(temp.as_file())?),
    };
    hook(Stage::Replace)?;
    let verified = validate_destination(&target, expected)?;
    drop(verified);
    drop(original);
    // Never delete the destination as a fallback. New paths use no-clobber.
    if expected.is_some() {
        temp.persist(&target).map_err(|error| {
            FileError::Io(io::Error::new(
                error.error.kind(),
                format!("Committing temporary file: {}", error.error),
            ))
        })?;
    } else {
        temp.persist_noclobber(&target).map_err(|error| {
            FileError::Io(io::Error::new(
                error.error.kind(),
                format!("Committing temporary file: {}", error.error),
            ))
        })?;
    }
    // Commit point: later durability warnings must not be reported as failure.
    #[cfg(unix)]
    let warning = File::open(parent)
        .and_then(|directory| directory.sync_all())
        .err()
        .map(|e| format!("Saved, but directory sync failed: {e}"));
    #[cfg(not(unix))]
    let warning = None;
    document.path = Some(target);
    document.recovery_source = None;
    document.fingerprint = Some(saved_fingerprint);
    document.mark_saved();
    Ok(SaveReport { warning })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn replacement_identity_conflict_and_hardlinks_preserve_all_original_names() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.txt");
        fs::write(&path, "original").unwrap();
        let mut d = open(&path, false, 100).unwrap();
        d.insert("edit ").unwrap();
        let previous = dir.path().join("previous.txt");
        fs::rename(&path, &previous).unwrap();
        fs::write(&path, "original").unwrap();
        // Simulate a same-content/same-mtime replacement: identity must still fail.
        d.fingerprint.as_mut().unwrap().modified = fs::metadata(&path).unwrap().modified().ok();
        assert!(matches!(save(&mut d, &path), Err(FileError::Conflict)));
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        let mut d = open(&path, false, 100).unwrap();
        d.insert("edit ").unwrap();
        let alias = dir.path().join("alias.txt");
        fs::hard_link(&path, &alias).unwrap();
        assert!(matches!(save(&mut d, &path), Err(FileError::HardLinks)));
        assert_eq!(fs::read(&path).unwrap(), fs::read(&alias).unwrap());
        assert!(d.is_dirty());
        save(&mut d, &dir.path().join("safe-copy.txt")).unwrap();
        assert_eq!(fs::read_to_string(&alias).unwrap(), "original");
    }

    #[cfg(windows)]
    #[test]
    fn windows_save_preserves_alternate_streams_and_protected_acl() {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType, SecurityInformation},
            utilities, wrappers,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("streams.txt");
        fs::write(&path, "original").unwrap();
        let stream = format!("{}:guineanote-test-metadata", path.display());
        fs::write(&stream, b"private metadata\0exact").unwrap();
        let sid = utilities::current_process_sid().unwrap();
        let acl: LocalBox<SecurityDescriptor> = format!("D:P(A;;FA;;;{sid})").parse().unwrap();
        wrappers::SetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
            None,
            None,
            acl.dacl(),
            None,
        )
        .unwrap();
        let mut document = open(&path, false, 100).unwrap();
        document.insert("Việt 👩‍💻 ").unwrap();
        save(&mut document, &path).unwrap();
        assert_eq!(fs::read(&stream).unwrap(), b"private metadata\0exact");
        let actual = wrappers::GetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner | SecurityInformation::Dacl,
        )
        .unwrap();
        assert_eq!(actual.owner(), Some(&*sid));
        let dacl = actual.dacl().unwrap();
        assert_eq!(dacl.len(), 1);
        assert_eq!(dacl.get_ace(0).unwrap().sid(), Some(&*sid));
        assert_eq!(fs::read_to_string(&path).unwrap(), "Việt 👩‍💻 original");
    }

    #[test]
    fn streaming_builder_detects_newlines_across_chunks_without_normalizing_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("chunked.txt");
        let original = format!("\u{feff}{}\r\nViệt e\u{301} 👩‍💻\nend\r", "a".repeat(65532));
        fs::write(&path, &original).unwrap();
        let mut d = open(&path, false, 100_000).unwrap();
        assert_eq!(d.line_ending(), guinea_core::LineEnding::Mixed);
        save(&mut d, &path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn valid_utf8_roundtrips_without_normalization(text in ".{0,200}", bom in any::<bool>()) {
            let text = text.replace('\0', "");
            let mut bytes = Vec::new();
            if bom { bytes.extend_from_slice(BOM); }
            bytes.extend_from_slice(text.as_bytes());
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("roundtrip.txt");
            fs::write(&path, &bytes).unwrap();
            let mut d = open(&path, false, u64::MAX).unwrap();
            save(&mut d, &path).unwrap();
            prop_assert_eq!(fs::read(path).unwrap(), bytes);
            prop_assert!(!d.is_dirty());
        }
    }

    #[test]
    fn golden_roundtrips_are_byte_exact() {
        let dir = tempfile::tempdir().unwrap();
        let cases: &[&[u8]] = &[
            b"",
            b"ascii without newline",
            b"a\nb\n",
            b"a\r\nb\r\n",
            b"a\r\nb\nc\r",
            b"\xef\xbb\xbf",
            b"\xef\xbb\xbfa\r\nb\n",
            "Việt e\u{301} 👩‍💻 🇻🇳\t界\n".as_bytes(),
        ];
        for (i, bytes) in cases.iter().enumerate() {
            let path = dir.path().join(format!("Việt {i}.txt"));
            fs::write(&path, bytes).unwrap();
            let mut d = open(&path, false, u64::MAX).unwrap();
            assert!(!d.is_dirty());
            save(&mut d, &path).unwrap();
            assert_eq!(fs::read(&path).unwrap(), *bytes);
            assert!(!d.is_dirty());
        }
    }

    #[test]
    fn invalid_input_never_becomes_lossy_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad");
        for bytes in [b"\xff\xf0".as_slice(), b"\xff\xfea\0", b"a\0b"] {
            fs::write(&path, bytes).unwrap();
            assert!(open(&path, false, 100).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }

    #[test]
    fn conflicts_and_save_as_do_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a");
        fs::write(&path, "original").unwrap();
        let mut d = open(&path, false, 100).unwrap();
        d.insert("edit").unwrap();
        fs::write(&path, "external").unwrap();
        assert!(matches!(save(&mut d, &path), Err(FileError::Conflict)));
        assert!(d.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        let other = dir.path().join("other");
        fs::write(&other, "keep").unwrap();
        assert!(matches!(
            save(&mut d, &other),
            Err(FileError::DestinationExists)
        ));
        let fresh = dir.path().join("new");
        save(&mut d, &fresh).unwrap();
        assert_eq!(fs::read_to_string(fresh).unwrap(), "editoriginal");
        assert!(!d.is_dirty());
        d.undo().unwrap();
        assert!(d.is_dirty());
    }

    #[test]
    fn missing_paths_are_not_created_until_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new");
        let mut d = open(&path, false, 100).unwrap();
        assert!(!path.exists());
        d.insert("new content").unwrap();
        fs::write(&path, "concurrent creator").unwrap();
        assert!(save(&mut d, &path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "concurrent creator");
        assert!(d.is_dirty());
    }

    #[test]
    fn failures_before_commit_preserve_original_and_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("safe");
        for stage in [
            Stage::Create,
            Stage::Write,
            Stage::Flush,
            Stage::Sync,
            Stage::Replace,
        ] {
            fs::write(&path, "original").unwrap();
            let mut d = open(&path, false, 100).unwrap();
            d.insert("edit").unwrap();
            let result = save_with_hook(&mut d, &path, |at| {
                if at == stage {
                    Err(io::Error::other("injected disk/permission/replace failure"))
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());
            assert!(d.is_dirty());
            assert_eq!(fs::read_to_string(&path).unwrap(), "original");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
            d.undo().unwrap();
            assert!(!d.is_dirty());
        }
    }

    // Child-only entry point for abrupt-termination tests. The normal test
    // invocation has no probe environment and returns without file operations.
    #[test]
    fn save_kill_probe() {
        let Some(stage) = std::env::var_os("GUINEANOTE_TEST_KILL_STAGE") else {
            return;
        };
        let path =
            PathBuf::from(std::env::var_os("GUINEANOTE_TEST_KILL_PATH").expect("probe path"));
        let barrier =
            PathBuf::from(std::env::var_os("GUINEANOTE_TEST_KILL_BARRIER").expect("probe barrier"));
        let mut document = open(&path, false, 1024).unwrap();
        document.insert("unsaved edit ").unwrap();
        save_with_hook(&mut document, &path, |at| {
            if format!("{at:?}") == stage.to_string_lossy() {
                fs::write(&barrier, b"ready")?;
                loop {
                    std::thread::park();
                }
            }
            Ok(())
        })
        .unwrap();
        panic!("probe did not reach the requested stage");
    }

    #[test]
    fn killing_save_process_before_commit_preserves_original() {
        use std::{
            process::{Child, Command, Stdio},
            time::{Duration, Instant},
        };
        struct StopChild(Child);
        impl Drop for StopChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        for stage in [
            Stage::Create,
            Stage::Write,
            Stage::Flush,
            Stage::Sync,
            Stage::Replace,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("original.txt");
            let barrier = directory.path().join("ready");
            let original = "\u{feff}Việt 👩‍💻\r\nunchanged\n".as_bytes();
            fs::write(&path, original).unwrap();
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "tests::save_kill_probe", "--nocapture"])
                .env("GUINEANOTE_TEST_KILL_STAGE", format!("{stage:?}"))
                .env("GUINEANOTE_TEST_KILL_PATH", &path)
                .env("GUINEANOTE_TEST_KILL_BARRIER", &barrier)
                .stdout(Stdio::null())
                .spawn()
                .unwrap();
            let mut child = StopChild(child);
            let deadline = Instant::now() + Duration::from_secs(20);
            while !barrier.exists() {
                assert!(
                    child.0.try_wait().unwrap().is_none(),
                    "probe exited before {stage:?}"
                );
                assert!(Instant::now() < deadline, "probe timeout at {stage:?}");
                std::thread::sleep(Duration::from_millis(5));
            }
            child.0.kill().unwrap();
            assert!(!child.0.wait().unwrap().success());
            assert_eq!(
                fs::read(&path).unwrap(),
                original,
                "interruption at {stage:?}"
            );
            // Killed processes may leave temp files; dropping only this owned
            // temp directory cleans up the test, never the user's documents.
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_sharing_violation_at_replace_preserves_original_and_dirty() {
        use std::os::windows::fs::OpenOptionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("locked.txt");
        fs::write(&path, "original").unwrap();
        let mut document = open(&path, false, 1024).unwrap();
        document.insert("edit ").unwrap();
        // Allow reads/writes so fingerprint checks work, but withhold
        // FILE_SHARE_DELETE: Windows itself must reject the replacement.
        let lock = File::options()
            .read(true)
            .share_mode(0x03)
            .open(&path)
            .unwrap();
        let result = save(&mut document, &path);
        assert!(matches!(result, Err(FileError::Io(_))), "{result:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert!(document.is_dirty());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        drop(lock);
        save(&mut document, &path).unwrap();
        assert!(!document.is_dirty());
        document.undo().unwrap();
        assert_eq!(document.text(), "original");
        assert!(document.is_dirty());
    }

    #[cfg(windows)]
    #[test]
    fn windows_denied_temp_creation_preserves_original_and_dirty() {
        use std::process::Command;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.txt");
        fs::write(&path, "original").unwrap();
        let mut document = open(&path, false, 1024).unwrap();
        document.insert("edit ").unwrap();
        let identity = Command::new("whoami.exe")
            .args(["/user", "/fo", "csv", "/nh"])
            .output()
            .unwrap();
        assert!(identity.status.success());
        // Extract the ASCII SID without depending on the console code page or
        // on localized/non-ASCII account names. icacls accepts *SID principals.
        let sid = identity
            .stdout
            .split(|&byte| byte == b'"')
            .find(|field| field.starts_with(b"S-1-"))
            .expect("whoami SID field");
        let identity = format!("*{}", std::str::from_utf8(sid).expect("ASCII SID"));
        struct RestoreAcl<'a> {
            directory: &'a Path,
            identity: &'a str,
        }
        impl Drop for RestoreAcl<'_> {
            fn drop(&mut self) {
                let _ = Command::new("icacls.exe")
                    .arg(self.directory)
                    .args(["/remove:d", self.identity])
                    .output();
            }
        }
        // Only the newly-created, owned temporary directory is affected.
        // No inheritance: existing file reads are still permitted.
        let restore = RestoreAcl {
            directory: directory.path(),
            identity: &identity,
        };
        let denied = Command::new("icacls.exe")
            .arg(directory.path())
            .args(["/deny", &format!("{identity}:(WD)")])
            .output()
            .unwrap();
        assert!(denied.status.success(), "icacls deny failed: {denied:?}");
        let result = save(&mut document, &path);
        let reset = Command::new("icacls.exe")
            .arg(directory.path())
            .args(["/remove:d", &identity])
            .output()
            .unwrap();
        drop(restore);
        assert!(reset.status.success(), "icacls restore failed: {reset:?}");
        assert!(
            matches!(result, Err(FileError::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied),
            "{result:?}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert!(document.is_dirty());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        save(&mut document, &path).unwrap();
        assert!(!document.is_dirty());
    }

    #[test]
    fn detect_change_during_save_before_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("safe");
        fs::write(&path, "original").unwrap();
        let mut d = open(&path, false, 100).unwrap();
        d.insert("edit").unwrap();
        let result = save_with_hook(&mut d, &path, |at| {
            if at == Stage::Replace {
                fs::write(&path, "external")?;
            }
            Ok(())
        });
        assert!(matches!(result, Err(FileError::Conflict)));
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert!(d.is_dirty());
    }

    #[test]
    fn readonly_and_large_file_are_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("read-only");
        fs::write(&path, "content").unwrap();
        assert!(matches!(
            open(&path, false, 2),
            Err(FileError::LargeFile { .. })
        ));
        let mut d = open(&path, true, 100).unwrap();
        assert!(matches!(save(&mut d, &path), Err(FileError::ReadOnly)));
        d.readonly = false;
        let original = fs::metadata(&path).unwrap().permissions();
        let mut permissions = original.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        let result = save(&mut d, &path);
        fs::set_permissions(&path, original).unwrap();
        assert!(matches!(result, Err(FileError::ReadOnly)));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("real");
        let link = dir.path().join("link");
        fs::write(&path, "original").unwrap();
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(matches!(
            open(&link, false, 100),
            Err(FileError::SpecialFile)
        ));
        let mut d = Document::from_text("new");
        assert!(matches!(save(&mut d, &link), Err(FileError::SpecialFile)));
        assert_eq!(fs::read_to_string(path).unwrap(), "original");
    }
}
