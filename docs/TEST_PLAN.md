# Test plan and reproduction

## Folder features removed; key and mouse synchronization — 12/11/2026

The directory explorer, open folder, project file/content search and folder
arguments were removed. `pty_project.py` and `pty_workspace.py` were deleted;
older sections below that run them are historical. Other probes now pass files
only (Agent workspace = the first file's folder). Rust tests added:

- `every_clickable_region_maps_to_a_handled_action` (render.rs): 5 sizes × 2
  languages × welcome/editor/focus, every dialog opened; main-window actions must
  run a known command, dialog buttons must have a mouse handler, F2 rows must be
  keymap commands.
- `keyboard_guides_only_name_bound_or_context_keys` (commands.rs): F1 guides and
  UI string literals (dialogs, workspace renderer, i18n) may only name keys bound
  in the keymap or listed dialog/context keys.
- `folders_are_rejected_and_files_require_an_interactive_terminal` (tests/cli.rs).

Header, tab toolbar, footer and welcome key labels are generated from the keymap
(`commands::short_key`, `commands::hints`).

## Unprepared long lines, keymap v1 and editing — 12/11/2026

After `cargo build --workspace --release`, run sequentially:

```powershell
python -X utf8 scripts/pty_navigation.py --trace --record-only
python -X utf8 scripts/pty_navigation.py --trace --record-only --fixtures unprepared-12mib,unprepared-line-101,unprepared-after-newline --affinity 0,1
python -X utf8 scripts/pty_navigation.py --trace --record-only --fixtures unprepared-12mib,unprepared-line-101,unprepared-after-newline --affinity 0
```

The unprepared fixtures cover lines the open worker does not pre-index: a 12 MiB
line (opened through the large-file prompt), a 5 MiB line at line 101 and a 10 MiB
line after a newline edit. `--affinity` pins the editor (via `GUINEANOTE_PROBE_AFFINITY`
in `pty_smoke.probe_child`) to model one/two-CPU or contended machines.
`first-end` measures **feedback** (exact column or the pending caret window with
`Col …`); `first_end_completion_ms` records when the exact column appears and is
background completion, not an interaction sample. `GUINEANOTE_BINARY` selects
another build for interleaved A/B runs. See
[unprepared long lines](performance/UNPREPARED_LONG_LINES.md).

Every probe now sets `GUINEANOTE_CLIPBOARD=internal` so runs never replace the
developer's system clipboard. Keymap v1 is covered by Rust tests (unique
bindings, portable primary keys, guide coverage, editing commands through the real
dispatcher) and by `pty_keyboard.py`.

## Keyboard-only TUI audit — 2026-11-11

After `cargo build --workspace --release`, run
`python -X utf8 scripts/pty_keyboard.py`. It launches with `mouse = false`,
uses only key input and saves its report/log under the directory named by
`target/keyboard-latest-path.txt`.

Coverage: unified Find/Replace, all search focus stops, Enter and Alt+A
replacement, Undo/save byte integrity, F12/Ctrl+P manager filtering, Tab/BackTab,
F6 previous/F7 next, Alt arrows, Ctrl+PageUp/PageDown, Ctrl+L, Alt+S, F2 list navigation,
project/file dialogs, pane/focus, review/info/settings, VI/EN help scrolling at
30x9 and safe dirty-close confirmation. Ctrl+Tab/BackTab variants are also
checked at dispatcher and native Win32 decoder level; terminal interception
can require F6 or Alt arrows instead.

Keep `pty_smoke.py`, `pty_search.py --trace`, `pty_tabs.py`,
`pty_dialog_mouse.py`, project/workspace, Agent/recovery and terminal matrix
regressions. Run latency probes sequentially after builds. A search unit-test
helper now waits for shared worker admission instead of assuming a free permit
while other tests run in parallel; runtime worker limits remain unchanged.

## Search feedback and observer profiling — 2026-11-11

After installing the PTY development dependencies below (plus Pillow for search
screenshots) and building release, run sequentially:

```powershell
python -X utf8 -m unittest discover -s scripts -p test_pty_observer.py -v
python -X utf8 scripts/pty_search.py --trace --profile-observer
python -X utf8 scripts/pty_search_matrix.py --runs 20 --record-only
```

