# Contributing to Guinea Note

Thank you for helping. Guinea Note is a small project with strict rules about
data safety and responsiveness; this page explains them so a contribution can
be merged without surprises. By taking part you agree to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- **Bugs:** open an issue with the bug template. Include `guinea --version`,
  the OS, the terminal (Windows Terminal, conhost, GNOME Terminal, …), the
  exact keys and, for file problems, whether the file has a BOM or CRLF
  newlines. Never attach private files; a minimal reproduction is enough.
- **Security problems:** do not open a public issue; see [SECURITY.md](SECURITY.md).
- **Features:** open a feature request first. The editor deliberately stays
  simple (folder browsing was removed on purpose), so agree on the scope
  before writing code.
- **Platforms:** releases target Windows x64 and Linux x64. macOS is not a
  release target; macOS fixes are welcome but cannot be verified by the
  maintainer.

## Development setup

Rust 1.98+ (edition 2024); Windows needs the MSVC build tools.

```sh
cargo build --workspace
cargo run -- file.txt              # needs an interactive terminal
cargo run -- --language vi file.txt
```

`CLAUDE.md` and `AGENTS.md` summarize the architecture for people and coding
agents; `docs/ARCHITECTURE.md` describes implemented behavior.

## Required checks

There is **no CI**. Run these locally before opening a pull request and paste
the result in its description:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Changes to terminal behavior also need the PTY probes from
[docs/TEST_PLAN.md](docs/TEST_PLAN.md) (Python, dependencies installed into
`target/pty-tools`), for example:

```sh
python -X utf8 scripts/pty_smoke.py
python -X utf8 scripts/pty_keyboard.py
```

Linux can be checked from any OS with Docker: `bash scripts/linux/run.sh`
builds and runs the test suite and probes in a Debian 12 container.

### Latency rule

Run latency probes **one at a time**, never while a build or another probe is
running. Every measured interaction above **50 ms** must be investigated and
explained or fixed (targets: p50 ≤ 10 ms, p95 ≤ 20 ms, p99 ≤ 50 ms).
`--record-only` keeps an outlier as evidence; it does not make it pass.

## Design rules

- **The TUI owns all state.** Workers (file I/O, search, layout, review,
  recovery, Agent requests) return epoch/revision-tagged results; the UI
  applies them only if they still match.
- **Exact bytes.** Strict UTF-8; BOM and LF/CRLF/mixed newlines are
  preserved. Edits must end on extended-grapheme boundaries and use the
  distinct coordinate types of `guinea-core` (`ByteOffset`, `CharIndex`,
  `GraphemeColumn`, …).
- **Bounded resources.** Caches, history, review output and queues have
  limits; keep them when changing related code.
- **No `unsafe`** (forbidden workspace-wide).
- **Keyboard first.** Every mouse action needs a keyboard alternative; key
  labels come from the keymap in `crates/guinea-tui/src/commands.rs`.
- **Both languages.** User-facing strings go through
  `crates/guinea-tui/src/i18n.rs` with Vietnamese and English text.
- **Evidence is immutable.** Files under `docs/diagnostics/**` are checksummed
  byte-for-byte; add new evidence directories instead of editing old ones.

## Tests

Use descriptive `snake_case` test names. Cover Unicode boundaries (combining
marks, emoji ZWJ sequences, CJK width), BOM/newline preservation, save
conflicts and Undo/Redo where relevant. Render tests use Ratatui's
`TestBackend` at several sizes (down to 30×9) and in both languages.

## Commits and pull requests

- Concise imperative subjects, e.g. `Reduce long-line edit latency`; one
  focused change per commit.
- Base pull requests on `dev`. Describe the behavior change, link the issue,
  list the checks you ran and on which platforms, and add a terminal
  screenshot for UI changes.
- Update the relevant documentation (`README.md`, `docs/KEYBOARD_SHORTCUTS.md`,
  `docs/EDITOR_BEHAVIOR.md`, `CHANGELOG.md`) in the same pull request.

## License

Contributions are accepted under the project's [MIT License](LICENSE).
