# File safety

Current policy: 2026-11-10. Native Windows tests exist; Unix-specific metadata
behavior is cross-compiled, not runtime-certified on this host.

## Reads and exact bytes

Only strict UTF-8 with optional initial UTF-8 BOM is editable. UTF-16 BOM,
invalid UTF-8 and NUL input are refused without lossy conversion. Legacy/binary
bytes that happen to be valid non-NUL UTF-8 cannot be identified reliably.
Original LF, CRLF, bare CR and mixed ending bytes remain in the rope. Controls
are rendered safely. No normalization is performed on save.

Parent directories resolve first; final symlink/reparse and non-regular files
are refused, including Windows alternate-stream path syntax. The final handle
uses NOFOLLOW/OPEN_REPARSE_POINT. The I/O adapter captures volume+file-index on
Windows or dev+inode on Unix, compares metadata around streaming reads, and
rechecks that the path still names that object. Reads use a 64 KiB buffer into
Ropey's strict UTF-8 builder, hashing original BOM+payload without full input
Vec/String copies. The size limit reads at most limit+1 bytes to detect growth.
Above the default 10 MiB threshold requires explicit confirmation; this is not
a bounded-memory large-file backend. Missing paths are created only on save.

## Save protocol and metadata

1. Reject readonly buffers/targets, unrelated existing Save As destinations,
   deleted or externally changed originals, links and hard-linked destinations.
   Hard links require Save As rather than silently breaking other names.
2. Pin a regular handle and verify identity, size, modification time and streamed
   SHA-256. A replacement object with identical bytes/time is still a conflict.
3. Create a same-directory named temp with private initial permissions. Apply
   existing metadata before copying original/edited bytes. On Windows std file
   copy retains alternate streams, then only the temporary main stream is reset.
   Stream snapshot chunks plus BOM through a 64 KiB writer and hash the output.
4. Preserve supported metadata again, flush, sync the temporary file, and fully
   revalidate the destination immediately before commit. Windows validation
   handles are dropped immediately before persist to avoid sharing conflicts.
5. Persist existing files or persist_noclobber for a new destination. Never
   delete an original as a fallback. Update fingerprint/savepoint only on commit;
   owner merges metadata without losing edits made during background saving.
6. Unix directory sync failure after replacement is a post-commit warning.
   It must not be reported as if the original remained unchanged.

Windows preservation copies owner/group/DACL, protecting the effective copied
DACL against accidental inherited grants; inheritance flags may therefore
change. ADS byte preservation and protected owner-only ACLs are tested natively.
SACL/audit entries, encryption, compression and every filesystem attribute are
not guaranteed. Unix requires matching owner/group or refuses replacement;
visible extended attributes and Rust permissions are copied through handles.
Hidden/privileged attributes and ACL variants need native validation. Any
reported preservation failure aborts before replacing the original.

Injected failures at create/partial-write/flush/sync/pre-replace retain original
bytes and dirty/history state. Tests also cover real Windows sharing violations,
ACL-denied temp creation, same-content identity replacement, hard links and
byte-exact BOM/Unicode/mixed-ending streaming boundaries. A parent kills/reaps a
child save at all five pre-commit barriers. These tests do not interrupt the OS
replacement syscall or simulate actual disk exhaustion/power loss.

Atomicity/durability depend on OS/filesystem. Identity and full revalidation
reduce detectable races but do not provide filesystem compare-and-swap. A hostile
writer can still change a path after the final check, and ancestor-directory
races/network filesystems are outside this guarantee. File locking is advisory
for recovery sessions, not an exclusion lock against every external file writer.

## Crash recovery

Enabled by default; set recovery = false in explicit TOML to disable it.
A separate worker captures dirty source tabs about once per second after changes
and commits private, versioned, SHA-256-checked snapshots. It never writes the
source path. Windows root defaults to %LOCALAPPDATA%/GuineaNote/recovery;
Unix uses $XDG_STATE_HOME/GuineaNote/recovery or ~/.local/state/GuineaNote/recovery.
GUINEANOTE_RECOVERY_DIR overrides the root. An explicit GUINEANOTE_PREFERENCES
uses a sibling recovery directory for isolated environments.

Directories/files are owner-only (Unix 0700/0600; protected owner-only Windows
DACL); foreign owners, symlinks and reparse points are refused. Snapshots are
plaintext local drafts and may contain secrets. OS session locks prevent opening
a live editor's records. Inactive sessions are claimed until checkpoint handoff.

Each record preserves strict UTF-8 content, BOM, selection, native source-path
provenance and optional original fingerprint. Checksum/schema/size validation
precedes recovery. Original files are never automatically overwritten or merged,
even if externally changed: recovery opens an independent dirty tab with no save
destination. Ctrl+S asks for a new path; deliberate Save As is required. F9 shows
provenance. Agent policy applies to recovered secret/out-of-workspace provenance,
including across repeated crashes and Save As metadata changes.

Limits: 64 MiB payload per draft, 256 MiB aggregate store, 128 managed records /
scanned sessions. Header max 16 KiB; quota includes conservative header overhead.
One failed/oversized draft does not starve other tabs. Existing checkpoints are
retained on failure; corrupt/unknown leftovers are retained with a warning.
Explicit clean/closed records free space before quota evaluation. No automatic
old-draft deletion is used to make space. Storage failure warns to save manually.

Only a successful new checkpoint retires its previous recovered origin. Normal
quit after deliberate save/discard removes acknowledged drafts. A quick quit
before recovered tabs are delivered retains unseen drafts. Panic/disconnect/
process kill preserves the last committed checkpoint. Checkpoints are periodic,
not every keystroke: changes since the last completed checkpoint can be lost.
They do not guarantee physical power-loss durability, encrypted storage, backups,
source autosave or session/tab-layout restoration. A killed save/checkpoint may
leave a temporary file; unsupported/corrupt records are not silently erased.
