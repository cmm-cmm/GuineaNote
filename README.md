# Guinea Note

Current decisions: [Notion alignment](docs/NOTION_ALIGNMENT.md). Product name is
**Guinea Note**; the canonical executable is **guinea**. See the [migration policy](docs/CLI_MIGRATION.md).
Research and older measurements retain their historical names.

Keyboard-only workflow: one Find/Replace panel, F12/Ctrl+P tab manager,
F6/F7 or Alt+Left/Right tab switching, and a scrollable F1 guide.
See the [complete shortcut reference](docs/KEYBOARD_SHORTCUTS.md).

Terminal workflow: F2 searchable command suggestions and `goto`, F11 /
Shift+F11 focus mode, ANSI16 fallback. Folder features (directory tree, open
folder, project file/content search, `guinea <folder>`) were removed on
12/11/2026 to keep the editor simple; Ctrl+O still opens a file picker.
The main menu and tab-management toolbar have a dedicated separator, including
compact layouts. Current performance policy investigates every interaction >50 ms;
see [targets](docs/performance/INTERACTION_LATENCY_TARGETS.md).
See [workflow and limits](docs/TERMINAL_WORKFLOW.md) and
[grouped interaction baseline](docs/performance/LATENCY_BASELINE.md).

Native terminal text editor, built in Rust with Ratatui, Crossterm and Ropey.
Implementation follows the supplied technical specification. Work in progress;
see [implementation status](docs/IMPLEMENTATION_STATUS.md) for verified scope.

```sh
cargo build --release
cargo run -- file.txt
cargo run --release -- --language vi notes.md
cargo run -- --help
```

On Windows use `target\release\guinea.exe`; on Linux/macOS use
`target/release/guinea`. Runtime does not require Rust or Node.js.
Build with current stable Rust and the native platform toolchain (MSVC build
tools on Windows). During development checks run locally; GitHub Actions/CI
is intentionally not configured. Linux/macOS runtime verification remains
pending. No package has been published.

Open files with `guinea file.txt` (or `cargo run -- file.txt`). No arguments
show the welcome screen. A folder argument is rejected with a message before the
terminal starts; use Ctrl+O to pick a file.

The window has a tab strip, breadcrumbs, syntax colors, current-line
highlighting and a status/shortcut bar. Click tabs or toolbar actions, or place
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

The tab toolbar shows the active position and total open count, with New,
Save, Close, previous/next and All tabs buttons. Click a numbered tab to switch
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

The file context strip shows source/split/review mode, format and buffer size;
long paths keep the filename visible. During AI work it shows the current
operation in Vietnamese or English. Locked tabs have a muted close indicator,
and navigation/copy/other-tab shortcuts remain visible. Ctrl+P includes cached
file format, size, line count and AI activity. Wide status bars show selection
size (Unicode scalars) and caret progress without hiding save status. Format,
encoding and newline information is aligned to the right. A spacer separates
the tab toolbar from the tab labels at normal terminal sizes.

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
LSP, plugin runtime or Git integration. macOS/Linux clipboard tools
(pbcopy, wl-copy, xclip, xsel) are not yet runtime-verified.

Crash recovery is enabled by default. Dirty tabs are checkpointed by a separate
worker about once per second and reopen after a crash as independent unsaved
drafts. Use Save As to keep a recovered draft; the source file is never overwritten
automatically. Set `recovery = false` in TOML to disable it. Drafts are private
local plaintext with quotas; the last uncheckpointed edits can still be lost.
See [file safety](docs/FILE_SAFETY.md) for storage paths and limits.

History defaults to 2048 Undo/Redo transactions and 64 MiB charged edit payload,
retaining the nearest transaction even if oversized. Old steps are trimmed with
feedback; F9 reports their counts. This is not a process-RAM limit. The
[risk mitigation report](docs/RISK_MITIGATION.md) records all five research risks,
implemented controls, verification and remaining uncertainty.

Run the required checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

[Test plan](docs/TEST_PLAN.md) includes native PTY checks and panic cleanup.
[Performance](docs/PERFORMANCE.md) records measured workloads and limitations.
[Dependency audit](docs/DEPENDENCY_AUDIT.md) records the advisory snapshot and
license/source policy results. M4 continuation adds native NTFS/kill tests and
long-line latency measurements; Linux/macOS execution is still unverified.
[Architecture](docs/ARCHITECTURE.md) and [editor behavior](docs/EDITOR_BEHAVIOR.md)
describe the APIs and editing rules. Build and test locally on each target OS;
there are no GitHub CI workflows during development. No npm package,
Cargo registry package, public release or optional `gn` alias has been created.

MIT for original project code; dependencies retain their own licenses.
