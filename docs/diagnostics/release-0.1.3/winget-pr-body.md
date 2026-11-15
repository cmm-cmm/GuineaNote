## 📖 Description

New package: **Guinea Note** (`cmm-cmm.GuineaNote`) version **0.1.3**, a terminal text editor written in Rust (MIT).

- Installer: portable zip, x64, from the GitHub Release https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.3
- `InstallerSha256` matches `SHA256SUMS` of the release. The zip holds `guinea.exe`, `guineanote-agent.exe` and `guineanote-mcp.exe`, exposed through the `guinea`, `guineanote-agent` and `guineanote-mcp` command aliases.
- The executables link the C runtime statically and import only system DLLs; they are not code-signed.
- This PR first proposed 0.1.2; it now submits 0.1.3, which fixes saving from elevated sessions (the 0.1.2 files are no longer part of the change).

## ✅ Checklist

- [ ] Signed the [Contributor License Agreement](https://cla.opensource.microsoft.com) (first submission from this account; will sign when the bot asks)
- [ ] Linked to an issue (if applicable)
  - Not applicable

## 📦 Manifest Checklist

- [x] Checked that there aren't other open [pull requests](https://github.com/microsoft/winget-pkgs/pulls) for the same manifest update/change
- [x] This PR only modifies one (1) manifest
- [x] Validated manifest locally with `winget validate --manifest <path>` (winget v1.29.380: "Manifest validation succeeded")
- [ ] Tested manifest locally with `winget install --manifest <path>` (not run: local manifest files are disabled on the submitting machine; the archived `guinea.exe` passed the project's ConPTY smoke test)
- [x] Manifest conforms to the [1.28 schema](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.28.0)
