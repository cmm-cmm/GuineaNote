# Repository Guidelines

## Project Structure & Module Organization

Guinea Note is a Rust terminal editor; its canonical executable is `guinea`. `src/main.rs` provides the application CLI. The `crates/` workspace separates `guinea-core` (text, coordinates, history), `guinea-io` (file safety and recovery), `guinea-tui` (application state and rendering), `guinea-syntax` (detection and highlighting), `guinea-service` (Agent contracts and IPC), and `guinea-agent` (JSON CLI and MCP binaries).

Root `tests/` contains integration tests; crate modules also contain unit tests. `benches/editor.rs` provides microbenchmarks. `scripts/` holds Python terminal probes; `docs/` holds architecture, test plans, and diagnostic evidence. Start with `README.md` and `docs/ARCHITECTURE.md`.

## Build, Test, and Development Commands

Use Rust 1.98 or newer with native build tools; Windows requires MSVC tooling. Run commands from the repository root:

- `cargo run -- .`: launch the workspace browser in an interactive terminal.
- `cargo build --workspace --release`: build the editor and Agent binaries.
- `cargo fmt --all -- --check`: verify formatting; use `cargo fmt --all` to apply it.
- `cargo clippy --workspace --all-targets -- -D warnings`: reject lint warnings.
- `cargo test --workspace`: run all Rust tests.
- `cargo bench --bench editor`: run editor microbenchmarks.

Checks run locally; GitHub CI is intentionally unconfigured.

## Coding Style & Naming Conventions

Follow Rust 2024 conventions and rustfmt defaults, including four-space indentation. Use `snake_case` for modules/functions and `UpperCamelCase` for types. Workspace lints forbid unsafe code. Preserve crate boundaries: the TUI owns application state, and core APIs enforce distinct coordinate types and grapheme-safe edits.

## Testing Guidelines

Use Rust `#[test]` tests and existing `proptest` suites. Give tests descriptive `snake_case` behavior names. No numeric coverage threshold is specified. Cover Unicode boundaries, BOM/newline preservation, save conflicts, and undo/redo when relevant.

For terminal changes, follow `docs/TEST_PLAN.md`; after a release build, run `python -X utf8 scripts/pty_smoke.py` with its documented development dependencies. Run latency probes sequentially without concurrent builds; investigate every measured interaction above 50 ms.

## Commit & Pull Request Guidelines

Follow the history's concise imperative subjects, such as `Reduce long-line edit latency`. Keep changes focused. PR descriptions should explain behavior, link relevant issues, report checks and platform limits, and include terminal screenshots for UI changes.

## Agent Instructions

When interacting with Cloudflare, use the cf CLI unless the project has a Wrangler configuration file.
