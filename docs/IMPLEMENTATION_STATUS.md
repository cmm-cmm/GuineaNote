# Implementation status

## Initial audit

Only the two supplied Markdown documents existed. No Git repository was
initialized and no user changes were removed. M0–M4 plan recorded before code.
Local host: Windows x64 / MSVC, Rust 1.98.1 stable.

## Development Git policy

The user authorized committing/pushing the project and explicitly requested no
GitHub CI during development. The initial GitHub Actions workflow was removed
before publication; format, lint, tests, PTY checks and dependency audits remain
local commands. Build outputs, Python caches and local environment files are
excluded from version control. Older CI/Git statements below are historical
milestone notes, not the current development policy.

## Latest checkpoint — Agent CLI/MCP and AI read-only tabs

Continuing polish: 99 Rust tests pass; native and Linux/macOS cross-target
strict Clippy, debug/release builds and native UI/Agent regression probes pass.
The new release corpus has **282 UI responses, max 71.884 ms**, up to 10 MiB.
Six prepared first-End responses span 3.146–10.768 ms, with opening/preparation
reported separately. Core prepares the first-line display index in the open
worker and preserves complete tab-free indexes across safe small ASCII edits.
The warm accent is honey amber; navy backgrounds are unchanged. Context badges,
localized AI operation, disabled close affordances, cached tab metadata and
selection/progress status are implemented. See UX_IMPROVEMENTS.md and
PERFORMANCE.md for current details; the P0–P4 checkpoint below is historical.

P0–P3 now have a shared typed service, separate JSON CLI and MCP stdio binaries,
explicit live-session IPC, workspace/capability policy, unsaved snapshots,
patch plan/diff/apply, one-step undo, revision-checked save and bounded retry
receipts. The source and linked review show AI and temporarily become read-only
while leased; other tabs remain editable. P4 adds native Windows isolation,
cancellation, overload, path/junction, secret and malformed-input checks.
See AGENT_IMPLEMENTATION.md for current phase evidence and platform scope,
and AGENT_USAGE.md for dev commands. No GitHub CI was added.

P0–P4 pass native Windows validation: 96 Rust tests, strict native and
Linux/macOS cross-target Clippy, builds and dependency audit. The 10 MiB
release corpus has 250 UI samples, max 98.483 ms; its 28 concurrent Agent
samples max at 27.395 ms. Native Linux/macOS and named external MCP host
applications remain unverified. All work is on dev; main is unchanged.

## Previous checkpoint — visible tab management and Agent/MCP assessment

Added local New/Save/Close/Previous/Next/All tabs controls, active/total count,
unsaved state, numbered blank names and per-tab close buttons. Ctrl+P opens
the searchable manager with paths and states; Ctrl+T creates a blank, and
Ctrl+PageUp/PageDown switches. Closing an inactive tab preserves selection;
closing an active linked review returns to its source. Dirty source close
requires confirmation and closing the final tab leaves a blank buffer.

84 Rust tests, strict Clippy and release gates pass; native tab controls pass
26 measured responses (max 36.585 ms), and the extended 10 MiB corpus passes
196 (max 53.054 ms). JSON preview completion is 448.020 ms separately.
See TAB_MANAGEMENT_PLAN.md and PERFORMANCE.md for evidence and scope.

AGENT_MCP_PLAN.md records a separate assessment and phased proposal for a
typed service, CLI JSON, live-session bridge, MCP stdio and revision-checked
patch/review/save. It cites current primary protocol sources and estimates
12–20 engineering days subject to validation. Agent/MCP runtime remains
unimplemented; no endpoint, new dependency or GitHub CI was enabled.

## Previous checkpoint — Settings, file navigation and linked reader

F8 now opens a language Settings dialog with explicit Apply/Cancel and
asynchronous persistent preferences. Ctrl+O shows a filtered file/directory
picker; F9 shows active-file metadata and background text statistics. F7 offers
split review or a linked read-only review tab, reused per source and synchronized
after edits, save and reload. Review tabs cannot mutate source/history; closing
one preserves source edits, while closing the source also closes its reviews.

The Markdown reader adds heading cards, a navigable contents sidebar, reading
and task statistics, callouts, aligned tables, link references and numbered
code cards. Syntax coloring distinguishes functions, types, constants,
properties, attributes, escapes and operators. Source bytes are never formatted
or executed. See UX_IMPROVEMENTS.md for behavior and verification evidence.

