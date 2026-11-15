"""Write the GitHub Release notes for a packed release (docs/RELEASE.md).

The notes are the version's CHANGELOG.md section, followed by install lines
for the archives that release-manifest.json lists. Fails if CHANGELOG.md has
no section for the version, so a release cannot go out without one:

    python -X utf8 scripts/release/notes.py --dist target/dist/v0.1.4 --out notes.md
    python -X utf8 scripts/release/notes.py --version 0.1.4    # check only
"""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REPO = "https://github.com/cmm-cmm/GuineaNote"
WINDOWS = "x86_64-pc-windows-msvc"
LINUX = "x86_64-unknown-linux-musl"


def section(version):
    text = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    heading = re.compile(rf"^## \[{re.escape(version)}\][^\n]*\n", re.MULTILINE)
    match = heading.search(text)
    if not match:
        raise SystemExit(f"CHANGELOG.md has no '## [{version}]' section")
    following = re.compile(r"^## \[", re.MULTILINE).search(text, match.end())
    body = text[match.end():following.start() if following else len(text)].strip()
    if not body:
        raise SystemExit(f"CHANGELOG.md section {version} is empty")
    return body


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", type=Path, help="folder with release-manifest.json")
    parser.add_argument("--version", help="only check that CHANGELOG.md has the section")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    if not args.dist:
        if not args.version:
            parser.error("--dist or --version is required")
        section(args.version)
        print(f"CHANGELOG.md has a section for {args.version}")
        return
    release = json.loads((args.dist / "release-manifest.json").read_text(encoding="utf-8"))
    version, tag, targets = release["version"], release["tag"], release["targets"]
    lines = [section(version), "", "### Install"]
    if WINDOWS in targets:
        lines.append(
            f"- Windows x64: download `{targets[WINDOWS]['archive']}`, compare its SHA-256 with "
            "`SHA256SUMS`, unpack it and add the folder to your PATH. The C runtime is linked "
            "statically. The binaries are not code-signed, so SmartScreen may warn about a "
            "browser download.")
        lines.append("- npm (Node.js 18+): `npx guinea-note notes.md` or `npm install -g guinea-note`.")
    if LINUX in targets:
        lines.append(f"- Linux x64 (static musl): download `{targets[LINUX]['archive']}`, check it "
                     "with `sha256sum --ignore-missing -c SHA256SUMS` and unpack it.")
    else:
        lines.append(f"- Linux x64: this release has no Linux archive; use the latest release "
                     f"that has one ({REPO}/releases).")
    lines += ["", f"Built by GitHub Actions from `{release['commit'][:7]}` ({release['rustc'][WINDOWS]}). "
              f"Full changelog: [CHANGELOG.md]({REPO}/blob/{tag}/CHANGELOG.md)."]
    text = "\n".join(lines) + "\n"
    if args.out:
        args.out.write_text(text, encoding="utf-8", newline="\n")
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
