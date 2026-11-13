# Release plan (Windows and Linux)

Repository copy of the Notion page "Kế hoạch phát hành: npx (npm), winget,
Homebrew (12/11/2026)", updated 13/11/2026 with the owner's decision: **macOS
is not released and not verified**; only Windows x64 and Linux x64 ship.
Channels were re-planned per platform on 13/11/2026 (see "Channel plan by
platform"). The owner asked for npx and Homebrew in the first wave; Homebrew
ships as a Linux-only tap.

## Channel status

| Channel | v0.1.0 (13/11/2026) |
| --- | --- |
| GitHub Release | **Published** as a pre-release: [v0.1.0](https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.0), tag on `8120df7`; anonymous download re-checked against `SHA256SUMS` |
| winget `cmm-cmm.GuineaNote` | Manifest in `packaging/winget/manifests/c/cmm-cmm/GuineaNote/0.1.0`, `winget validate` OK; Windows Sandbox install and PR to `microsoft/winget-pkgs` not done |
| npm `guinea-note` | Packages staged by `scripts/release/npm.py` and tested from tarballs (`npx guinea --version` on Windows and Linux); not published (needs the owner's npm login); platform packages renamed to unscoped `guinea-note-win32-x64` / `guinea-note-linux-x64` on 13/11 |

Evidence: `docs/diagnostics/release-0.1.0/` (archives' SHA-256, manifest,
archive checks, npm manifest) and `docs/diagnostics/release-0.1.0-gate/`
(full Windows and Linux suites).

## Channel plan by platform (revised 13/11/2026)

The owner asked for channels chosen per platform (for example winget for
Windows). Every channel installs the **same files** as the GitHub Release
archives (same SHA-256); none rebuilds. Name availability was checked on
13/11/2026 (read-only lookups).

### Windows x64

| Channel | User command | Gatekeeping | Plan |
| --- | --- | --- | --- |
| **winget** `cmm-cmm.GuineaNote` | `winget install cmm-cmm.GuineaNote` | PR to `microsoft/winget-pkgs`: automated validation (schema, URL from the publisher's release, antivirus/Defender scan, silent install/uninstall), then a moderator; one package version per PR | **Phase 1.** Manifest generator ready (`scripts/release/winget.py`, portable zip). Test with `winget validate` and the repo's `Tools/SandboxTest.ps1` (needs Windows Sandbox) |
| **Scoop**, own bucket `cmm-cmm/scoop-bucket` | `scoop bucket add guinea https://github.com/cmm-cmm/scoop-bucket` then `scoop install guinea-note` | None (our repository). The official Main bucket needs a well-known tool (example: ≥500 stars, ≥150 forks) and a stable, non-pre-release version, so it is not possible yet | **Phase 1.** One JSON manifest with `checkver`/`autoupdate` from GitHub Releases |
| Chocolatey `guinea-note` | `choco install guinea-note` | Human moderation on the community repository (days to weeks), package verifier, API key | Phase 3, after winget and Scoop have users |

### Linux x64 (static musl binary, any distribution)

| Channel | User command | Gatekeeping | Plan |
| --- | --- | --- | --- |
| **AUR** `guinea-note-bin` | `yay -S guinea-note-bin` (or `makepkg -si`) | AUR account with an SSH key; prebuilt binaries must use the `-bin` suffix; no official Arch package of the same name exists | **Phase 1.** PKGBUILD that downloads the musl tar.gz with `sha256sums`; checked with `makepkg` and `namcap` in an `archlinux` container |
| **.deb / .rpm** on the GitHub Release | `sudo apt install ./guinea-note_0.1.1_amd64.deb`, `sudo dnf install ./guinea-note-0.1.1-1.x86_64.rpm` | None (release assets; no APT/YUM repository to host) | **Phase 2.** Built from the same musl binaries by `dpkg-deb` / `rpmbuild` in containers; install/uninstall checked on Debian 12, Ubuntu 22.04 and Fedora |
| Homebrew tap `cmm-cmm/homebrew-tap` | `brew install cmm-cmm/tap/guinea-note` | None (our tap); homebrew-core needs source builds and popularity | **Phase 1** (owner's request 13/11). Linux-only formula (`depends_on :linux`) installing the musl binaries; `brew install`, `brew test` and `brew audit --strict` checked in the `homebrew/brew` container. On macOS the formula refuses to install with a clear message |
| Nix (nixpkgs) | `nix profile install nixpkgs#guinea-note` | Review in `NixOS/nixpkgs`, source build, a maintainer | Phase 3 |
| Snap, Flatpak, AppImage | — | — | **Not planned.** Flatpak and AppImage target desktop GUI apps. A strictly confined snap cannot edit files outside home, and classic confinement needs manual approval |

### Both platforms

| Channel | User command | Gatekeeping | Plan |
| --- | --- | --- | --- |
| **GitHub Releases** | download, check `SHA256SUMS` | — | Done (v0.1.0) |
| **npm / npx** `guinea-note` | `npx guinea-note file.txt`, `npm i -g guinea-note` | Owner's npm account with 2FA. **Decision 13/11: unscoped platform packages** `guinea-note-win32-x64` and `guinea-note-linux-x64` (no npm organization needed). Publish by hand; trusted publishing needs CI | **Phase 1.** Staging and tarball tests ready (`scripts/release/npm.py`); rename the platform packages |
| crates.io + cargo-binstall | `cargo install guineanote --locked`, `cargo binstall guineanote` | Every workspace crate must be published, and `guinea-core` is already taken on crates.io | Phase 2: rename the internal crates (for example `guineanote-core`) or merge them, add `[package.metadata.binstall]` pointing at the release archives |

### Phase 1 order (v0.1.1)

1. **Bump to 0.1.1** (workspace version and internal dependency versions,
   `Cargo.lock`, CHANGELOG `[0.1.1]`, README download URLs); the release
   contains the Windows path-display fix.
2. **Gate:** `cargo fmt --check`, Clippy, tests, `scripts/windows/verify.sh`,
   `scripts/linux/run.sh`, run one after the other.
3. **GitHub Release v0.1.1:** tag, build from the tag on both platforms,
   `package.py`, archive checks, upload, anonymous download check. Proposed:
   publish 0.1.1 as a normal release (not a pre-release) so that
   `releases/latest` and Scoop `checkver` resolve to it.
4. **winget:** `winget.py` → `winget validate` → Sandbox test → PR from the
   owner's fork of `microsoft/winget-pkgs` (`gh`).
5. **Scoop:** create `cmm-cmm/scoop-bucket` with `bucket/guinea-note.json`;
   test `scoop install` from the bucket.
6. **Homebrew:** create `cmm-cmm/homebrew-tap` with `Formula/guinea-note.rb`
   (generated from `release-manifest.json`); check install/test/audit in the
   `homebrew/brew` container; then `brew install cmm-cmm/tap/guinea-note`.
7. **AUR:** `guinea-note-bin` PKGBUILD and `.SRCINFO`; check in an
   `archlinux` container; push to `aur@aur.archlinux.org:guinea-note-bin.git`.
8. **npm / npx:** stage with unscoped names, test `npx` from tarballs on both
   platforms, owner runs `npm login`, publish platform packages, then the
   launcher; check `npx guinea-note@0.1.1 --version` from the registry.
9. **README "Install" section** with the per-platform commands, CHANGELOG,
   IMPLEMENTATION_STATUS and Notion.

Owner actions needed: `npm login` (2FA); an AUR account with an SSH public key;
approval to create the public repositories `cmm-cmm/scoop-bucket` and
`cmm-cmm/homebrew-tap`; Windows Sandbox enabled for the winget test (optional:
otherwise the bot's validation is the install test).

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