Local validation: 80 Rust tests, required format/strict Clippy/release gates,
native dialogs/edit/workspace/panic probes, Linux/macOS cross-Clippy and
dependency audit pass. Two 10 MiB release PTY budget runs cover 374 interactions:
max 58.551 ms, none >=100 ms; full background JSON review takes 419.093 /
406.744 ms separately. Raw data and limits are in PERFORMANCE.md. Native
Linux/macOS execution remains unverified. No GitHub CI.

## Previous checkpoint — ENV/config and universal UTF-8 source review

Added structured YAML (multiple documents/aliases), JSON5/JSONC, Jupyter
nbformat 4 cells/saved text outputs, ENV, INI and Properties. Config names and
extensions infer XML/JSON5/YAML/INI or retain source review for unknown dialects.
ENV values are literal: export/quotes/multiline/comments/duplicate keys are
shown without environment mutation, interpolation or command execution.
Section/key/value review includes source line numbers and preserves duplicates.

All UTF-8 source files can open/edit/review. Named aliases now additionally
cover Dart/Zig/R/Julia/Scala/Haskell/Clojure/Elixir/Erlang/F#/OCaml/Perl/Lisp,
Fortran/Assembly/Pascal/HDL, CMake/Nix/HCL/Proto/Solidity and web/template types.
Unknown extensions have generic lexical colors; extensionless script shebangs
are recognized. This is editable text/lexical support, not every grammar or LSP.

Performance: ordered token painting is linear; repeated ignored typing in
read-only review does not force repeated frames. The bounded input, viewport,
Unicode index and asynchronous parsing protections remain in place.
Final expanded native budget: 276 samples across two 10 MiB release runs,
max 57.719 ms (<100 ms). Full background JSON review: 777.682 / 779.545 ms.
Raw results/earlier failures are in PERFORMANCE.md. 69 Rust tests and required
checks pass; runtime on Linux/macOS is unverified despite cross-Clippy passing.

Dependency audit rejected the initial libyaml-based YAML parser. Replaced it
with serde-saphyr 2.1.1 and an ordered generic Serde value, retaining parser
budgets and rejecting unsupported tags; no include/property resolver is used.
The reviewed BSD-3-Clause allowance is scoped to encoding_rs only. Final
advisories/bans/licenses/sources pass; no advisory was waived. No GitHub CI.

## Previous checkpoint — responsiveness, review and Vietnamese/English, 2026-11-09

The confirmed benchmark scope is UTF-8 text up to 10 MiB. The user's final
interaction target is **strictly below 100 ms**, superseding the initial 150 ms
request. Local release PTY probes fail at or above 100 ms. Actual disk sync and
full review parsing have separately reported completion times; no universal
bound is claimed for arbitrary files, clusters, filesystems or hardware.

Delivered: chunk-aware grapheme navigation; O(1) history-state dirty queries;
bounded viewport extraction; sparse, incrementally invalidated display indices
for long Unicode lines; parallel cold-index construction over grapheme-safe
segments with correct global tab stops; 8 ms polling while workers are active;
bounded input batching to avoid a redraw per character in bursts. Idle work
returns to 250 ms waits and does not redraw. Saved-snapshot metadata merges
without dropping edits made during I/O.

F7 cycles source/split/read-only review. Markdown renders styled content;
JSON/JSONL, XML, CSV/TSV and TOML have structured/formatted review. YAML/INI
have highlighted source review. Parsing/wrapping is asynchronous, revision-
and viewport-keyed, stale results are discarded, and source bytes/history are
never replaced by preview output. CSV/TSV supports later columns via Left/Right.
Textual programming/data detection now spans 30+ language/format categories.
See FORMATS.md for exact capabilities and parser/display limits.

Vietnamese and English UI are selected with `--language vi|en`, configuration
`language`, F8, or `language-vi`/`language-en` commands. OS/parser diagnostic
technical details can retain their original language. Filenames, source and
command identifiers are not translated. Screenshots come from actual PTY
sessions via scripts/tui_review_preview.py; no runtime image/font dependency.

Final validation: 64 Rust tests passed, required fmt/Clippy/release build passed,
and two native 10 MiB budget runs passed (214 samples, max 49.018 ms).
Full JSON review completion was 790.227 / 778.185 ms in background. Normal
workspace/edit and panic cleanup PTY probes passed; Linux/macOS cross-Clippy
passed, native runtime there remains unverified. Exact latency evidence is in PERFORMANCE.md and
DEPENDENCY_AUDIT.md. GitHub CI remains disabled as requested.

