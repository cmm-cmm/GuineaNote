# Implementation status

## Checkpoint 13/11/2026 (11) — v0.1.1: GitHub Release and cargo install --git

Owner decision 13/11: for a new project, ship GitHub Releases and
`cargo install --git` first, then winget and npm/npx; Homebrew deferred;
Scoop and AUR dropped.

| Commit | Change |
| --- | --- |
| e693766 | README screenshots from the real editor (`scripts/readme_screenshots.py`); Windows paths shown without the `\?\` prefix (`paths.rs`) |
| 5bc94fe, 96ea536, 5d2522d | Release channels re-planned per platform |
| a7f8b95 | Dialogs lose the mouse-only corner close mark; Esc closes every dialog |
| 158ec99 | Screenshots re-rendered after all tabs open |
| 1bc0998 | Version 0.1.1; README "Install with Cargo"; welcome screen and `tests/cli.rs` take the version from Cargo (the welcome screen showed a hard-coded v0.1.0) |
| 6b698d9 | Release gate evidence — tagged **v0.1.1**, `main` fast-forwarded |

Gate on 1bc0998 (sequential):
- Windows 21/21 steps: PTY max 25.5 / 28.7 ms, navigation max 25.7 / 21.8 /
  24.5 ms.
- Linux container: 207 tests, 29/29 steps, PTY max 6.6 / 8.1 ms, navigation
  max 9.5 / 8.7 / 10.5 ms.
- No interaction above 50 ms.
- Correction: the v0.1.0 record says 32 Linux steps; the suite has 29.

Release:
- **GitHub pre-release v0.1.1**: both archives checked unpacked (Windows
  system-DLL imports and PTY smoke; Linux static-pie, PTY smoke and four
  distributions), anonymous download re-verified.
- **`cargo install --git --tag v0.1.1`** verified in a clean Linux container
  and from GitHub on Windows. On Windows it links the VC runtime dynamically,
  as the README says.

Evidence: [release-0.1.1-gate](diagnostics/release-0.1.1-gate/manifest.json),
[release-0.1.1](diagnostics/release-0.1.1/manifest.json).
Next: winget PR, npm publish (owner's login).

## Checkpoint 13/11/2026 (10) — public repository and v0.1.0 release

Owner decisions 13/11: D1 public repository, D2 builds by hand (no GitHub
Actions), D4 names (`guinea-note`, `@guinea-note/*`, `cmm-cmm.GuineaNote`),
D5 `v0.1.0` as a pre-release labelled preview, D6 SHA-256 only. GitHub Release
first; winget and npm are prepared but not submitted.

| Commit | Change |
| --- | --- |
| c45741a | Quit/Close (keys, F2, tab close buttons) pressed during a save wait for it (Esc withdraws; a failed save closes nothing and says so) instead of being refused; `pty_typeahead.py` covers Ctrl+S Ctrl+Q on 9 MiB |
| 231dadc | README rewritten for the public (features, install, docs index); CONTRIBUTING, CODE_OF_CONDUCT (Contributor Covenant 2.1), SECURITY, SUPPORT, issue forms, PR template, CODEOWNERS; release tooling `scripts/release/{package.py,linux-build.sh,linux-check.sh,winget.py,npm.py}`, `packaging/` npm launcher; `scripts/windows/verify.sh` |

Release gate on 231dadc (sequential): Windows 11/ConPTY 213 Rust tests,
rustfmt, strict Clippy, 21/21 suite steps with the real clipboard, PTY max
23.8 / 26.0 ms, navigation max 22.5 / 22.4 / 29.0 ms on 12/2/1 CPUs; Linux
Debian 12 container 206 tests and all 32 steps (musl build and smoke), PTY
max 6.5 / 7.3 ms, navigation max 9.1 / 6.6 / 8.0 ms. No interaction above
50 ms. Evidence: [docs/diagnostics/release-0.1.0-gate](diagnostics/release-0.1.0-gate/manifest.json).

| Commit | Change |
| --- | --- |
| 7fc1ad8 | Release gate evidence; scripts marked executable |
| 8120df7 | README rewritten (banner, badges, install, configuration, platforms), mascot icons in `icon/`, old Codex prompt removed — tagged **v0.1.0** |

Release: repository made public (secret scanning and push protection on,
GitHub Actions disabled, private vulnerability reporting on, community
profile 100%); `main` fast-forwarded to `8120df7`; GitHub **pre-release
v0.1.0** with the Windows zip, the Linux tar.gz, `SHA256SUMS` and
`release-manifest.json`. Both archives checked unpacked (Windows imports +
PTY smoke; Linux static-pie + PTY smoke + Debian 11/12, Ubuntu 22.04, Alpine
3.20), downloaded anonymously and re-verified. Linux binaries are
reproducible (two builds from different tag commits were byte-identical).
winget manifest validated and npm packages tested from tarballs, neither
submitted. Evidence: [docs/diagnostics/release-0.1.0](diagnostics/release-0.1.0/manifest.json).
Open: winget PR, npm publish, M9 features (P7), ARM, real Linux terminal
emulators, Wayland, SSH/tmux.

## Checkpoint 13/11/2026 (9) — responsiveness (research 09) and release prep

Owner decisions 13/11: release **Windows x64 and Linux x64 only** (macOS not
released, not verified); Linux is verified with Docker Desktop.

| Commit | Change |
| --- | --- |
| c8caf5b | Session restore on Windows kept no caret/active tab (verbatim `\\?\` paths never matched); found running the session tests on Windows for the first time |
| fbc54ba | P1 typing queued during jobs (1,024 events / 4 MiB, in order, held while a job's dialog is answered); P2 a job pauses only its tab, one worker job at a time; P3 Undo charged by inserted bytes (6 whole-file steps of 10 MiB in 64 MiB, was 3); P4 block tools splice the changed middle; P5 Esc cancels large line/case/comment tools; whole-document line tools leave the caret and the final newline |
| c4c35a8 | Workers start after the frame flush (a 12 MiB open preempted the prompt answer's terminal write: 53.7 → 15.6 ms on one CPU) |
| b66f132 | Probe races: `pty_agent.py` second PTY reader unlocked on Windows (22-minute hang), `pty_typeahead.py` quit before the save receipt |
| a7f1523 | Release prep: Windows static CRT, static musl Linux build (Debian 11+, Ubuntu 22.04, Alpine), third-party notices generator, Cargo metadata, README install section, CHANGELOG, `docs/RELEASE.md` |

Verification (product code of c4c35a8, sequential runs):

- Windows 11 x64, ConPTY, static CRT: 212 Rust tests, rustfmt, strict Clippy;
  21/21 probe steps including the new `pty_typeahead.py` and the real
  clipboard; interaction corpus 1,540 PTY samples per run, PTY max 27.7 /
  27.9 ms (trace on/off), application max 3.9 ms; navigation max 24.0 / 23.1 /
  27.4 ms on 12/2/1 CPUs; no investigation, 0 dropped rows.
- Linux x86_64, Debian 12 container (Docker Desktop): 205 Rust tests, all 29
  steps of `scripts/linux/run.sh` (21 probes, musl build + musl smoke); PTY
  max 6.5 / 6.6 ms, application max 5.6 ms; navigation max 8.5 / 9.8 / 6.5 ms.

Evidence: [docs/diagnostics/responsiveness-release-prep](diagnostics/responsiveness-release-prep/manifest.json).
Open (at the time): release decisions D1/D2/D4–D6, M9 features (P7), ARM,
real Linux terminal emulators, Wayland, SSH/tmux; Quit/Close during a save
(fixed in checkpoint 10).

## Checkpoint 12/11/2026 (8) — UI separation polish

| Change | Detail |
| --- | --- |
| Menu | Full labels whenever they fit beside the help hints (Settings F10, Info F9 at 100 columns instead of bare keys); keys drawn as bold accent keycaps; right margin |
| Tab bar | Dividers between tabs, dimmed inactive close buttons, keycap keys on the tab controls |
| Footer / F2 / Find | Footer hints as keycaps; F2 right-aligns as many key bindings as fit; search results colour the match |
| Border | One border cell between the unsaved count and the context chip |
| Probe | `pty_agent.py` drains the PTY while blocked in Agent calls (see TEST_PLAN) |

Verification (Linux x86_64 cloud container, Python PTY + pyte): 196 Rust
tests, rustfmt/strict Clippy (1.98.1); all 12 functional probes pass (agent
3/3 after the probe fix, also 3/3 on 83e6953); interaction corpus 1,540 PTY
samples, max 6.9 ms, application max 6.2 ms, 0 above 50 ms. Windows not re-run.

## Checkpoint 12/11/2026 (7) — edit responsiveness and research 09

| Change | Detail |
| --- | --- |
| Block edits | Indent/comment rewrite the block in one pass (433 → 30 ms, 328 → 17 ms on 10 MiB); line tools use borrowed slices (sort 111 → 54 ms); line tools, case and comment above 1 Mi scalars run on the compute worker |
| Up/Down | Goal column from a bounded prefix or cached index (133 → 0.01 ms into a 10 MiB Unicode line); queued while the caret line's index is built, never overtaking later input |
| Research | [09 — responsiveness and editing proposals](research/09_RESPONSIVENESS_AND_EDITING_PROPOSALS.md): input dropped during background jobs (P1), app-wide job pause (P2), Undo charged twice (P3) |

Verification on 82d46f5 (Linux x86_64 cloud container, Python PTY + pyte):
196 Rust tests, rustfmt/strict Clippy (1.98.1); 12 functional probes pass;
interaction corpus 1,540 PTY samples, max 9.4 ms, application max 4.6 ms;
unindexed long-line navigation corpus max 6.1 ms; 0 samples above 50 ms.
Windows not re-run.

## Checkpoint 12/11/2026 (6) — regex Find/Replace (M9), tab bar

| Change | Detail |
| --- | --- |
| One-row tab bar | Tabs left, count + controls right, path/context in the editor border; the editor gains four rows. Control labels shorten first so every tab shows; ‹ / › mark hidden tabs |
| Background | A darker palette was tried and reverted at the owner's request; colours are unchanged |
| Regex | Alt+R / `[ ] Regex` in Find/Replace: rust-lang `regex` (linear time), `^`/`$` per line with CRLF, Match case / Whole word still apply, empty and grapheme-splitting matches skipped, 4 MiB program cap, `$1`/`${name}` in replacements, invalid patterns reported as `Regex: …`; regex-mode size estimate fixed |

Verification (Linux x86_64 cloud container, Python PTY + pyte; tests on Rust
1.99.1, rustfmt/strict Clippy on 1.98.1): 194 Rust tests; `cargo deny check`
advisories/bans/licenses/sources ok; new `pty_regex.py` plus search, smoke,
keyboard, tabs, dialog/mouse, dialogs/reader, mark, session, recovery,
terminal-matrix and agent probes pass. Interaction corpus: 1,540 PTY samples,
max 8.5 ms; application max 4.6 ms; 0 samples above 50 ms, 0 dropped rows.
Windows not re-run.

## Checkpoint 12/11/2026 (5) — Mark All (M8)

| Change | Detail |
| --- | --- |
| Mark | Notepad++ Search › Mark All from F2: `mark-all` marks every occurrence of the one-line selection, else the caret word (NFD-safe), else the Find query with the current Match case / Whole word options; matches in the visible text are painted reversed warm (reverse video under NO_COLOR) until `mark-clear`. `mark-bookmark` bookmarks every line with a match on a worker (merged only at the same revision, at most 100,000 lines), feeding `bookmark-copy` / `bookmark-remove-lines` |

Verification (Linux x86_64 cloud container, Python PTY + pyte; tests on Rust
1.99.1, rustfmt/strict Clippy on 1.98.1): 187 Rust tests; new `pty_mark.py`
plus smoke, keyboard, tabs, search, dialog/mouse, session and terminal-matrix
probes pass. Interaction corpus: 1,540 PTY samples, max 10.3 ms; application
max 4.1 ms; 0 samples above 50 ms, 0 dropped rows. Windows not re-run.

## Checkpoint 12/11/2026 (4) — session restore (M8)

| Change | Detail |
| --- | --- |
| Session | Notepad++-style "remember current session": a normal exit writes `session.toml` (open file tabs, caret line + grapheme column, active tab; paths only). A launch without file arguments reopens them through the normal loader; named files win; missing or unreadable files are skipped and reported; `session = false` disables reading and writing. See FILE_SAFETY.md |
| Test fix | `bookmarks_smart_highlight_and_brackets_are_painted` no longer depends on TERM/NO_COLOR (it failed under TERM=linux, which selects ANSI-16) |

Verification (Linux x86_64 cloud container, kernel 6.18, Python PTY + pyte,
Rust 1.99.1 for tests and 1.98.1 for rustfmt/strict Clippy): 181 Rust tests;
new `pty_session.py` (5 cases) and the existing smoke, panic cleanup, recovery,
tabs, keyboard, dialogs/reader, search, dialog/mouse and terminal-matrix probes
pass. Interaction corpus: 1,540 PTY samples, max 13.8 ms; application warm max
5.1 ms; 0 samples above 50 ms, 0 dropped rows. Windows was not re-run for this
change; the input path is unchanged (session work happens at startup and exit).

## Checkpoint 12/11/2026 (3) — bookmarks, smart highlighting, native Linux

| Commit | Change |
| --- | --- |
| 79073e9 | Bookmarks (Ctrl+F2 / gutter click, Alt+F2 / Shift+F2, F2 copy/remove/clear) following edits, Undo and Redo; smart highlighting of the selected word in visible lines; bracket pair highlight (64 Ki-scalar window) |
| next | Native Linux x86_64 verification (`scripts/linux/`); fixes it found: Unix-only Clippy error in `clipboard.rs`, duplicate recovered drafts after a crash during checkpoint handoff, `pty_agent` lease-wait race; live clipboard probe `pty_clipboard.py` |

Verification on that tree, sequentially:

- Windows x64 MSVC release, ConPTY: 182 Rust tests, rustfmt, strict Clippy;
  all 17 probe steps pass including the real system clipboard (copy and paste
  byte-exact, previous text restored). Interaction corpus 1,540 PTY samples per
  run, application max 4.4 ms, PTY max 27.4 / 29.5 ms trace on/off; navigation
  max 21.0 / 23.8 / 27.5 ms on 12/2/1 CPUs; nothing above 50 ms, 0 dropped rows.
- Linux x86_64, Debian 12 container (WSL2 kernel 6.6), Python PTY: 176 Rust
  tests (5 repeated runs), rustfmt, strict Clippy; all 17 probe steps pass,
  clipboard through xclip on a private Xvfb display. PTY max 6.4 / 6.2 ms,
  application max 4.2 ms; navigation max 7.6 / 6.8 / 6.6 ms on 12/2/1 CPUs.

Evidence: [docs/diagnostics/linux-container](diagnostics/linux-container/manifest.json).
Open: macOS, ARM, real Linux terminal emulators, Wayland clipboard, SSH/tmux;
Up/Down from a pending long line; startup resize on one pinned CPU (Windows).

## Checkpoint 12/11/2026 (2) — folder removal, key/mouse sync, M7 tools

| Commit | Change |
| --- | --- |
| 87fd3a3 | Folder features removed (explorer, Ctrl+K/B, F4/F5, Ctrl+E/G project search, `guinea <folder>`); key labels on header/toolbar/footer/welcome generated from the keymap; tests for every clickable region and for key names in guides/UI strings |
| 29177fe | F2 line/case/comment tools (sort, dedupe, remove empty, trim trailing, UPPER/lower/Title, toggle comment); Ctrl+G joins Ctrl+L for go to line |
| c6234bb | Go to line resolves the column from a bounded prefix; start-of-line queries on unindexed long lines skip the full-line copy |

Verification on c6234bb (Windows x64 MSVC release, ConPTY): 177 Rust tests,
rustfmt and strict Clippy; 13 native probes plus observer unit tests pass;
general corpus 1,900 samples (project groups removed), application max 6.9 ms,
PTY max 26.3 / 26.0 ms trace on/off, no sample above 50 ms. Navigation on 12, 2
and 1 CPUs: PTY max 24.1 / 23.0 / 25.1 ms, goto 20.5–25.1 ms. Two startup
resize frames on one pinned CPU (127/148 ms, pre-existing) were investigated:
terminal flush while the open worker indexes with threads Windows cannot see are
pinned; see [unprepared long lines](performance/UNPREPARED_LONG_LINES.md).


## Checkpoint 12/11/2026 — review follow-up, keymap v1 and editing (dev, local)

Commits 7fa645a…1b533dc on `dev` (pushed with 455e484; GitHub Actions stay disabled):

| Commit | Change |
| --- | --- |
| 7fa645a | MCP returns protocol errors instead of panicking on malformed editor replies; go-to-line error translated |
| 6942b8b | Long lines without a complete display index are indexed on a worker; caret-anchored window and `Col …` meanwhile ([report](performance/UNPREPARED_LONG_LINES.md)) |
| 6693d04 | Find/Replace: Alt+C case-insensitive, Alt+W whole word (simple one-to-one folding) |
| e2ce2bb | System clipboard (Windows API; pbcopy/wl-copy/xclip/xsel unverified), OSC 52 opt-in, internal fallback |
| 8047451 | Core: typing Undo groups, word motion/deletion, block indent, duplicate, move lines, whole-line cut/copy |
| 4519a98 | Keymap v1 (`commands.rs`) as the single source; terminal-safe primary keys; F2 shows keys and unavailable reasons |
| 5dcbd4e | Parameterized status messages localized when set; catalog coverage test |
| 87d2641 | Multi-line lexer state (block comments, triple strings, templates, markup comments) |
| 1b533dc | app.rs split into command_app/modal_app/mouse_app |

Verification on HEAD 1b533dc (Windows x64 MSVC release, ConPTY): 176 Rust tests,
rustfmt and strict Clippy pass; 13 native probes plus the observer unit tests
pass; general interaction corpus 2,050 samples, application max 5.2 ms, PTY max
28.1 / 25.7 ms (trace on/off), no sample above 50 ms, zero dropped rows. Navigation
including unprepared fixtures on 12, 2 and 1 CPUs: no sample above 50 ms (max
41.4 ms). Evidence: `docs/diagnostics/unprepared-lines/`.

Open: Linux/macOS/SSH runtime, the historical cold End 916.326 ms sample (a
plausible mechanism is documented, not proven), synchronous goto/Up-Down on
unindexed multi-MiB lines (34–41 ms), Notepad++ M7 line/case/comment operations.


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

## Current mapping — adjacent function keys, 2026-11-11

By the user's latest request: F6 previous tab, F7 next tab, F8 Review and
F10 Settings. Markdown contents moves to Alt+M to avoid a collision. Hints,
VI/EN F1, palette descriptions, CLI help and native probes use the new map.
The keyboard audit below records the preceding checkpoint and its evidence.

147 Rust tests, strict Clippy, fmt, release build, debug `cargo run -- --help`
and 13 native probes passed with the new mapping. General trace-on: 2050
input-to-flush samples, max 4.038 ms, no >50 ms samples/warnings or dropped
rows. [Current evidence](diagnostics/function-keys/summary.json) keeps the
new runs separate from the preceding keyboard audit.

The containing Git commit closes this checkpoint. Evidence metadata preserves
the pre-commit measurement state and hashes; the final commit and push are
recorded on the linked Notion project after remote verification.

## Previous work — unified search and keyboard-only TUI, 2026-11-11

Find/Replace is one panel and one toolbar/palette action. Both fields and all
action buttons are keyboard reachable, including at 30x9. Added portable Alt+A
replace-all, Alt-arrow tab switching, F12 tab manager, Ctrl+L goto and Alt+S
Save As; existing tab bindings remain. VI/EN F1 help now scrolls completely.
Tab keys preserve dirty confirmation defaults and readonly/Agent restrictions.

146 Rust tests, fmt, strict Clippy, release build and 12 sequential native
probes passed. The dedicated audit uses mouse=false. General trace-on has
2050 input-to-flush samples, max 4.644 ms, zero dropped; PTY trace on/off maxima
17.018/27.901 ms. See [audit/evidence](KEYBOARD_AUDIT.md) and
[shortcut reference](KEYBOARD_SHORTCUTS.md). These measurements were captured
in the working tree on dev 3cca7bf; native validation scope is Windows/ConPTY.

## Previous work — resumed, search observation, 2026-11-11

The user requested reading the project on Notion and continuing development.
Notion still referenced fe46835; local dev had advanced to 3cca7bf. All 166
artifacts in the ASCII checkpoint manifest matched their stored checksums.

The search harness now measures optional PTY wait/receive/decode/display stages
and records the first observed search states. Reproductions show that the old
replace-all predicate skipped visible Searching feedback and waited for final
No matches instead. The corrected predicate accepts Searching while completion
remains separately timed. A legacy mode and sequential eight-condition matrix
preserve the old failure mode for comparison on the same release binary.

140 Rust tests, fmt, strict native Clippy, release workspace build and three
Python reader tests passed. Native smoke verified idle behavior and terminal
cleanup. See the [investigation](performance/SEARCH_OBSERVER_INVESTIGATION.md)
for the comparison, raw evidence and remaining uncertainty. Changes are in the
working tree on 3cca7bf; the previous checkpoint below remains historical.

## Previous checkpoint — paused by user, ASCII layout performance

Development is paused at the user's request. Code, harness and evidence are
checkpointed together; see [pause/process report](performance/ASCII_LAYOUT_PAUSE.md).
The dev2 review was rejected and the branch was deleted by explicit user request;
the current work started independently from dev fe46835.

Long ASCII lines now use the existing layout cache. Printable-ASCII edits before
the first tab preserve exact cell checkpoints; other edits retain conservative
fallback. New full ASCII layout copies are bypassed above 16 MiB. Cache RAM
increases for long ASCII lines; this is documented, not a memory reduction claim.

140 Rust tests, strict native Clippy and release build pass. All scheduled final
local probes finished before pause and exited 0. Final ASCII corpus: 364 internal
samples, max 4.346 ms; warm End p95 ASCII/tabbed lines 2.131/2.169 ms versus
78.776/112.062 ms before. No final ASCII response >50 ms; previous failures and
intermediate p95 misses remain in raw evidence. Search PTY 55.417 ms and historical
cold End 916.326 ms remain open investigations. No Ubuntu/SSH runtime validation,
GitHub Actions or package publication. No new development runs after pause.

## Previous checkpoint — 50 ms investigations and menu/tab separation

Every interactive response >50 ms now requires investigation. Local Windows/
ConPTY is the current validation scope; Ubuntu/SSH is excluded by user request.
Display-cache prefix reuse removes repeated long-suffix moves for eligible ASCII
edits; explicit replacement results refresh without typing debounce. Main menu
and tab-management controls have a dedicated separator and disjoint click regions.

136 Rust tests, fmt, strict native Clippy and release build pass. The final
paired trace-on/off corpus has 2050 internal samples (2016 warm/28 groups),
all-phase max 10.558 ms, zero trace loss and no measured interaction >50 ms.
Unicode 10 MiB delete/history p95 are 3.238/3.293 ms, versus 20.03/20.07 ms before.
An intermediate cache-only binary also passed 9250 internal samples, 106 fresh
navigation sessions and ten Agent-contention sessions (280 active samples).
Binary-specific results remain separate in [report and evidence](performance/LATENCY50_IMPROVEMENTS.md).

Search 71.135/73.491 ms observations were investigated: old harness waited for
result completion, while traced input-to-flush stayed <=5.386 ms. Raw failures
remain; final feedback/completion are distinct and unnecessary refresh debounce
was removed. Completion itself is not certified <=50 ms. Agent harness still
used a composed old editor executable name; it now runs guinea and has fresh
native regression evidence. Earlier CLI migration Agent claims are corrected;
their raw historical reports are preserved. Cold End 916.326 ms remains unexplained.
Native final search/tab/mouse/Agent/recovery probes and terminal matrix pass.
GitHub Actions remains inactive; no package publication.

## Previous checkpoint — official executable guinea, 2026-11-10

Product Guinea Note; canonical executable guinea/guinea.exe. Root Cargo package
guineanote, config/preferences/recovery paths, GUINEANOTE_* and Agent/MCP binaries
and IPC contracts remain unchanged. No legacy editor alias or package publication.
See [migration policy](CLI_MIGRATION.md) and
[native evidence](performance/CLI_MIGRATION_VALIDATION.md).

134 Rust tests, strict native and Linux/macOS cross-Clippy, fmt and release build
pass. Native Windows smoke verifies help/version, open/save Unicode/BOM/mixed EOL,
project, Settings, Agent and recovery on guinea.exe; terminal matrix 6/6 passes.
The recovery harness's old process-name filter failed once; exact executable
matching and repeat passed, with original failure retained. 2016 warm samples in
28 groups meet goals for this run, all-phase max 22.349 ms, zero dropped rows;
trace on/off use the same binary. No cold-tail/global SLA or native Unix/SSH
certification. Cold End 916.326 ms remains open; Actions remains inactive.

## Previous TUI checkpoint — Notion alignment, 2026-11-10

Project file/content picker, command suggestions, grapheme-aware goto with
out-of-range rejection, focus mode, compact editor header, ANSI16 fallback,
Windows newline handling and grouped input-to-flush tracing are implemented.
See [terminal workflow](TERMINAL_WORKFLOW.md) and [Notion decisions](NOTION_ALIGNMENT.md).
The executable rename to guinea is tracked separately in
[CLI migration](CLI_MIGRATION.md); package/paths/protocol identities stay compatible.

134 Rust tests, fmt, native and Linux/macOS cross-Clippy, release workspace build
and dependency audit pass. Native Windows functional probes cover tabs,
project/search, mouse/dialogs, Unicode/BOM/mixed EOL, Agent/MCP and recovery.
Terminal matrix passes 6/6 configurations. The final grouped trace corpus has
2016 warm samples across 28 operation/corpus groups, all meeting the adopted
goals for this run; all-phase max is 20.956 ms and zero trace rows dropped.
Cold category counts are insufficient for tail acceptance. See
[baseline and raw evidence](performance/LATENCY_BASELINE.md), including retained
harness failures, independent Windows sharing reproduction and trace-off repeat.

Cold End 916.326 ms remains unexplained. Full unified action/disabled-reason
dispatch, OSC52, true rapid-resize stress, native Linux/macOS/ARM/SSH/tmux,
power-loss and release/install matrix remain open. GitHub Actions stays inactive.

## Previous checkpoint — architectural risk mitigation, 2026-11-10

Implemented history-free/opaque snapshots, streamed strict I/O, bounded history,
view/cache/worker admission, file identity/hardlink/metadata controls, private
versioned draft recovery, recovered-file Agent policy, correct syntax clipping
and NO_COLOR. See [risk report](RISK_MITIGATION.md), [architecture](ARCHITECTURE.md)
and [file safety](FILE_SAFETY.md) for exact scope and remaining limits.

127 Rust tests, fmt, strict native/Linux/macOS cross-Clippy, debug/release workspace
builds and the refreshed dependency audit pass. Native TUI/Agent/MCP probes pass;
387 latest feedback samples reach <=10 MiB, max 70.618 ms. Cold End 916.326 ms
from the historical checkpoint remains unexplained. A separate ~500 ms exit
probe artifact was diagnosed with tracing as an unanswered terminal CPR and the
harness now replies correctly; raw failures are retained. Native Unix/ARM,
power-loss, real disk-full, filesystem CAS and a bounded-memory large-file
backend remain unverified/unimplemented as described in the risk report.

This development checkpoint is recorded on dev; its enclosing Git commit contains
the implementation, eight research reports and validation evidence. The user
authorized commit/push and synchronization of this status to Notion. No CI,
package publication or main-branch change is introduced.
Older sections are historical and their no-recovery/unbounded-history notes
have been superseded by this checkpoint.

## Previous checkpoint — dialogs, mouse and find/replace

Implemented a shared dialog design, explicit default-cancel decisions, clickable
close/submit controls, native mouse drag and wheel handling, file/tab double-click,
middle-click tab closing, separated tab controls and right-aligned file metadata.
The navy background is slightly darker; green, honey amber and warm white remain.

Find/replace now has two editable fields, counted/paged contextual results,
mouse actions, replacement preview, replace-one/all and one-step Undo. Work runs
on bounded background workers; stale revisions and read-only/AI-owned buffers
cannot receive prepared changes. Save remains explicit. VI/EN and F2 fallbacks
are available. See [SEARCH_REPLACE.md](SEARCH_REPLACE.md).

108 Rust tests, fmt, strict native/Linux/macOS cross-Clippy and debug/release
workspace builds pass. Native functional tests cover Unicode/BOM/CRLF saves,
Undo, readonly protection, mouse, dialogs, tabs, review and Agent cleanup.
278 general/tab/Agent/dialog/search responses are <100 ms (max 64.349 ms).
The first navigation run includes one unexplained 916.326 ms outlier; four
repeat runs (128 responses) max at 14.848 ms. The strict global budget remains
unproven; raw failures and repeats are retained. Background completion is
reported separately in [PERFORMANCE.md](PERFORMANCE.md). Native Linux/macOS
execution remains unverified. No GitHub CI was added.
After the final help/translation copy update, search (13 samples, max 67.142 ms)
and native Unicode smoke passed again on the rebuilt binary.

## Previous checkpoint — Agent CLI/MCP and AI read-only tabs

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
