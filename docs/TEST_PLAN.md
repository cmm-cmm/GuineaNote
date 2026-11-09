# Test plan and reproduction

Use current Rust stable with native build tools. Required commands:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

The suite covers checked buffer ranges; byte/scalar/grapheme/cell/UTF-16
conversions; NFC/NFD Vietnamese, ZWJ emoji, flags, CJK, CRLF; selection/history;
dirty savepoints and branching; literal matching; Windows surrogate key-up and
AltGr; readonly; config; dialogs; worker multi-tab open/save/search/replace;
resize/clipping; idle redraw behavior; CLI noninteractive/help/version errors.
Workspace coverage additionally checks lazy tree order/expand/collapse/refresh,
stale-root results, empty/missing folders, explicit listing caps, pane focus,
dirty tabs across folder changes, mouse hit regions and grapheme-safe caret
clicks, neighboring tabs and compact layouts. The CLI directory contract now
accepts folders while noninteractive browsing still reports a clear error.
Property tests generate Unicode insertion and command sequences and verify
boundary invariants and complete undo/redo roundtrips. Default proptest case
count is 256 for each of four core properties; the I/O roundtrip property
uses 64 cases. Raise the core count, for example, with
`PROPTEST_CASES=4096 cargo test -p guinea-core` (PowerShell uses `$env:`).

Golden byte fixtures are literal byte arrays in guinea-io's tests: empty,
ASCII/no final newline, LF, CRLF, mixed/bare CR, BOM-only, BOM+mixed, Vietnamese
NFD/emoji/CJK/tabs. Expected bytes are compared directly after saving. Binary,
invalid UTF-8 and UTF-16 are rejected. Injected failures after a partial temp
write and at create/flush/sync/replace verify original bytes, dirty state,
history and temp cleanup. Integration tests exercise real Unicode paths,
independent document savepoints, repeated saves, conflicts and readonly flags.
Unix symlink tests run only on Unix and are not counted as locally executed on
Windows.

Additional M4 cases now exercise actual NTFS sharing violations at replace and
actual directory ACL denial of temporary-file creation. They invoke whoami and
icacls only on an owned temporary directory; a scope guard restores its ACL.
Five child processes are killed at pre-commit save-stage barriers, checking
that the original remains byte-identical. The `save_kill_probe` helper test is
inert during normal suite invocation and activated only in its child process.
No test is ignored to obtain a passing result.

For native Windows PTY and idle CPU sampling, install dev-only tools locally:

```powershell
python -m pip install --target target/pty-tools pywinpty==3.1.2 psutil==7.2.2 pyte==0.8.2
python scripts/pty_smoke.py
python scripts/pty_workspace.py
cargo build -p guinea-tui --example panic_cleanup
python scripts/pty_smoke.py --panic target/debug/examples/panic_cleanup.exe
```

On Unix omit pywinpty from installation and `.exe` from the example path. The
script uses a PTY and emulates terminal diffs with pyte, verifies Vietnamese/emoji
typed bytes, find/replace, save, copy/paste, new tab, undo/redo and save-as byte
comparison, unsaved cancellation, F1/F2
fallback, resize and exit. A wrapper compares native input/output console modes
(Windows) or termios (Unix) before and after. The panic example proves cleanup
and a nonzero exit without suppressing the panic. Raw transcripts are written
under `target/pty-*.log`. CPU samples are short smoke evidence, not a sustained
profiling study. PTY tooling is not an application runtime dependency.

The workspace PTY probe expands/collapses an owned temporary tree, opens and
saves a file, changes workspace with dirty tabs, toggles/focuses the sidebar,
and verifies cleanup. SGR pointer input exercises the toolbar and editor caret
through the native input adapter. The panic probe enables mouse capture too.

To capture the actual PTY screen for visual review:

```powershell
python -m pip install --target target/pty-tools Pillow
python scripts/tui_preview.py
```

PNG/text captures are written to `target/previews/`. Only the capture child
overrides NO_COLOR to demonstrate the theme; the application honors the user's
environment. Pillow is a dev-only capture dependency. Captures were visually
inspected for color, glyph portability, panel focus and layout.

GitHub CI is deliberately not configured during development at the user's
request; the initial workflow was removed before pushing the repository.
Run the required gates, native PTY/panic probes and dependency audit locally.
Windows/Linux/macOS native runs remain separate verification evidence.
At the initial checkpoint cargo-deny was unavailable; license metadata was reviewed with
`cargo tree --locked --workspace --prefix none --format '{p} {l}'` at the initial
checkpoint. The 2026-11-09 continuation installed cargo-deny 0.20.2 under
`target/devtools` and completed advisory/bans/license/source checks. Reproduce
on Windows without modifying the global PATH:

```powershell
cargo install cargo-deny --version 0.20.2 --locked --root target/devtools
.\target\devtools\bin\cargo-deny.exe --locked fetch db
.\target\devtools\bin\cargo-deny.exe --workspace --locked --offline check --hide-inclusion-graph
```

The offline second step uses the explicitly fetched snapshot, not an implicit
claim of perpetual advisory freshness. See DEPENDENCY_AUDIT.md for the observed
database revision, one retained duplicate-version warning, and the manifest fix
that keeps the existing wildcard ban enabled.

Linux/macOS all-target checks and clippy can be cross-checked from this host:

```sh
rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin
cargo clippy --locked --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
cargo clippy --locked --workspace --all-targets --target aarch64-apple-darwin -- -D warnings
```

These checks do not link or execute native binaries. The Unix PTY helper now
creates a controlling terminal/session so /dev/tty and resize SIGWINCH can work
on a native Unix validation run. Screen waits inspect the current emulated screen,
not historical output. `python scripts/pty_latency.py` records narrow native
latency/working-set measurements; see PERFORMANCE.md for scope and limitations.

Remaining manual cases: Windows Terminal/PowerShell human UX and SSH, actual
disk full, kill during the replacement syscall/power loss, unusual
symlinks/reparse points, network filesystems, terminal/font Unicode variations,
Linux/macOS native runs and long-line responsiveness. Record NOT TESTED until
there is actual evidence. See the acceptance matrix in IMPLEMENTATION_STATUS.

## Review, localization and response budget

Run `python -X utf8 scripts/pty_responsiveness.py 10485760` after the release
build. It fails when any measured interaction reaches 100 ms. It uses owned
temporary files, fresh screen state changes, exact saved-byte checks and normal
terminal cleanup, including 10 MiB ASCII/Unicode single lines, end movement,
edits/history, F7 review, F8 localization, 10 MiB background JSON parsing, and
tab/clipboard/prompt/search/folder/tree/quit workflows. Full JSON parse completion
is separate from interactive feedback. Raw output is target/responsiveness.json.
Run without concurrent compilation for comparable local measurements.

Review unit tests cover malformed JSON/XML, quoted/multiline and unequal-width
CSV records, TSV, source integrity, stale worker revisions, locale/resize cache
keys, later table columns and cell widths. Core tests cover grapheme chunk
boundaries, sparse index edits/history and parallel index/tab-stop equivalence.
CLI tests reject unsupported locale values before terminal entry.

`python -X utf8 scripts/tui_review_preview.py` captures actual Markdown/CSV/JSON
panes in Vietnamese and English under target/previews. pyte 0.8.2's display
adapter tolerates orphaned wide-cell stubs after ZWJ redraw; the parser/cell
buffer and actual key/output transport are retained. Byte-exact saves and native
Unicode input tests remain independent checks.
