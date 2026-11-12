# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Guinea Note is a native Rust terminal text editor (Ratatui + Crossterm + Ropey). The canonical executable is `guinea`; the Cargo package is `guineanote`, and crate IDs, config/preferences paths, `GUINEANOTE_*` env vars and Agent/MCP IPC names intentionally keep the old naming (see `docs/CLI_MIGRATION.md`). `AGENTS.md` holds the repository guidelines; `docs/ARCHITECTURE.md` describes implemented behavior (research docs describe older baselines).

## Commands

Rust 1.98+ (edition 2024); Windows needs MSVC build tools. Run from the repo root. There is no CI — these checks are run locally and are the required gate:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release      # editor + guineanote-agent + guineanote-mcp
```

- Run: `cargo run -- file.txt` (needs an interactive terminal; folder arguments are rejected), `cargo run -- --language vi file.txt`
- Run with Agent access: `cargo build --workspace` then `cargo run -- --agent --agent-write file.txt` (Agent workspace = the first file's folder, else the cwd)
- Single crate / single test: `cargo test -p guinea-core <test_name_substring>`
- Integration tests: `cargo test --test cli`, `cargo test --test file_workflow`, `cargo test -p guinea-service --test contract`
- Benchmarks: `cargo bench --bench editor`; core profiling: `cargo run --release -p guinea-core --example interaction_profile`
- Tracing: set `GUINEANOTE_TRACE` to a new CSV path to record per-stage timings.

### PTY / terminal probes (Python, in `scripts/`)

Dev-only; install deps into `target/pty-tools` (the scripts add it to `sys.path`):

```sh
# Windows
python -m pip install --target target/pty-tools pywinpty==3.1.2 psutil==7.2.2 pyte==0.8.2
# Unix
python -m pip install --target target/pty-tools psutil==7.2.2 pyte==0.8.2
```

After a release build: `python -X utf8 scripts/pty_smoke.py`. Panic cleanup: `cargo build -p guinea-tui --example panic_cleanup` then `python -X utf8 scripts/pty_smoke.py --panic target/debug/examples/panic_cleanup.exe`. Other probes (`pty_keyboard.py`, `pty_search.py`, `pty_tabs.py`, `pty_dialog_mouse.py`, `pty_interaction_latency.py`, `terminal_matrix.py`, …) and their invocation order are in `docs/TEST_PLAN.md`.

Latency rules: run probes **sequentially**, never while a build or another probe is running. Every measured interaction **>50 ms must be investigated** (targets p50 ≤10 ms, p95 ≤20 ms, p99 ≤50 ms); `--record-only` keeps outliers as evidence but does not make them pass. Verification currently targets native Windows/ConPTY; Linux/macOS/SSH runtime is unverified.

## Architecture

Workspace crates (dependency direction: core ← io ← service ← tui ← root binary; syntax is standalone, used by service and tui; agent depends only on service):

- **guinea-core** — Private Ropey storage behind `TextBuffer` (UI code never touches a `ropey::Rope`). `Document` owns transactions, Undo/Redo history and dirty tokens; `ContentSnapshot` is an opaque shareable immutable view for workers; `ViewState` (selection, preferred column, display caches) is detachable and checked by document id/revision. Coordinate types `ByteOffset`, `CharIndex`, `GraphemeColumn`, `CellColumn`, `Utf16Offset` are distinct and must not be mixed; selections must end on extended-grapheme boundaries. `view.rs` holds the long-line layout cache (incremental ASCII/prefix reuse — conservative invalidation for Unicode/tabs).
- **guinea-io** — Streaming strict UTF-8 reads/writes with BOM kept separate and original LF/CRLF/mixed bytes preserved, file identity + external-modification conflict checks, platform metadata (Windows/Unix specific deps), and the private checksummed crash-recovery store. Policy details: `docs/FILE_SAFETY.md`.
- **guinea-syntax** — Filename/content language detection and a revision-tagged lexer; `LineState` carries block comments/triple strings/templates/markup comments across lines (TUI caches states per tab in `syntax_state.rs`).
- **guinea-service** — Typed Agent operations, workspace/secret path policy, bounded analysis, and the live revision/lease IPC protocol (`interprocess` + tokio).
- **guinea-tui** — The **sole owner of application state** and of all document commits. `app.rs` is the central state (with `command_app.rs` for action dispatch/availability, `modal_app.rs` for dialog keys, `mouse_app.rs` for mouse input, all `impl App` child modules); `commands.rs` is keymap v1, the single source for bindings, F2 and guide checks; `lib.rs` holds the event loop and `TerminalGuard` (restores raw mode/alt screen/mouse/cursor on exit and panic). Feature modules: dialogs, search/search_app, file_picker, tab_manager, review (Markdown/JSON/CSV/XML/TOML/YAML/… read-only formatted views), recovery, agent, i18n (Vietnamese/English), theme, trace.
- **guinea-agent** — Two binaries: `guineanote-agent` (JSON CLI) and `guineanote-mcp` (rmcp stdio server), talking to a running editor over authenticated local IPC.
- **Root `src/main.rs`** — clap CLI that calls `guinea_tui::run_with_agent`.

Key cross-cutting patterns:

- **Background work, single committer.** File I/O, full-document search/replace, long-line layout indexing, review rendering, recovery checkpoints and Agent requests run on workers; results are epoch/revision-tagged and the UI applies them only if they still match (stale results are dropped). Agent edits are applied only at the expected document revision with access/lease checks; while an Agent owns a document, its source and linked review tabs are read-only to the user.
- **Bounded resources.** At most two search/review CPU jobs hold a shared permit (`guinea-tui/src/work.rs`); large cache drops go through a bounded reclaim thread. Caches, review output (100,000 rows / 16 MiB), history (2048 transactions / 64 MiB charged payload) and file-picker listings are all capped — preserve these bounds when changing related code.
- **Event loop.** Polls at 8 ms while jobs are active, otherwise waits up to 250 ms; idle does not redraw; input bursts coalesce (≤4 ms, ≤128 events) per frame.
- **Mouse is optional.** Every mouse action has a keyboard alternative; renders publish hit rectangles for clicks. Windows uses a dedicated safe input adapter (UTF-16, repeats, AltGr, native mouse records).

## Conventions

- `unsafe_code` is forbidden workspace-wide.
- Preserve crate boundaries (TUI owns state; core APIs enforce coordinate types and grapheme-safe edits).
- Tests should cover Unicode boundaries, BOM/newline byte preservation, save conflicts and Undo/Redo when relevant; `proptest` suites exist in core. TUI render tests use Ratatui `TestBackend` at multiple sizes (down to 30×9) and both languages. `docs/diagnostics/**` evidence is checksummed byte-for-byte, and `.gitattributes` disables normalization there (and for any `tests/fixtures/**`). Don't rewrite those files.
- User-facing strings go through `i18n.rs` (VI and EN). Some docs (e.g. `NOTION_ALIGNMENT.md`, `CLI_MIGRATION.md`, `AGENT_USAGE.md`) are in Vietnamese.
- Commits: concise imperative subjects (e.g. `Reduce long-line edit latency`), focused changes.