The matrix runs the legacy/corrected feedback predicate with trace on/off and
observer profiling on/off. It rotates condition order; each condition repeats
the same small Unicode/BOM/CRLF, readonly and 10 MiB replacement/Undo workflow.
Every run waits for completion and checks file bytes and terminal cleanup.
Raw logs, CSV, per-action stages, reports and checksums live in the directory
named by `target/search-matrix-latest-path.txt`.

`--record-only` retains investigative outliers and does not certify acceptance.
Missing/incomplete traces, dropped samples and correctness failures still fail.
Without that flag the matrix also fails corrected responses above 50 ms; legacy
outliers remain evidence. `--legacy-replacement-feedback` on the single probe
reproduces the old predicate. See the
[investigation](performance/SEARCH_OBSERVER_INVESTIGATION.md) for its limitations.

## Current investigation threshold

Every measured interaction >50 ms requires investigation, even if p99 passes.
Current scripts retain outliers and use >50 rather than the historical >=100
gates described below. Ubuntu/SSH verification is outside this iteration by user
request; checks run on the current native Windows/ConPTY environment.

After a release build, run sequentially (do not build concurrently):

```powershell
cargo run --release -p guinea-core --example interaction_profile
python -X utf8 scripts/pty_interaction_latency.py
python -X utf8 scripts/pty_interaction_latency.py --no-trace
python -X utf8 scripts/pty_interaction_latency.py --iterations 1000
python -X utf8 scripts/pty_navigation.py --trace --record-only --cold-runs 100
python -X utf8 scripts/pty_contention_latency.py --runs 10
```

Record-only preserves investigative data; it does not make outliers pass.
Check application and PTY investigations separately, including dropped samples
and groups with insufficient n. Core microbenchmark is not terminal latency.
Menu/tab regression covers disjoint hit regions and a separator across VI/EN,
welcome/editor and 30x9–148x42 layouts. Core tests compare cache-prefix reuse
against fresh Unicode layout through snapshots, edits and Undo/Redo.

## Project workflow and grouped interaction latency

Build release first, then run sequentially:

```powershell
python -X utf8 scripts/pty_project.py
python -X utf8 scripts/terminal_matrix.py
python -X utf8 scripts/pty_interaction_latency.py
python -X utf8 scripts/pty_interaction_latency.py --no-trace
```

Do not build/audit concurrently with latency measurement. Corpus: 1 KiB and
10 MiB Unicode long lines, typing/delete/caret/history/Home/End/tabs/dialogs/
project/search/mouse/resize. Internal input-to-flush and PTY observer are separate;
groups with n<20 remain insufficient. Trace drops invalidate acceptance. Preserve
failed logs. Terminal matrix checks BOM/NFD/ZWJ/mixed EOL paste/save and cleanup,
truecolor, ANSI16, mono, small windows and keyboard-only. Use explicit --ssh host
--remote-binary path or --tmux for those runtime checks; unavailable is not pass.
See [workflow limits](TERMINAL_WORKFLOW.md) and [baseline](performance/LATENCY_BASELINE.md).

Rust checks cover project ignore/cancel/stale result behavior, safe dirty-buffer
jumps, grapheme goto/out-of-range, focus/palette, ANSI16 and narrow layouts.

## Session restore — 2026-11-12

`python -X utf8 scripts/pty_session.py` (after a release build) runs five
native cases with a private preferences/session folder: exit records the open
files; a bare `guinea` reopens both tabs with the previous active tab and the
caret at a grapheme column after an NFD "é" (typed text and Ctrl+S keep the
CRLF/emoji bytes exact); files named on the command line skip the restore and
replace the record; a vanished file is skipped and reported; `session = false`
neither restores nor rewrites the record. Rust tests cover damaged/absent
records, duplicate entries, positions past a changed file and atomic rewrite.

## UI separation polish — 2026-11-12

