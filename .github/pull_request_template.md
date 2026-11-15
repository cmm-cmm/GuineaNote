## What changes

<!-- Behavior before and after; link the issue (Fixes #123). -->

## Checks

<!-- CI runs fmt, Clippy and tests; paste what you ran locally, including PTY probes. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] PTY probes from `docs/TEST_PLAN.md` for terminal changes (which ones, on which OS)
- [ ] No measured interaction above 50 ms, or it is explained

Platforms verified: <!-- Windows / Linux (container or native), terminal -->

## Notes

- [ ] User-facing strings in Vietnamese and English (`i18n.rs`)
- [ ] Docs and `CHANGELOG.md` updated
- [ ] Screenshot attached for UI changes
