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

## Latest checkpoint — visual workspace redesign, 2026-11-09

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