Keys are drawn as accent keycaps in the menu, tab controls and footer (same
text as before), tabs have dividers, the F2 list right-aligns key bindings and
search results colour the match. Frames therefore carry more colour codes.
`pty_agent.py` used to block in MCP/CLI calls without reading the PTY; on
Linux the fuller frames filled the PTY buffer, the editor blocked in its frame
write and the waited-for Agent request was cancelled (reproduced 4/4, stack
in `write`/`flush`). The probe now drains the PTY on a pump thread and records
every screen state, so waits still see short-lived states such as
`AI · READ ONLY`. It passes 3/3 on both the previous binary (83e6953) and the
new one. The Windows ConPTY path of the probe is unchanged.

## Edit responsiveness — 2026-11-12

`cargo run --release -p guinea-core --example edit_profile` prints core
timings of whole-document edit commands and long-line Up/Down on a 10 MiB
corpus (see `docs/performance/EDIT_RESPONSIVENESS.md`). Rust tests check that a
block edit above 1 Mi scalars runs on the worker without blocking input and
undoes as one step, that small scopes still apply immediately, and that
Up/Down wait for a pending long-line index and never overtake later input.

## Regex search — 2026-11-12

`python -X utf8 scripts/pty_regex.py` (after a release build): a capture-group
pattern finds nothing as literal text, Alt+R shows `[x] Regex` and two matches,
Replace all with `$2:$1` rewrites both lines and Ctrl+S keeps BOM/CRLF/NFC
bytes exact, and an unclosed group shows `Regex: …` instead of scanning. Rust
tests cover per-line `^`/`$` with CRLF, options, empty-match stepping,
grapheme-split rejection, invalid and oversized patterns, `$1`/`${name}`/`$$`
expansion, replace one/all as one Undo, a pattern longer than the document
(the size estimate no longer treats it as the match length) and Mark keeping
selected text literal.

## Tab bar redesign — 2026-11-12

The tab area is one row (tabs left, count and controls right) under the menu
rule, and the path/context moved into the editor border, so the editor gains
four rows. Render tests check that tabs share the controls' row left of every
control at every size and locale, that the menu rule stays, and that overflowing
tabs keep the count, active tab and close regions. `pty_dialog_mouse.py` now
asserts the shared row and the rule above it, and picks the close `x` after the
named tab instead of the last one in the row.

## Mark — 2026-11-12

`python -X utf8 scripts/pty_mark.py` (after a release build): F2 `mark-all`
with the caret on a word paints both occurrences reversed (and not their
neighbours), `mark-bookmark` bookmarks the two lines, `bookmark-remove-lines`
plus Ctrl+S leaves exactly the remaining CRLF line, and `mark-clear` reports.
Rust tests cover the target choice (selection, NFD caret word, Find query and
options), the worker merge at the same revision, the 100,000-line limit and
painting in the visible text.

## Dialog, mouse and search coverage

After building release binaries, run sequentially:

```powershell
python -X utf8 scripts/pty_dialog_mouse.py
python -X utf8 scripts/pty_search.py
python -X utf8 scripts/pty_responsiveness.py 10485760
python -X utf8 scripts/pty_tabs.py
python -X utf8 scripts/pty_agent_latency.py
python -X utf8 scripts/pty_navigation.py
```

Do not compile or run another performance probe concurrently. These probes
enforce strictly <100 ms for measured fresh UI feedback. Search/replacement,
preview and Agent completion are reported separately. Search exercises a
10 MiB input with nearly a million matches, page wrap and replace-all/Undo.
Native small-file assertions verify BOM/CRLF/Unicode byte preservation,
replace-one/all, two Undo steps, explicit save and read-only protection.
Dialog/mouse coverage includes real native drag/copy, scroll without caret
movement, middle-close, default cancel, clickable decision/submit/close,
file/tab double-click, Vietnamese confirmation and terminal cleanup.

Rust tests cover grapheme-safe matching/field edits, cancellation, page bounds,
stale query/result rejection, immutable original replacement revision,
replace-one progression when inserted text contains the needle, and one-step
Undo. Rendering tests verify small-window hit regions, modal click isolation,
tab spacing, right-aligned format metadata and default confirmation safety.
Run `pty_smoke.py`, `pty_workspace.py`, `pty_dialogs_reader.py` and `pty_agent.py`
for functional regressions. Linux/macOS cross-Clippy does not replace native
runtime verification.

## Continuing UI and navigation performance

