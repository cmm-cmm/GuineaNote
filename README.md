# Guinea Note

**A Notepad++-style text editor for the terminal**, written in Rust with
[Ratatui](https://ratatui.rs), [Crossterm](https://github.com/crossterm-rs/crossterm)
and [Ropey](https://github.com/cessen/ropey). Tabs, one Find/Replace panel
with regular expressions, line tools, bookmarks, session restore, read-only
formatted previews and saves that keep your bytes exactly as they were.

> **Status: 0.1.0 preview.** Release targets are **Windows x64** and
> **Linux x64** (static musl, any distribution). macOS is not released.
> Interfaces are in English and Vietnamese (`--language vi`).

## Features

- **Tabs like Notepad++** — one-row tab bar, tab manager (F12 / Ctrl+P),
  session restore of open files, carets and the active tab.
- **Find/Replace** — one panel (Ctrl+F / Ctrl+H), match case, whole word,
  linear-time regular expressions with `$1` / `${name}` replacements, Mark
  All, bookmarks; Replace all is a single Undo step.
- **Editing tools** — word motion, auto-indent, block indent, duplicate and
  move lines, sort / remove duplicate or empty lines / trim, case changes and
  per-language line comments from the F2 action list.
- **Exact files** — strict UTF-8 with BOM and LF/CRLF/mixed newlines
  preserved, atomic saves with external-change conflict checks, checksummed
  crash-recovery drafts.
- **Responsive on large files** — long lines are indexed and large tools run
  on workers; typing during a background job is queued, never dropped; Esc
  cancels large tools. Every interaction above 50 ms is investigated.
- **Previews** — Markdown (with contents sidebar), JSON, CSV/TSV, XML, TOML,
  YAML, INI/ENV, notebooks, as a split or a linked read-only tab.
- **Keyboard first** — every mouse action has a key; key labels on screen
  are generated from the keymap. Optional mouse support.
- **Opt-in Agent access** — `guineanote-agent` (JSON CLI) and
  `guineanote-mcp` (MCP stdio server) edit live documents at an expected
  revision under a read-only lease. See the [Agent guide](docs/AGENT_USAGE.md)
  (Vietnamese).

## Installation

### Download

Each [GitHub Release](https://github.com/cmm-cmm/GuineaNote/releases) has a
`.zip` for Windows and a `.tar.gz` for Linux with `guinea`,
`guineanote-agent`, `guineanote-mcp`, `LICENSE` and
`THIRD_PARTY_NOTICES.txt`, plus `SHA256SUMS`:

```sh
# Linux x64 (static: Debian 11+, Ubuntu 22.04+, Alpine, ...)
curl -LO https://github.com/cmm-cmm/GuineaNote/releases/download/v0.1.0/guinea-0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/cmm-cmm/GuineaNote/releases/download/v0.1.0/SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS
tar -xzf guinea-0.1.0-x86_64-unknown-linux-musl.tar.gz
install -m 755 guinea-0.1.0-x86_64-unknown-linux-musl/guinea ~/.local/bin/
```

On Windows, unpack `guinea-0.1.0-x86_64-pc-windows-msvc.zip` and put
`guinea.exe` on your `PATH` (no Visual C++ redistributable needed). Check the
download with `Get-FileHash -Algorithm SHA256`. Binaries are not code-signed
yet, so SmartScreen may warn about a browser download. winget and npm
packages are planned ([release plan](docs/RELEASE.md)).

### Build from source

Rust 1.98+ (MSVC build tools on Windows):

```sh
# Windows: the C runtime is linked statically (.cargo/config.toml)
cargo build --workspace --release --locked
# Linux: static musl binaries that run on any distribution
rustup target add x86_64-unknown-linux-musl   # plus musl-tools on Debian/Ubuntu
cargo build --workspace --release --locked --target x86_64-unknown-linux-musl
```

Copy `guinea` (plus `guineanote-agent` / `guineanote-mcp` for Agent access)
onto your `PATH`. Runtime needs neither Rust nor Node.js. On Linux the system
clipboard uses `wl-clipboard`, `xclip` or `xsel` when present (otherwise an
internal clipboard; OSC 52 is opt-in).

## Quick start

```sh
guinea notes.md               # open (or create on save) a file
guinea a.txt b.txt            # several tabs
guinea                        # reopen the previous session
guinea --language vi file.txt # Vietnamese interface
guinea --readonly log.txt     # no edits or saves
guinea --help
```

Ctrl+S saves, Ctrl+Q quits, F1 shows the keyboard guide and F2 lists every
action with its key. Recommended terminal size: 100×28 or larger.

## Documentation

| Topic | Document |
| --- | --- |
| Keyboard shortcuts (keymap v1) | [docs/KEYBOARD_SHORTCUTS.md](docs/KEYBOARD_SHORTCUTS.md) |
| Editing rules | [docs/EDITOR_BEHAVIOR.md](docs/EDITOR_BEHAVIOR.md) |
| Find/Replace | [docs/SEARCH_REPLACE.md](docs/SEARCH_REPLACE.md) |
| File safety, recovery | [docs/FILE_SAFETY.md](docs/FILE_SAFETY.md) |
| Preview formats | [docs/FORMATS.md](docs/FORMATS.md) |
| Terminal workflow and limits | [docs/TERMINAL_WORKFLOW.md](docs/TERMINAL_WORKFLOW.md) |
| Architecture | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| Implementation status | [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md) |
| Test plan, performance | [docs/TEST_PLAN.md](docs/TEST_PLAN.md), [docs/PERFORMANCE.md](docs/PERFORMANCE.md) |
| Release plan | [docs/RELEASE.md](docs/RELEASE.md) |
| Changes | [CHANGELOG.md](CHANGELOG.md) |

The product is **Guinea Note** and the command is **guinea**; crate names,
config paths and `GUINEANOTE_*` variables keep the older spelling
([migration policy](docs/CLI_MIGRATION.md)). Research documents under
`docs/research/` describe older baselines.

## Reference

Open files with `guinea file.txt` (or `cargo run -- file.txt`). No arguments
show the welcome screen. A folder argument is rejected with a message before the
terminal starts; use Ctrl+O to pick a file.

The window has a one-row tab bar, the file path and context in the editor
border, syntax colors, current-line highlighting and a status/shortcut bar. Click tabs or toolbar actions, or place
the editor caret, when mouse support is enabled. Every key label on buttons,
the tab toolbar and the footer is generated from the keymap, so labels always
match the active bindings. Recommended size: 100×28 or larger.

Open several files with `guinea a.txt b.txt`. Missing paths become buffers;
files are created only on save. Use `--readonly` to prevent edits and saves,
`--config guineanote.example.toml` for explicit TOML configuration, and
`--help` / `--version` without entering the terminal UI. An interactive terminal
is required to edit. Folder browsing, added after the original MVP scope
([redesign notes](docs/UI_REDESIGN.md)), was removed again on 12/11/2026.

| Keys | Action |
| --- | --- |
| Ctrl+N / Ctrl+O / Ctrl+S | New / open / save |
| Alt+S / Ctrl+Shift+S | Save to a new path |
| Ctrl+W / Ctrl+Q | Close tab / quit (dirty buffers require confirmation) |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+A / Ctrl+C / Ctrl+X / Ctrl+V | Select all / copy / cut / paste (system clipboard) |
| Ctrl+Left / Ctrl+Right, Ctrl+Backspace / Ctrl+Delete | Move / delete by word (Alt+B/F, Alt+Backspace, Alt+D also work) |
| Tab / Shift+Tab, Ctrl+D, Alt+Up / Alt+Down | Indent/outdent lines, duplicate line or selection, move lines |
| Ctrl+F / Ctrl+H | One Find/Replace panel with both fields visible |
| F3 | Next match; Shift+F3 goes back inside the search panel |
| Ctrl+L / Ctrl+G | Go to line[:grapheme column], 1-based |
| Arrows, Home/End, PageUp/PageDown | Navigate; Shift extends selection |
| Ctrl+Home / Ctrl+End | Document start / end |
| F6 / F7 | Previous / next tab; Alt+Left / Alt+Right and Ctrl+Tab remain alternatives |
| Ctrl+P / F12 | All tabs: type to filter, arrows or Tab/Shift+Tab select, Enter switch, Ctrl+S save, Ctrl+W close |
| Ctrl+T, Ctrl+PageUp / Ctrl+PageDown | New blank tab, previous / next tab |
| F8 | Choose split view or open a linked review tab (any UTF-8 file) |
| F10 | Settings: choose Vietnamese / English, Apply or Cancel |
| F9 | File details: metadata, text statistics and last verified fingerprint |
| Alt+M | Show/hide Markdown contents; Tab focuses the visible outline |
| F1 / F2 | Scrollable keyboard guide / action list navigable with arrows and Enter |

Keymap v1, terminal conflicts and alternatives: [shortcut reference](docs/KEYBOARD_SHORTCUTS.md).
Typing coalesces into one Undo step until a pause or caret move; Enter keeps
indentation; Ctrl+C/Ctrl+X without a selection copy/cut the whole line.

Prompts use Enter or the Submit button; Esc or the top-right `x` cancels.
Confirmations show the affected file and consequences, with Keep working
selected initially. Click a decision, or use Tab/Left/Right then Enter. Y also
confirms; N/Esc cancels. Save first, then close/quit, to keep edits. Copy/cut/
paste use the system clipboard locally (`clipboard = "auto"`), the internal
clipboard over SSH, or OSC 52 with `clipboard = "osc52"`; see the example config.
Terminal paste (bracketed paste) also works. File operations
and full-document search/replace run in the background. Open/search/replace
temporarily pause edits with visible feedback. Saving allows continued editing;
completion marks the committed snapshot saved and keeps later edits dirty.
All actions also work without a mouse. Drag or Shift-click selects text;
the mouse wheel scrolls without moving the caret. Shift+wheel or a horizontal
wheel scrolls sideways. Keyboard navigation resumes following the caret.
Middle-click closes a tab with the same unsaved-change protection. Double-click
a file-picker or tab-manager row to open/switch; one click selects it.

Ctrl+F and Ctrl+H open a shared search panel with clickable input fields,
line-numbered context, a match count, Previous/Next and Go to match buttons.
Search is literal and case-sensitive by default, with Unicode grapheme
boundaries; Alt+C toggles case-insensitive and Alt+W whole-word matching.
Enter/Shift+Enter selects the next/previous match in the fields; Tab/Shift+Tab
cycles both fields, results and every button. Enter or Space activates a focused
button; Enter in results returns to the editor. Ctrl+R replaces the current match;
Alt+A replaces all (Ctrl+Shift+R remains an alternative when delivered by the
terminal). An empty replacement deletes matches. Each replacement
operation has one Undo step and is saved separately with Ctrl+S. Read-only and
AI-owned documents cannot be replaced. Results are paged in groups of 200;
navigation wraps through all matches. Scanning/replacement runs in cancellable
workers; closing the panel cancels uncommitted work. See the
[search guide](docs/SEARCH_REPLACE.md) for details and limits.

The tab bar is one row below the menu rule: numbered tabs on the left, then
the active position and total count with New, Save, Close, previous/next and
All tabs buttons on the right. Like Notepad++, tabs take priority: the button
labels shorten (down to `+ < > F12`) when that lets every tab show, and `‹` /
`›` mark tabs scrolled out of view. Click a numbered tab to switch
or its `x` to close. Blank tabs have stable numbered names; `●` marks unsaved
changes and review tabs have a distinct label. Closing an inactive tab preserves
the current tab. Closing a linked review returns to its source; closing the last
tab leaves a blank document. Closing never deletes a file from disk.
When the strip overflows, Ctrl+P lists every tab with its full path and state;
type to filter, use arrows and Enter to switch, Ctrl+S to save, Ctrl+W to close,
or Ctrl+N to create. Unsaved source changes require confirmation before discard.
Narrow windows retain compact controls and keyboard access. See the
[tab improvement plan](docs/TAB_MANAGEMENT_PLAN.md) and
[Agent CLI/MCP implementation](docs/AGENT_IMPLEMENTATION.md).
The slightly darker navy backgrounds use green, honey-amber and warm-white details
and content. The theme uses true color on Windows and terminals reporting true
color; `basic_colors = true` selects a 256-color fallback. `NO_COLOR` is honored.
No Nerd Font is required. Use a tall terminal for the complete F1 guide.

The editor (or full review) border shows the file path after its title and,
on the right, the unsaved-tab count plus source/split/review mode, format and
buffer size; long paths keep the filename visible. During AI work it shows the current
operation in Vietnamese or English. Locked tabs have a muted close indicator,
and navigation/copy/other-tab shortcuts remain visible. Ctrl+P includes cached
file format, size, line count and AI activity. Wide status bars show selection
size (Unicode scalars) and caret progress without hiding save status. Format,
encoding and newline information is aligned to the right.

Local Agent access is opt-in: `cargo build --workspace`, then
`cargo run -- --agent --agent-write .` during development. Separate native
`guineanote-agent` JSON CLI and `guineanote-mcp` stdio binaries can read live
unsaved buffers, manage tabs and plan/apply/save revision-checked changes.
While an Agent owns a document, its source and linked review tabs show **AI**
and become read-only to the user; other tabs remain editable. Completion,
failure or cancellation restores the original access state after safe cleanup.
See the [Vietnamese CLI/MCP guide](docs/AGENT_USAGE.md) for session selection,
permissions, patch examples and MCP host configuration.

Use `--language vi` / `--language en`, or `language = "vi"` / `"en"` in the
configuration, for Vietnamese / English. The default is English. F10 opens
Settings; Enter applies the selected language, while Esc cancels. Applied
preferences are saved for the next launch in `%APPDATA%/GuineaNote/preferences.toml`
on Windows, or `$XDG_CONFIG_HOME/guineanote/preferences.toml` (otherwise
`~/.config/guineanote/preferences.toml`) on Unix. An explicit config language
overrides preferences, and `--language` overrides both. F2 commands
`language-vi` and `language-en` change only the current session.
Document contents and filenames remain unchanged. Parser/OS diagnostics may
retain their original technical details.

Ctrl+O opens a file picker with an asynchronously loaded directory listing.
Type to filter, use arrows to select, Tab to complete, and Enter to open a file
or navigate a folder. Alt+Up returns to the parent; Ctrl+U clears the query.
Full paths and new filenames also work. F9 opens details for the active file,
including buffer versus disk size, encoding/newlines, UTC timestamps and the
last verified SHA-256 fingerprint. Text statistics run in the background.

F8 offers split view and a separate linked, read-only review tab. F6/F7 switch
tabs; Esc returns from review to its editable source. Markdown includes a
contents sidebar, heading hierarchy, reading/task statistics, callout cards,
aligned tables, numbered syntax-colored code blocks and link references.
The sidebar appears in a full-width reader with enough space; Alt+M toggles it,
Tab focuses it, and Enter jumps to the selected heading. JSON/JSONL/NDJSON
is formatted with key/value colors; XML/SVG displays element structure;
CSV/TSV displays numbered rows and header columns, including quoted/multiline
fields. CSV/TSV treats the first record as the header; Left/Right in full review
browses further columns. TOML, YAML (including multiple documents), and JSON5/JSONC have formatted
views. ENV/INI/Properties show sections, fields and line numbers with literal
values. Common config files infer XML/JSON5/YAML/INI and retain source review
for unfamiliar dialects. Notebook `.ipynb` renders Markdown/code cells and saved
text outputs without running them. All UTF-8 programming files also support
source review, including unknown extensions via a generic lexer. F2 `source`
or `split` selects a mode directly; `review` / `review-tab` opens the linked tab.
In full review, arrows/PageUp/PageDown/Home/End and the mouse wheel scroll;
Esc returns to source. Source and review have independent scroll positions.
Review is read-only and refreshes after source edits, saves and reloads.
Closing a review tab preserves the source and its undo history. It never saves formatted
content over the source. HTML in Markdown is literal text and links are inert.
Review output is capped at 100,000 display rows with an explicit notice; all
source data remains available in the editor. See [formats](docs/FORMATS.md).

UTF-8 editing only; BOM and original LF/CRLF/mixed bytes are preserved. Enter
uses CRLF for exclusively CRLF files and LF otherwise. Invalid encodings,
NUL/binary content, symlink files, and unrelated existing save-as destinations
are refused. External modifications cause an error: use F2 → `reload` with
confirmation, save to a new path, or cancel. Read the
[file safety policy](docs/FILE_SAFETY.md) before relying on filesystem guarantees.

Ropey keeps content in resident memory; history and search snapshots add memory.
Files above 10 MiB trigger confirmation by default. Long lines and full-document
operations can be expensive. The 100 ms interaction budget is measured on the
local 10 MiB corpus, with full I/O/parse completion reported separately; it is
not a universal timing guarantee. Syntax distinguishes functions, types,
constants, properties, attributes, escapes, operators and punctuation, with
italic comments and separate semantic colors. It is a lightweight lexer that carries
block comments, Python triple-quoted strings, JS/TS template literals and
`<!-- -->` comments across lines (state is recomputed from the edited line, within
a per-frame budget; lines whose state is not yet known render plain, never
mis-colored). Horizontally-scrolled lines use real line-origin context up to a
bounded prefix, then plain text. No bounded-memory large-file support,
LSP, plugin runtime or Git integration. On Linux, `xclip` under X11 is
verified; `wl-copy` (Wayland) and `xsel` are not yet runtime-verified.

Crash recovery is enabled by default. Dirty tabs are checkpointed by a separate
worker about once per second and reopen after a crash as independent unsaved
drafts. Use Save As to keep a recovered draft; the source file is never overwritten
automatically. Set `recovery = false` in TOML to disable it. Drafts are private
local plaintext with quotas; the last uncheckpointed edits can still be lost.
See [file safety](docs/FILE_SAFETY.md) for storage paths and limits.

Like Notepad++'s "remember current session", a normal exit records the open
file tabs, each caret (line and grapheme column) and the active tab. Running
`guinea` without file arguments reopens them; files named on the command line
open instead, missing or unreadable files are skipped with a status message, and untitled or
review tabs are not recorded (unsaved text stays with recovery drafts). Set
`session = false` in TOML to neither restore nor record it.

History defaults to 2048 Undo/Redo transactions and 64 MiB charged edit payload,
retaining the nearest transaction even if oversized. Old steps are trimmed with
feedback; F9 reports their counts. This is not a process-RAM limit. The
[risk mitigation report](docs/RISK_MITIGATION.md) records all five research risks,
implemented controls, verification and remaining uncertainty.

## Contributing

Bug reports, ideas and pull requests are welcome; read
[CONTRIBUTING.md](CONTRIBUTING.md) first. The required local checks are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

There is no CI: checks and the PTY probes in the [test plan](docs/TEST_PLAN.md)
run locally on Windows and in a Linux container. Please follow the
[Code of Conduct](CODE_OF_CONDUCT.md), report vulnerabilities privately as
described in [SECURITY.md](SECURITY.md), and see [SUPPORT.md](SUPPORT.md) for
help.

## License

[MIT](LICENSE) for the project code. Dependencies keep their own licenses;
release archives include `THIRD_PARTY_NOTICES.txt`
(`python -X utf8 scripts/release/notices.py`).
