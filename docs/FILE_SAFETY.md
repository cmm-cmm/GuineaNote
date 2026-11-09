# File safety

Reads accept strict UTF-8, with an optional initial UTF-8 BOM. UTF-16 BOMs,
invalid UTF-8 and NUL input are rejected without lossy decoding. No heuristic
can identify every legacy encoding that happens to form valid UTF-8; such input
is treated as UTF-8. Non-NUL binary data that happens to be valid UTF-8 cannot
be detected reliably. Controls are rendered safely. A BOM is stored separately
from text and preserved on save. Original newline bytes stay inside the rope.

Paths resolve their parent directory without following the final component.
Final symlinks and non-regular files are refused. Parent directories must
exist; the editor does not create directories. New paths create no file until
save. Large reads first compare metadata with the configured warning threshold
(10 MiB default), then read at most limit+1 bytes to handle growth. Confirmation
removes the size limit; this does not promise bounded memory. Metadata changes
during a read cause a retry error.

Save protocol:

1. Reject readonly documents, readonly targets, unexpected existing save-as
   destinations, deleted original targets and detectable external changes.
2. Compare the target's SHA-256, length and modification time to the loaded or
   previously saved fingerprint. The hash is streamed with a 64 KiB buffer.
3. Encode strict UTF-8 plus the original BOM. Create a uniquely named temporary
   file in the destination directory; write all bytes and flush. Copy the old
   file's Rust permissions when replacing, then sync the temporary file.
4. Recheck the destination fingerprint immediately before replacement. Use
   tempfile persist for existing files and persist_noclobber for new paths.
   Never delete the original as a fallback for a failed replacement.
5. After successful replacement, update path/fingerprint/savepoint without
   clearing history. On Unix, try to sync the parent directory; failure is a
   post-commit warning, not a claim that saving did not occur.

Every error before replacement leaves the document's dirty/savepoint/history
unchanged. An injected partial temporary write, flush/sync/create/replace error
is tested to leave original bytes intact and remove temporary files on normal
unwinding. Native Windows tests additionally withhold FILE_SHARE_DELETE to
provoke a real replacement failure, and deny FILE_ADD_FILE on a newly-created
owned NTFS test directory to provoke actual PermissionDenied creating the temp
file. ACLs are restored with a scope guard. Original bytes, dirty state and
subsequent successful saving are checked.

A parent-process test starts a child save, waits at deterministic barriers at
create/partial-write/flush/sync/pre-replace, then forcibly kills and reaps it.
The original BOM/Vietnamese/emoji/mixed-ending bytes must remain identical at
all five pre-commit stages. This is a real process kill, not just a returned
error. It does not interrupt the OS replacement syscall or simulate power loss,
and no crash recovery is promised. Actual filesystem exhaustion is NOT TESTED.

Atomicity and durability depend on OS/filesystem. [tempfile's persist
documentation](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html#method.persist)
describes its replacement semantics; no universal power-loss or remote-filesystem
guarantee is asserted here. A hostile concurrent writer can change an existing
target between the final check and replacement (TOCTOU); there is no filesystem
compare-and-swap or interprocess lock. Existing hard links continue to reference
the old inode after replacement. ACLs, ownership, extended attributes, Windows
alternate streams and all platform-specific metadata are not fully preserved
by Rust permissions. Avoid relying on this editor for those metadata guarantees.
Network filesystems, junction/reparse-point edge cases, antivirus interference,
real disk exhaustion and power loss require separate validation. Native Windows
symlink creation was attempted but denied with WinError 1314 (missing privilege);
the Unix symlink test has been cross-compiled but not executed on this host.

There is no crash recovery/autosave. A killed process can leave a temporary
file. Never treat the existence of a saved fingerprint as a backup mechanism.
