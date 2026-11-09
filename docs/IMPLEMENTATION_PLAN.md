# Implementation plan

The authoritative scope is `GUINEANOTE_TECHNICAL_SPEC.md` v1.0. The initial
repository contains only that specification and the implementation prompt; it
has no source, tests, CI, dependencies, or Git metadata. Existing documents are
preserved. Local environment: Windows x64, Rust stable 1.98.1 (MSVC).

Later user override: no GitHub CI during development. The initially prepared
workflow was removed before the first push; all checks below run locally.

| Milestone | Dependency | Deliverables and definition of done |
| --- | --- | --- |
| M0 Foundation | none | Cargo workspace, CLI, terminal RAII/panic cleanup, MIT, README, three-OS CI; formatting, clippy, tests and build pass locally. Interactive OS checks reported separately. |
| M1 Text Core | M0 | Private Ropey backend behind TextBuffer; checked coordinates; grapheme movement, selection, edits, clipboard commands, snapshot history and content-based savepoint. Unicode unit/property tests pass. |
| M2 File Safety | M1 | Strict UTF-8, BOM and exact newline preservation, safe same-directory temporary writes, content fingerprints, no silent overwrite, readonly/symlink checks. Golden and injected-failure tests pass. |
| M3 Usable TUI | M2 | Tabs, keyboard-only editing, dialogs, literal search/replace, status, viewport, configuration, background file jobs and unsaved confirmation. Event/render tests pass; manual terminal tests remain explicit. |
| M4 Hardening | M3 | Basic revision-tagged syntax, integration and render golden tests, reproducible benchmark, release build, architecture/behavior/safety/testing/performance documentation, acceptance matrix. No claim of cross-platform or PTY success without execution. |

## Decisions and risks

- Each editing command is one undo transaction; no timed typing coalescing.
  Rope snapshots share storage, but retained history is not bounded memory.
- Preserve actual newline bytes inside the buffer. Enter inserts CRLF only for
  exclusively CRLF documents; LF is used for LF/mixed/empty files. No normalization.
- Strictly reject invalid UTF-8, UTF-16 BOMs and NUL-containing input. No lossy editing.
- Refuse symlink targets and saves to an existing unrelated save-as path. An
  existing destination must be opened before editing it; no force-overwrite API.
- Fingerprints include SHA-256, length and modification time. A recheck before
  replacement reduces races but cannot eliminate concurrent-writer TOCTOU races.
- Use tempfile's platform replacement operation; never delete the target as a
  fallback. Filesystem/OS atomicity and power-loss guarantees are not universal.
- Expensive file work runs in a worker; editor changes are suspended during that
  job while the event loop can render and process resize. Large-file opt-in is
  explicit. Resident Ropey, full-line grapheme scans and snapshots have limits.
- No LSP, plugins, Git integration, paged engine, recovery or npm publication (M5).

## Validation sequence

Later explicit user scope change: a visual workspace redesign and directory
tree are authorized now. UI_REDESIGN.md records this exception to the original
post-MVP explorer deferral; no other M5 feature is implied.

At each milestone run the relevant unit/integration checks, then update
`IMPLEMENTATION_STATUS.md` with actual evidence. Final gates:
`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo build --release`.
Use Vietnamese NFC/NFD, emoji ZWJ, CJK, tabs, CRLF, BOM, mixed endings,
invalid input and failure injection. Record unexecuted PTY/manual OS checks as
NOT TESTED. Benchmark results must include dataset, command and environment.
