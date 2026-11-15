**Windows x64 only.** Like 0.1.2, this release ships no Linux archive (the Linux build and its checks are paused), so Linux x64 stays on [v0.1.1](https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.1).

### Fixed
- Saving no longer fails with "Recovery path belongs to another user" when Guinea Note runs elevated ("Run as administrator"). Windows makes the Administrators group the owner of new files there; the owner check now also accepts the default owner of the process's own new files, while foreign owners are still refused. Found by the new CI on the elevated Windows runner; present since 0.1.0.
- Session restore matches files whose saved path uses Windows 8.3 short names; their caret position was not restored.

### Changed
- npm: the [`guinea-note`](https://www.npmjs.com/package/guinea-note) page shows this README, with the GitHub description and topics. Platform packages are scoped (`@cmm_cmm/guinea-note-win32-x64`).
- README: a Published packages table, an npm badge and "Install with npm".

### Install (Windows x64)
- Archive: download `guinea-0.1.3-x86_64-pc-windows-msvc.zip`, compare its SHA-256 with `SHA256SUMS`, unpack it and add the folder to your PATH. The C runtime is linked statically. The binaries are not code-signed, so SmartScreen may warn about a browser download.
- npm (Node.js 18+): `npx guinea-note notes.md` or `npm install -g guinea-note`.

### Checks
CI (fmt, Clippy, tests) passes on windows-latest and ubuntu-latest. Windows release gate on the tagged code: native ConPTY probes, no interaction over 50 ms. Archive binaries import only system DLLs; the PTY smoke test passes on the archived `guinea.exe`. Full changelog: [CHANGELOG.md](https://github.com/cmm-cmm/GuineaNote/blob/v0.1.3/CHANGELOG.md).