## Previous checkpoint — visual workspace redesign, 2026-11-09

The user explicitly requested a more visual TUI and both folder/file modes.
This authorizes directory browsing earlier than the original post-MVP scope;
other M5 features remain deferred. See UI_REDESIGN.md for the revised design.

Delivered: navy/mint theme, portable ASCII welcome wordmark and actionable
welcome screen, toolbar, lazy directory tree, neighboring file tabs,
breadcrumbs, focused pane borders, current-line highlight, compact layouts,
clear status/shortcuts, and themed dialogs. No Nerd Font or image runtime.

- `cargo run -- .` opens a directory workspace. No arguments show cwd/welcome;
  file arguments open standalone mode, and folder+file arguments are supported.
- Ctrl+K opens a folder; Ctrl+B toggles the tree; F4 switches pane; F5 refreshes;
  tree arrows/Enter browse/open, Tab/Esc returns to editor. Mouse clicks work
  on tree rows, tabs, toolbar/welcome actions and grapheme-safe text positions.
- One asynchronous directory scan at a time; children load only on expansion,
  symlinks are shown without traversal, old-root results are rejected, and the
  10,000-entry directory cap is visible. Folder changes keep dirty tabs/history.
- Mouse capture is optional via config and restored on exit/panic. Cleanup
  attempts are separated so an unavailable mouse state cannot prevent terminal
  restoration. The panic probe now activates mouse capture too.
- The sidebar hides below 66 columns. A compact layout retains document space
  in short terminals. Config adds sidebar_width (22–50), mouse and basic_colors;
  NO_COLOR remains respected. Preview-only child processes enable colors.

Latest results: **51 Rust tests PASS**, none ignored; format check, clippy
warnings-as-errors and native release build PASS. Linux x86_64/macOS ARM64
cross-target clippy PASS (native execution still NOT TESTED). The original
editing/save PTY probe and the workspace probe PASS on Windows ConPTY. The
workspace probe verifies expand/collapse, actual file save, pointer toolbar and
caret activation, focus/toggle, and preservation of dirty tabs across folders.

Real PTY screenshots/text were captured and visually reviewed:
`target/previews/guineanote-welcome.png` and
`target/previews/guineanote-workspace.png`. Reproduce with
`python scripts/tui_preview.py` (dev-only Pillow). Runtime still needs no Python.
Latest binary: `target/release/guineanote.exe`; SHA-256:
`f786bc86121d055f08fe7f56820d4ecc51641071db729e01a5bedbc34bcc8c5b`.

The earlier measured performance and advisory records below remain historical
evidence; the new chrome/tree has not been rebenchmarked for all corpus sizes.
Filesystem atomicity, no recovery, actual disk-full/power-loss tests and native
Linux/macOS acceptance remain open. No public release or remote CI was triggered.

## Earlier checkpoint — M4 continuation, 2026-11-09

M4 made further progress without adding M5 features:

- Native Windows tests now provoke a real replacement sharing violation and a
  real NTFS directory ACL denial. They verify original bytes/dirty state and
  successful saving after the restriction is removed. ACL changes affect only
  a newly-created test directory and are restored with a scope guard.
- Five child saves are forcibly killed at create/partial-write/flush/sync/
  pre-replace barriers; original BOM/Unicode/mixed-ending bytes remain exact.
  This does not interrupt the replacement syscall or simulate power loss.
- Core navigation uses contextual GraphemeCursor operations rather than an
  allocated boundary list for every scalar in a line. Coordinate/position
  conversions stop at the cursor prefix. Syntax stops at the viewport's right
  edge while retaining line-local prefix context.
- An added Unicode property compares full forward/backward navigation against
  unicode-segmentation. The suite has **42 passing tests**, none ignored,
  including one inert child-probe entry point, four core properties (256 cases
  each) and a 64-case I/O property.
- For a single ASCII line of 1 MiB, Start→Right→Left+position p95 improved from
  841.542 ms to 5.365 ms on this host. Native typing into a single-line 1 MiB
  Rust file measured p95 36.740 ms including PTY/client overhead; the <20 ms
  target is **not established**. See PERFORMANCE.md and raw CSV/JSON evidence.
- ConPTY smoke now verifies find/replace and save-as of actual copied bytes,
  and checks current screen state rather than accepting old transcript text.
  The Unix helper now establishes a controlling terminal for /dev/tty/resize;
  that path has not been executed here.