After `cargo build --workspace --release`, run
`python -X utf8 scripts/pty_navigation.py`. It opens six fresh Unicode editor
sessions and two sessions with a 10 MiB ASCII prefix followed by bare CR/CJK.
The 32 samples cover initial typing, first End and cached left/right movement.
The editor prepares its first-line index in the open worker; process startup,
file loading and index preparation are reported separately, before timing keys.
`--record-only` retains exploratory pre-change data without enforcing the
100 ms budget. Normal runs fail any measured response >=100 ms.

Core tests compare prepared/reused indexes with full Unicode segmentation and
cell positions through ASCII insert/delete, Unicode joins, undo, CRLF and tab
stops. UI tests check English/Vietnamese AI actions, linked-review lock status,
disabled close hit regions, cached manager details and small-window safety.
Run the general 10 MiB, tab, concurrent-Agent, live-Agent and dialogs/reader
probes sequentially after builds. Screenshots use the actual PTY cell colors.

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

These checks do not link or execute native binaries.

### Native Linux x86_64 in a container

With Docker running, from the repository root:

```sh
bash scripts/linux/run.sh
```

It builds `scripts/linux/Dockerfile` (Rust 1.98 on Debian 12 with rustfmt,
Clippy, Python, xclip, Xvfb and DejaVu fonts), copies the working tree and git
history into the `guinea-linux-work` volume, and runs `scripts/linux/verify.sh`:
fmt, strict Clippy, workspace tests, release build, then every PTY probe in the
Windows order plus `pty_clipboard.py` against a private Xvfb X server, the
interaction-latency corpus with and without trace, and navigation on 12/2/1
CPUs. Results land in `target/linux-out/`. Do not run it while Windows probes
are measuring latency: the container shares the CPUs.

Scope: a Linux kernel (WSL2 6.6 under Docker Desktop) driven through Python's
`pty` and pyte — not GNOME Terminal/Konsole/xterm, Wayland, SSH or tmux.
First run 12/11/2026 (HEAD 79073e9 + working tree): the gate and all 17 probe
steps pass;
see `docs/diagnostics/linux-container/`.

`python -X utf8 scripts/pty_clipboard.py --use-system-clipboard` is the only
probe that uses the real system clipboard: Ctrl+A/Ctrl+C of a Unicode CRLF
document must reach the OS clipboard byte for byte and OS text must paste and
save exactly. It saves and restores the previous clipboard text and refuses to
run when the clipboard holds an image or files.

The Unix PTY helper now
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
edits/history, F7 split/review-tab selection, F8 Settings/localization, F9 details,
10 MiB background JSON parsing, and
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

Expanded format coverage adds ENV export/quoted/multiline/duplicate fields,
INI sections, Properties continuation/escaped separators, config XML/YAML/JSON5
inference and unknown-dialect fallback, notebook Markdown/code/saved text,
non-finite JSON5 values, YAML aliases/multiple documents, complex keys, disabled
include/interpolation and malformed/deep data. Generic code and extensionless
shebang files get native source-review checks. The previous expanded-format
probe had 138 samples per run; its data remains under
docs/responsiveness-expanded-run*.json. The Settings/reader probe has 187
samples per run, with raw data under docs/responsiveness-ux-run*.json.

Run `python -X utf8 scripts/pty_dialogs_reader.py` for saved-language restart
(including a config that omits language), Settings cancellation, filtered file
opening, details, split versus dedicated review, outline navigation, read-only
save protection, live updates and a byte-exact source save. Preferences and files
use owned temporary directories. Screenshots in target/previews show actual
terminal output. Run `pty_smoke.py` and `pty_workspace.py` for native Unicode
input, cleanup, toolbar/caret mouse clicks and folder/tree editing regression.

Rust tests additionally cover Settings Apply/Cancel, queued preference writes,
picker navigation/filtering/missing files, metadata versus unsaved buffer,
linked review history isolation, reload invalidation from revision zero,
Markdown features at narrow widths, semantic spans and modal resize safety.

## Visible tab management

`python -X utf8 scripts/pty_tabs.py` uses twelve temporary files, mouse actions
and a filtered All tabs dialog. It checks blank creation, exact Unicode save,
previous/next, individual close, dirty cancel/discard, inactive selection,
hidden tabs, linked review close, the last blank, VI/EN and normal terminal
cleanup. It fails at any measured response >=100 ms, writing raw evidence to
target/tab-responsiveness.json. No real user file or profile is modified.

