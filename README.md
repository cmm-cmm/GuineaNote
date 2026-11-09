# GuineaNote

Native terminal text editor, built in Rust with Ratatui, Crossterm and Ropey.
Implementation follows the supplied technical specification. Work in progress;
see [implementation status](docs/IMPLEMENTATION_STATUS.md) for verified scope.

```sh
cargo build --release
cargo run -- file.txt
cargo run --release -- --language vi .
cargo run -- --help
```

On Windows use `target\release\guineanote.exe`; on Linux/macOS use
`target/release/guineanote`. Runtime does not require Rust or Node.js.
Build with current stable Rust and the native platform toolchain (MSVC build
tools on Windows). During development checks run locally; GitHub Actions/CI
is intentionally not configured. Linux/macOS runtime verification remains
pending. No package has been published.

Open a workspace with `guineanote .` or `cargo run -- .` during development.
No arguments show the current directory and welcome screen. Open a standalone
file with `guineanote file.txt`, or combine a folder and files with
`guineanote . src/main.rs README.md`.

The workspace has a lazy directory tree, tab strip, breadcrumbs, syntax colors,
current-line highlighting and a status/shortcut bar. Directories expand on
demand; opening a folder does not replace dirty tabs. Select with Up/Down,
expand/collapse with Right/Left, and open with Enter. Click tree rows, tabs or
toolbar actions, or place the editor caret, when mouse support is enabled. Use F4 to switch panes; Tab/Esc
returns from the tree to the editor. The sidebar hides automatically below
66 columns. Recommended workspace size: 100×28 or larger.

Open several files with `guineanote a.txt b.txt`. Missing paths become buffers;
files are created only on save. Use `--readonly` to prevent edits and saves,
`--config guineanote.example.toml` for explicit TOML configuration, and
`--help` / `--version` without entering the terminal UI. An interactive terminal
is required to edit or browse. Folder browsing was added at the user's request
after the original MVP scope; see [redesign notes](docs/UI_REDESIGN.md).

| Keys | Action |
| --- | --- |
| Ctrl+N / Ctrl+O / Ctrl+S | New / open / save |
| Ctrl+K / Ctrl+B | Open folder / toggle directory tree |
| F4 / F5 | Switch pane / refresh directory listing |
| Ctrl+Shift+S, or F2 → `save-as` | Save to a new path |
| Ctrl+W / Ctrl+Q | Close tab / quit (dirty buffers require confirmation) |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+A / Ctrl+C / Ctrl+X / Ctrl+V | Select all / copy / cut / paste internally |
| Ctrl+F / F3 / Ctrl+H | Find literal / next match / replace all |
| Arrows, Home/End, PageUp/PageDown | Navigate; Shift extends selection |
| Ctrl+Home / Ctrl+End | Document start / end |
| F6 / Shift+F6 | Next / previous tab (Ctrl+Tab also when available) |
| F7 | Cycle source / split / review (Markdown and textual data) |
| F8 | Switch Vietnamese / English interface |
| F1 / F2 | Help / command prompt with fallbacks for every main action |

Prompts use Enter to submit and Esc to cancel. Destructive confirmations
require Y; N/Esc cancels. Save first, then close/quit, to keep edits. Clipboard
is internal; normal terminal paste depends on the terminal. File operations
and full-document search/replace run in the background. Open/search/replace
temporarily pause edits with visible feedback. Saving allows continued editing;
completion marks the committed snapshot saved and keeps later edits dirty.
All actions also work without a mouse.
The navy/mint theme uses true color on Windows and terminals reporting true
color; `basic_colors = true` selects a 256-color fallback. `NO_COLOR` is honored.
No Nerd Font is required. Use a tall terminal for the complete F1 guide.

Use `--language vi` / `--language en`, or `language = "vi"` / `"en"` in the
configuration, for Vietnamese / English. The default is English. F8 switches
the current session; F2 commands `language-vi` and `language-en` also work.
Document contents and filenames remain unchanged. Parser/OS diagnostics may
retain their original technical details.

F7 opens a cached, background-rendered review. Markdown renders headings,
emphasis, lists/tasks, quotes, links, code fences and tables. JSON/JSONL/NDJSON
is formatted with key/value colors; XML/SVG displays element structure;
CSV/TSV displays numbered rows and header columns, including quoted/multiline
fields. CSV/TSV treats the first record as the header; Left/Right in full review
browses further columns. TOML has a formatted view. YAML/INI provide highlighted
source review. F2 `source`, `split` or `review` selects a mode directly.
In full review, arrows/PageUp/PageDown/Home/End and the mouse wheel scroll;
Esc returns to source. Source and review have independent scroll positions.
Review is read-only and refreshes after source edits. It never saves formatted
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
not a universal timing guarantee. Syntax is a small line-local lexer, without
multiline grammar state; very long horizontally-scrolled lines use viewport
context. No bounded-memory large-file support, system clipboard
integration, LSP, plugin runtime, Git integration or crash recovery.

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