- Cross-target all-target checks and clippy with warnings-as-errors PASS for
  Linux x86_64 and macOS ARM64. These are compile/type/lint checks from Windows,
  not native linking, execution or OS acceptance.
- Fresh RustSec fetch and workspace cargo-deny 0.20.2 checks PASS. Audit exposed
  missing version requirements on workspace path dependencies; explicit
  versions fixed the issue without weakening wildcard/advisory/license checks.
  One Syn 2/Syn 3 duplicate-version warning remains. See DEPENDENCY_AUDIT.md.

Latest local gates: required fmt, clippy, workspace tests and release build all
PASS. Native normal/panic PTY checks PASS. The release binary remains
`target/release/guineanote.exe`; its adjacent SHA-256 file is authoritative.
Current SHA-256:
`eef0dd39a5e1580abbef001b852df4dfdb5509a6eab3d3381ff435a36869822c`.

WSL and Docker are unavailable here, so Linux/macOS runtime checks remain
NOT TESTED. Native Windows symlink creation was attempted and refused by the
OS with WinError 1314 (missing privilege). No elevation/system-setting change
was attempted. Disk exhaustion, interruption during replace, power loss,
network filesystems, human UX/SSH and far-right long-line latency remain open.

The following milestone notes and original gate table record the earlier
2026-11-08 checkpoint. This current section and acceptance matrix supersede
their counts and then-unavailable audit tool.

## M0 Foundation / M1 Text Core — implemented, Windows verified

Workspace, CLI, MIT/README, CI matrix and terminal guard implemented. Core is
independent of the UI, with checked coordinate conversions, grapheme selection,
editing, literal search/replace, readonly enforcement, snapshot undo/redo and
content savepoints. At the first checkpoint nine core tests passed, including
two properties. Final core suite: 11 PASS, including three properties (256
generated cases each) and a bare-CR navigation regression. Windows ConPTY boot,
normal exit and panic cleanup now PASS. Human Windows Terminal/PowerShell UX,
Linux and macOS: NOT TESTED. No cross-platform completion is claimed.

## M2 File Safety — implemented, local fault model verified

Strict UTF-8/BOM, byte-preserving endings, SHA-256 conflict detection,
same-directory temporary writes with sync and no-delete replacement,
readonly/large-file/symlink policies implemented. First checkpoint: seven I/O
tests passed. Final I/O suite: eight PASS locally, including a 64-case property:
golden roundtrips, rejected binary/encoding, conflicts, save-as, creation races,
injected partial-write/stage failures and readonly/large-file errors. At the
first M2 checkpoint 16 total tests passed. Unix symlink test NOT TESTED here.
Actual full disk/process-kill/network filesystem behavior: NOT TESTED.

The final fingerprint precheck rejects obvious metadata changes before reading
and caps hash reads to expected size+1. Replacement TOCTOU, platform metadata and
power-loss limitations are recorded in FILE_SAFETY.md.

## M3 Usable TUI — implemented, native Windows PTY verified

Implemented tabs, internal clipboard, selection, key shortcuts with F2 command
fallback, F1 help, F6 tab switching, literal find/replace-all, dirty close/quit
and reload confirmation, TOML config, cell-aware scrolling, control sanitization,
and background open/save/search/replace. Edits pause during a job; rendering and resize
remain active. Mouse is optional and not enabled. Seven TUI/config/render tests
passed at this checkpoint (23 total). Final TUI suite has 11 PASS, including
worker multi-file isolation, conflicts and Windows input regression tests.

ConPTY testing exposed dropped supplementary Unicode characters in Crossterm
0.29's native Windows surrogate/key-up path. Added a safe crossterm_winapi
adapter, bounded event channel and regression tests; native typed Vietnamese
and emoji byte comparison now PASS. Core never required lossy conversion.

Final PTY run verifies typing/saving, clipboard, tabs, undo/redo, dirty quit
cancellation, resize, F1/F2 fallbacks and successful quit. Wrapper compares
console modes before/after. Separate intentional panic probe verifies cleanup
and nonzero child exit. Both PASS. Final release smoke after the long-path
status regression fix also PASS. Idle sample: 2.010 s wall time, 0.000 s CPU
at sampling precision, zero new terminal output, RSS 5,316,608 bytes. This is
short smoke evidence. Human UX and other OSes remain NOT TESTED.