The current general response probe has 196 samples; its previous 187-sample
UX checkpoint remains historical. Rust tab tests cover path disambiguation,
empty filters, preserved histories, save completion during the dialog, closing
inactive sources/reviews, stable blank names and narrow/overflow hit regions.
The Agent implementation and evidence are recorded in AGENT_IMPLEMENTATION.md.

## Agent CLI / MCP / AI leases

Build all binaries with `cargo build --workspace --release`, then run locally:

```powershell
python -X utf8 scripts/agent_smoke.py
python -X utf8 scripts/pty_agent_sessions.py
python -X utf8 scripts/pty_agent.py
python -X utf8 scripts/pty_agent_latency.py
```

Functional probes accept `--debug`; latency evidence uses release. Fixtures,
preferences and session credentials are owned temporary data. No real user
document, host MCP configuration or external AI service is modified.

Wire probes cover legacy initialize and modern discovery/per-request metadata,
Unicode tools/resources, schema rejection, missing/unsupported metadata,
oversized-frame recovery and EOF. Two native editor processes verify explicit
session/root matching, same-basename isolation, cross-session handle rejection,
server write grants and original read-only restoration.

Live editing covers unsaved reads; plan/replay/conflict; invalid overlapping
edits; no-op revisions; one undo; external disk conflict; exact BOM/CRLF/Unicode
save; MCP writes; AI badge and blocked human edits with other tabs editable;
client death; cancellation while tab manager is open; typed overload; save
client death while Settings is open; transaction lookup and credential cleanup.
Rust tests cover pre-start cancellation/timeout versus started commit results,
full UI queues, worker unwind cleanup, source/review locks and Windows junction
or Unix symlink escape rejection. No host-application certification is claimed.

The concurrent probe times fresh PTY screen feedback for navigation, blocked
edits, dialogs, switching/new tabs and editing another tab while MCP analyzes
a 10 MiB JSON array. It requires at least 20 genuinely concurrent inputs and
fails any response >=100 ms. API completion is measured separately. Run it
without concurrent builds, audits or other performance probes.

## Risk mitigation checkpoint — 2026-11-10

Run locally after `cargo build --workspace --release`, with no parallel build,
audit or competing performance probe:

```sh
python scripts/pty_recovery.py
python scripts/pty_navigation.py --trace
python scripts/pty_risk_stress.py
```

Recovery runs English and Vietnamese through two real process kills per locale,
checks committed draft content, changes the original externally, restarts again,
Save As exact BOM/NFD/ZWJ/mixed-newline bytes, and deliberate clean exit/cleanup.
This is a process-crash check, not a power-loss test. Unit tests separately cover
active session locks, malformed/checksum/quota failures, unseen drafts,
source metadata changes and recovered Agent secret/workspace restrictions.

Navigation includes twelve fresh sessions: single long Unicode line, ASCII
prefix/bare-CR suffix and a second long Unicode line after editing its short
predecessor. Raw stage CSVs include timestamps and dropped sample counts.
Latency includes ConPTY transport/pyte parsing, excludes physical paint.
Opening completion is measured separately. Historical 916.326 ms failure remains
an open diagnostic; repeats passing do not erase it.

History stress exceeds the 2048-transaction default on a <=10 MiB fixture,
then times a new key, Undo/Redo and F9 open/close feedback. Bulk 2400-key time
is throughput, not one interactive response. A 5 ms psutil observer samples RSS;
observed peak may miss shorter spikes and is not a hard memory bound.

All previous smoke/workspace/tabs/dialog/mouse/search/Agent/MCP probes remain
relevant. Record each exit code and preserve raw JSON with the current binary
hash in docs/diagnostics/risk-after/. Native Unix/ARM execution, real disk full,
power loss, privileged SACL/xattrs and hostile-writer CAS remain separate gates.

The PTY emulator now answers terminal status/cursor reports through pyte's
write_process_input callback. Its default no-op caused a reproducible ~500 ms
ConPTY timeout during cooked-mode restoration. Preserve pre-fix evidence;
this does not establish the cause of the historical cold End outlier.
