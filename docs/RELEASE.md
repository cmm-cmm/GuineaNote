# Release plan (Windows and Linux)

Repository copy of the Notion page "Kế hoạch phát hành: npx (npm), winget,
Homebrew (12/11/2026)", updated 13/11/2026 with the owner's decision: **macOS
is not released and not verified**; only Windows x64 and Linux x64 ship.
Homebrew therefore drops out of the plan (Linuxbrew alone is not worth a tap
for v0.1).

## Channel status

| Channel | v0.1.0 (13/11/2026) |
| --- | --- |
| GitHub Release | **Published** as a pre-release: [v0.1.0](https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.0), tag on `8120df7`; anonymous download re-checked against `SHA256SUMS` |
| winget `cmm-cmm.GuineaNote` | Manifest in `packaging/winget/manifests/c/cmm-cmm/GuineaNote/0.1.0`, `winget validate` OK; Windows Sandbox install and PR to `microsoft/winget-pkgs` not done |
| npm `guinea-note` | Packages staged by `scripts/release/npm.py` and tested from tarballs (`npx guinea --version` on Windows and Linux); not published (needs the owner's npm login and the `@guinea-note` scope) |

Evidence: `docs/diagnostics/release-0.1.0/` (archives' SHA-256, manifest,
archive checks, npm manifest) and `docs/diagnostics/release-0.1.0-gate/`
(full Windows and Linux suites).

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

## Owner decisions (13/11/2026)

- **D1 — Visibility:** `cmm-cmm/GuineaNote` is **public** (with Code of
  Conduct, contributing guide, security policy, support page, issue and PR
  templates).
- **D2 — Where to build:** **by hand** — Windows on the maintainer's machine,
  Linux in the Debian 12 container (`scripts/release/linux-build.sh`). No
  GitHub Actions workflow, not even a manual one.
- **D4 — Names:** npm `guinea-note` (platform packages
  `@guinea-note/win32-x64`, `@guinea-note/linux-x64`), winget
  `cmm-cmm.GuineaNote`; the command stays `guinea`.
- **D5 — First version:** `v0.1.0`, published as a GitHub **pre-release**
  labelled preview.
- **D6 — Signing:** SHA-256 only for v0.1 (`SHA256SUMS`,
  `release-manifest.json`); SmartScreen may warn on browser downloads.

Channel order: GitHub Release first. The winget manifest and npm packages are
prepared from the published archives (`packaging/`) but submitted only on a
separate decision (npm needs the owner's login and the `@guinea-note` scope).

## Runbook

1. On `dev`: version and CHANGELOG; `cargo fmt --check`, Clippy `-D warnings`,
   `cargo test --workspace`; `bash scripts/windows/verify.sh` and
   `bash scripts/linux/run.sh` sequentially with the latency gate.
2. Fast-forward `main` to `dev`, annotated tag `vX.Y.Z`.
3. Build both targets from the tag with `--locked`: Windows in a clean
   worktree of the tag (`git worktree add target/release-wt vX.Y.Z`), Linux
   with `bash scripts/release/linux-build.sh vX.Y.Z`. Generate notices
   (`scripts/release/notices.py`), pack with `scripts/release/package.py`
   (archives, `SHA256SUMS`, `release-manifest.json`), then check the unpacked
   archives: Windows `objdump -p` imports and `pty_smoke.py --binary`, Linux
   `bash scripts/release/linux-check.sh vX.Y.Z target/dist/vX.Y.Z`.
4. Push `main` and the tag; `gh release create` (pre-release while 0.x
   preview) with the archives, `SHA256SUMS` and `release-manifest.json`;
   download anonymously and re-check `SHA256SUMS`.
5. winget: update `packaging/winget/` (portable zip, `cmm-cmm.GuineaNote`),
   validate with `winget validate` and install in Windows Sandbox, then open
   the PR to `microsoft/winget-pkgs`.
6. npm: `python -X utf8 scripts/release/npm.py --tag vX.Y.Z` stages the
   platform packages from the archive binaries (same SHA-256) and the
   `guinea-note` launcher; test `npx` from the staged tarballs on both OSes,
   then `npm publish` platform packages before the launcher.
7. Record channel status here, in IMPLEMENTATION_STATUS and in Notion.

Rollback: never overwrite a published asset; publish a patch version
(`npm deprecate` + dist-tag, winget PR for the fix).