## M4 Hardening — implementation delivered; cross-platform signoff pending

Basic revision-tagged line syntax for Rust/Python/JS/TS/JSON/TOML/Markdown;
three-platform CI including PTY/panic probes; CLI and real filesystem workflow
integration tests; four property tests; byte goldens; reproducible 1 KiB/1 MiB/
10 MiB/100 MiB benchmarks; all required architecture/behavior/safety/test/
performance documentation; native release binary and Cargo.lock delivered.

Earlier local gates, 2026-11-08, Windows 11 x64 / Rust 1.98.1 MSVC:

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS — 36 tests, 0 failed, none ignored |
| `cargo build --release` | PASS — `target/release/guineanote.exe` |
| release `--help` / `--version`, noninteractive CLI errors | PASS (CLI integration + version smoke) |
| `python scripts/pty_smoke.py` | PASS, native Windows ConPTY |
| `python scripts/pty_smoke.py --panic target/debug/examples/panic_cleanup.exe` | PASS |
| `cargo bench --bench editor` plus 100 MiB opt-in | PASS, measurements in PERFORMANCE.md |
| dependency license metadata review with `cargo tree` | No GPL entries in the local dependency tree; own code written here |
| local cargo-deny / live advisory audit | NOT TESTED — cargo-deny unavailable; CI job configured |

Earlier release checksum (superseded above) was:
`716eecb7d5b4e52a19cc8ed421093c3a544577e30fed691656b9126c1b55726a`.
The status now keeps coordinates/encoding before long paths, and the tab strip
starts at the active tab to keep the active label visible.

Changes are in root Cargo/CLI/license/README/config, the four workspace crates,
`tests/`, `benches/`, `scripts/`, `docs/`, `.github/workflows/ci.yml` and
`deny.toml`. No commits: the initial directory is not a Git repository. No
package publication, public release, shell alias or M5 runtime was created.

## Acceptance criteria

PASS below means the listed local evidence, not certification on every OS.

| ID | Status | Evidence / remaining qualification |
| --- | --- | --- |
| AC-01 | PARTIAL | Windows ConPTY open/exit PASS; Linux/macOS cross-check/clippy PASS, native execution NOT TESTED |
| AC-02 | PASS locally | Core properties, TUI command tests, native PTY editing/clipboard/history |
| AC-03 | PASS locally | Two-file worker and filesystem workflow tests isolate text, paths and history |
| AC-04 | PASS locally | Byte goldens and 64 generated valid UTF-8 save roundtrips |
| AC-05 | PASS locally | BOM-only/BOM/mixed/LF/CRLF/bare-CR byte comparisons |
| AC-06 | PASS locally | Grapheme fixtures/properties and surrogate regression |
| AC-07 | PASS in tested fault model | Injected stages, actual NTFS sharing/ACL errors and five pre-commit process kills; full disk/replace interruption/power loss NOT TESTED |
| AC-08 | PASS locally | Existing target content/metadata conflicts and concurrent changes before replace; final TOCTOU caveat remains |
| AC-09 | PASS locally | Command tests + native dirty quit/cancel/discard |
| AC-10 | PASS locally | TestBackend tiny resize, ConPTY resize and native normal/panic mode restoration |
| AC-11 | PASS smoke | Blocking input + timeout wait; measured idle CPU/output, not sustained profiling |
| AC-12 | PASS locally | Direct native binary smoke; no Node dependencies/runtime calls |
| AC-13 | PASS source/license review | Original MIT implementation; no Fresh source imported; cargo-deny license/source checks PASS |
| AC-14 | PASS locally | Required gates, 51 tests; Linux/macOS cross-target clippy PASS; remote CI NOT EXECUTED |
| AC-15 | PASS locally | Warning-confirmation test, persistent resident-memory help, documented benchmark/limits |

## Remaining work and risks

Before calling v0.1.0 fully accepted: run native validation locally on Windows/
Linux/macOS; validate Unix symlinks and native human UX/SSH; test full disk and
replacement/power-loss stages; broaden latency/memory profiling beyond the
measured single-line start and microbenchmark cases. Live advisory/license
checks, native NTFS ACL/sharing failures and pre-commit process kills have now
been checked locally as documented above. Existing
metadata/atomicity caveats, input pause during jobs, basic line-only syntax and
resident-memory/history costs are explicit. No recovery or 1 GB support claim.

Next milestone is completion of M4's platform and filesystem validation before
any public release. M5 features remain out of scope.
