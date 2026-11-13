# Release plan (Windows and Linux)

Repository copy of the Notion page "Kế hoạch phát hành: npx (npm), winget,
Homebrew (12/11/2026)", updated 13/11/2026 with the owner's decision: **macOS
is not released and not verified**; only Windows x64 and Linux x64 ship.
Homebrew therefore drops out of the plan (Linuxbrew alone is not worth a tap
for v0.1). Nothing has been published: no tag, release, workflow or package.

## Artifacts

One build per target, published on GitHub Releases with `SHA256SUMS`; every
channel points at those exact files.

| Target | Archive | Channels |
| --- | --- | --- |
| `x86_64-pc-windows-msvc` (static CRT) | `guinea-<v>-x86_64-pc-windows-msvc.zip` | GitHub, winget, npm |
| `x86_64-unknown-linux-musl` (static) | `guinea-<v>-x86_64-unknown-linux-musl.tar.gz` | GitHub, npm |

Each archive: `guinea`, `guineanote-agent`, `guineanote-mcp` (`.exe` on
Windows), `LICENSE`, `THIRD_PARTY_NOTICES.txt`, `README.md`. ARM64 builds are
added only after running on ARM hardware.

## Preparation status (R0)

| # | Item | Status |
| --- | --- | --- |
| R0.1 | Queue input during background jobs (research 09 P1) | **Done 13/11** — `pty_typeahead.py` |
| R0.2 | Re-verify Windows/ConPTY on the release commit | **Done 13/11** for dev; repeat on the tagged commit |
| R0.3 | Linux static musl build | **Done 13/11** — `scripts/release/linux-musl.sh`; runs on Debian 11/12, Ubuntu 22.04, Alpine 3.20; smoke probe passes with it |
| R0.4 | Windows static CRT | **Done 13/11** — `.cargo/config.toml`; imports only system DLLs (no `VCRUNTIME140.dll`, no `api-ms-win-crt-*`) |
| R0.5 | macOS | **Dropped** (not released) |
| R0.6 | Cargo metadata and third-party notices | **Done 13/11** — `repository`, `homepage`, `readme`, `keywords`, `categories`; `scripts/release/notices.py` (193 crates, offline) |
| R0.7 | README installation section and CHANGELOG | **Done 13/11** |

## Decisions still needed from the owner

- **D1 — Visibility:** make `cmm-cmm/GuineaNote` public, or keep it private
  with a public releases-only repository (package managers cannot download
  private assets).
- **D2 — Where to build:** by hand (Windows machine + Docker for Linux), or
  a manually triggered release workflow (no push/PR CI).
- **D4 — Names:** npm `guinea-note`, winget `cmm-cmm.GuineaNote`; the command
  stays `guinea`.
- **D5 — First version:** `v0.1.0` labelled preview.
- **D6 — Signing:** SHA-256 only for v0.1 (SmartScreen may warn on browser
  downloads; winget and npm are not blocked).

## Runbook (once decided)

1. On `dev`: version and CHANGELOG; `cargo fmt --check`, Clippy `-D warnings`,
   `cargo test --workspace`; Windows probe suite and `bash scripts/linux/run.sh`
   sequentially with the latency gate.
2. Merge to `main`, tag `vX.Y.Z`.
3. Build both targets from the tag with `--locked`; generate notices; pack;
   check `--version`, linkage (`objdump -p` / `file`) and a smoke probe from
   the unpacked archive; write `SHA256SUMS` and `release-manifest.json`.
4. GitHub Release (draft → check anonymous download → publish).
5. winget: portable-zip manifest (`komac`), validate and install in Windows
   Sandbox, open the PR.
6. npm: platform packages `@guinea-note/win32-x64`, `@guinea-note/linux-x64`
   built from the archive binaries (same SHA-256), launcher package
   `guinea-note`; test `npx guinea-note --version` on both OSes.
7. Record channel status in Notion and IMPLEMENTATION_STATUS.

Rollback: never overwrite a published asset; publish a patch version
(`npm deprecate` + dist-tag, winget PR for the fix).
