# Release plan (Windows and Linux)

Repository copy of the Notion page "Kế hoạch phát hành: npx (npm), winget,
Homebrew (12/11/2026)", updated 13/11/2026 with the owner's decision: **macOS
is not released and not verified**; only Windows x64 and Linux x64 ship.
**Confirmed channels (owner, 13/11/2026), for a new project:** GitHub Releases
and `cargo install --git` first (done with v0.1.1), then winget and npm/npx.
Homebrew is deferred until macOS ships or users ask; Scoop and AUR are not
planned.

## Channel status

| Channel | Status |
| --- | --- |
| GitHub Releases | **v0.1.1 published**, marked **Latest** (full release since 13/11; published first as a pre-release), tag on `6b698d9`: [v0.1.1](https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.1); anonymous download re-checked against `SHA256SUMS`. v0.1.0 (pre-release, `8120df7`) remains |
| `cargo install --git` | **Available from v0.1.1**: `cargo install --git https://github.com/cmm-cmm/GuineaNote --tag v0.1.1 --locked guineanote` (plus `guinea-agent`); verified from the tag in a clean `rust:1.98-bookworm` container and from GitHub on Windows. Git installs use the user's toolchain defaults (dynamic VC runtime on Windows, glibc on Linux) |
| winget `cmm-cmm.GuineaNote` | **Next.** Generator `scripts/release/winget.py`; the 0.1.0 manifest validated but was never submitted |
| npm / npx `guinea-note` | **Next.** Unscoped platform packages `guinea-note-win32-x64` / `guinea-note-linux-x64`; staging and tarball tests ready (`scripts/release/npm.py`); needs the owner's `npm login` |
| Homebrew tap | Deferred (macOS is not released; few Homebrew users on Linux) |

Evidence: `docs/diagnostics/release-0.1.1/` and `release-0.1.1-gate/`
(v0.1.1), `release-0.1.0/` and `release-0.1.0-gate/` (v0.1.0).

## Channel choice for a new project (13/11/2026)

Few channels, each maintained well: every channel adds manual work to every
release (there is no CI). Channels that need popularity or long reviews wait.

| Channel | Decision | Why |
| --- | --- | --- |
| GitHub Releases | **Now** | Source of every other channel; SHA-256 checked |
| `cargo install --git` | **Now** | No publishing; only README instructions |
| winget | **Next** | Built into Windows 10/11; no popularity requirement (bot validation plus a moderator, one version per PR) |
| npm / npx | **Next** | Reaches developers with Node; three packages per release, owner's 2FA login |
| Homebrew tap | Later | Linux/WSL only without macOS; an extra repository to update per release |
| .deb / .rpm assets | Later | When Linux users ask |
| crates.io + cargo-binstall | Later | Internal crates must be renamed (`guinea-core` is taken) |
| homebrew-core, Scoop Main | Later | Need popularity (about 75 / 500 stars) |
| Chocolatey, Nix, distro repositories | Later | Manual moderation or a package maintainer |
| Scoop (own bucket), AUR | Not planned | Removed by the owner |
| Snap, Flatpak, AppImage | Not planned | Desktop GUI formats; strict snaps cannot edit files outside home |

### Next steps

1. **winget:** `scripts/release/winget.py --dist target/dist/v0.1.1`, then
   `winget validate`, a Windows Sandbox test if available, and a PR from the
   owner's fork of `microsoft/winget-pkgs` (`gh`); review takes days.
2. **npm / npx:** `scripts/release/npm.py --tag v0.1.1`, `npx` tested from the
   tarballs on both platforms, the owner runs `npm login`, platform packages
   are published before the launcher, then `npx guinea-note@0.1.1 --version` is
   checked from the registry.
3. README install commands, CHANGELOG, IMPLEMENTATION_STATUS and Notion.

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
  `guinea-note-win32-x64`, `guinea-note-linux-x64`, unscoped since 13/11), winget
  `cmm-cmm.GuineaNote`; the command stays `guinea`.
- **D5 — First version:** `v0.1.0`, published as a GitHub **pre-release**
  labelled preview.
- **D6 — Signing:** SHA-256 only for v0.1 (`SHA256SUMS`,
  `release-manifest.json`); SmartScreen may warn on browser downloads.

Channel order: GitHub Release first. The winget manifest and npm packages are
prepared from the published archives (`packaging/`) but submitted only on a
separate decision (npm needs the owner's login).

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
