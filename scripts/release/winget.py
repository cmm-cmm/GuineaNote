"""Write the winget manifest for a release (docs/RELEASE.md, step 5).

Reads release-manifest.json of the packed release and writes the three
manifest files (portable zip) under packaging/winget/manifests/, the layout
of microsoft/winget-pkgs. Validate with `winget validate --manifest <dir>`
before opening the pull request there.

    python -X utf8 scripts/release/winget.py --dist target/dist/v0.1.0 --date 2026-11-13
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IDENTIFIER = "cmm-cmm.GuineaNote"
REPO = "https://github.com/cmm-cmm/GuineaNote"
TARGET = "x86_64-pc-windows-msvc"
SCHEMA = "1.9.0"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--date", required=True, help="release date, YYYY-MM-DD")
    args = parser.parse_args()
    release = json.loads((args.dist / "release-manifest.json").read_text(encoding="utf-8"))
    version, tag = release["version"], release["tag"]
    entry = release["targets"][TARGET]
    folder = f"guinea-{version}-{TARGET}"
    out = ROOT / "packaging" / "winget" / "manifests" / "c" / "cmm-cmm" / "GuineaNote" / version
    out.mkdir(parents=True, exist_ok=True)
    header = f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{{}}.{SCHEMA}.schema.json\n"
    files = {
        f"{IDENTIFIER}.yaml": header.format("version") + f"""
PackageIdentifier: {IDENTIFIER}
PackageVersion: {version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: {SCHEMA}
""",
        f"{IDENTIFIER}.installer.yaml": header.format("installer") + f"""
PackageIdentifier: {IDENTIFIER}
PackageVersion: {version}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
- RelativeFilePath: {folder}\\guinea.exe
  PortableCommandAlias: guinea
- RelativeFilePath: {folder}\\guineanote-agent.exe
  PortableCommandAlias: guineanote-agent
- RelativeFilePath: {folder}\\guineanote-mcp.exe
  PortableCommandAlias: guineanote-mcp
Commands:
- guinea
- guineanote-agent
- guineanote-mcp
ReleaseDate: {args.date}
Installers:
- Architecture: x64
  InstallerUrl: {REPO}/releases/download/{tag}/{entry['archive']}
  InstallerSha256: {entry['sha256'].upper()}
ManifestType: installer
ManifestVersion: {SCHEMA}
""",
        f"{IDENTIFIER}.locale.en-US.yaml": header.format("defaultLocale") + f"""
PackageIdentifier: {IDENTIFIER}
PackageVersion: {version}
PackageLocale: en-US
Publisher: cmm-cmm
PublisherUrl: https://github.com/cmm-cmm
PublisherSupportUrl: {REPO}/issues
PackageName: Guinea Note
PackageUrl: {REPO}
License: MIT
LicenseUrl: {REPO}/blob/{tag}/LICENSE
ShortDescription: Notepad++-style terminal text editor with tabs, regex Find/Replace and exact-byte saves
Description: |-
  Guinea Note is a terminal text editor written in Rust. It has tabs, one Find/Replace panel with
  regular expressions, line tools, bookmarks, session restore, read-only formatted previews and
  saves that keep UTF-8 BOM and LF/CRLF/mixed newlines exactly as they were. English and Vietnamese
  interface. Run `guinea file.txt` in Windows Terminal or any console.
Moniker: guinea
Tags:
- editor
- text-editor
- terminal
- tui
- notepad
- rust
ReleaseNotesUrl: {REPO}/releases/tag/{tag}
ManifestType: defaultLocale
ManifestVersion: {SCHEMA}
""",
    }
    for name, text in files.items():
        (out / name).write_text(text, encoding="utf-8", newline="\n")
    print(out)


if __name__ == "__main__":
    main()
