"""Pack release archives, SHA256SUMS and release-manifest.json (docs/RELEASE.md).

Takes the binaries built from the release tag (Windows on the host, Linux in
the container: scripts/release/linux-build.sh) and writes, under --out:

    guinea-<v>-x86_64-pc-windows-msvc.zip
    guinea-<v>-x86_64-unknown-linux-musl.tar.gz
    SHA256SUMS
    release-manifest.json

Each archive holds one folder guinea-<v>-<target>/ with the three binaries,
LICENSE, THIRD_PARTY_NOTICES.txt and README.md. Entries are sorted and dated
at the tagged commit, so the same inputs give byte-identical archives.
Refuses to overwrite an existing archive (published assets are never
replaced; ship a patch version instead).

    python -X utf8 scripts/release/package.py --tag v0.1.0 \\
        --windows-dir <dir with guinea.exe> --linux-dir <dir with guinea> \\
        --notices target/release-notices/THIRD_PARTY_NOTICES.txt --out target/dist/v0.1.0
"""
import argparse
import gzip
import hashlib
import io
import json
import subprocess
import tarfile
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BINARIES = ["guinea", "guineanote-agent", "guineanote-mcp"]
WINDOWS = "x86_64-pc-windows-msvc"
LINUX = "x86_64-unknown-linux-musl"


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True,
                          text=True).stdout.strip()


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def members(binary_dir, exe, notices, tag):
    """(name, bytes, executable) for one archive; docs come from the tag."""
    items = []
    for name in BINARIES:
        path = binary_dir / (name + exe)
        items.append((name + exe, path.read_bytes(), True))
    for name in ["LICENSE", "README.md"]:
        data = subprocess.run(["git", "show", f"{tag}:{name}"], cwd=ROOT, check=True,
                              capture_output=True).stdout
        items.append((name, data, False))
    items.append(("THIRD_PARTY_NOTICES.txt", notices.read_bytes(), False))
    return sorted(items)


def pack_zip(folder, items, stamp):
    buffer = io.BytesIO()
    date = time.gmtime(stamp)[:6]
    with zipfile.ZipFile(buffer, "w") as archive:
        for name, data, executable in items:
            info = zipfile.ZipInfo(f"{folder}/{name}", date_time=date)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100755 if executable else 0o100644) << 16
            archive.writestr(info, data, compresslevel=9)
    return buffer.getvalue()


def pack_tar_gz(folder, items, stamp):
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as archive:
        directory = tarfile.TarInfo(folder)
        directory.type, directory.mode, directory.mtime = tarfile.DIRTYPE, 0o755, stamp
        archive.addfile(directory)
        for name, data, executable in items:
            info = tarfile.TarInfo(f"{folder}/{name}")
            info.size, info.mtime = len(data), stamp
            info.mode = 0o755 if executable else 0o644
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(data))
    out = io.BytesIO()
    with gzip.GzipFile(fileobj=out, mode="wb", mtime=stamp, compresslevel=9) as gz:
        gz.write(raw.getvalue())
    return out.getvalue()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--windows-dir", type=Path, required=True)
    parser.add_argument("--linux-dir", type=Path, required=True)
    parser.add_argument("--notices", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    version = args.tag.removeprefix("v")
    commit = git("rev-parse", f"{args.tag}^{{commit}}")
    stamp = int(git("log", "-1", "--format=%ct", commit))
    args.out.mkdir(parents=True, exist_ok=True)
    targets = {}
    for target, binary_dir, exe, pack, extension in [
        (WINDOWS, args.windows_dir, ".exe", pack_zip, ".zip"),
        (LINUX, args.linux_dir, "", pack_tar_gz, ".tar.gz"),
    ]:
        folder = f"guinea-{version}-{target}"
        path = args.out / (folder + extension)
        if path.exists():
            raise SystemExit(f"{path} exists; never overwrite a release asset")
        items = members(binary_dir, exe, args.notices, args.tag)
        data = pack(folder, items, stamp)
        path.write_bytes(data)
        targets[target] = {
            "archive": path.name,
            "sha256": sha256(data),
            "bytes": len(data),
            "contents": {name: {"sha256": sha256(blob), "bytes": len(blob)}
                         for name, blob, _ in items},
        }
    sums = "".join(f"{entry['sha256']}  {entry['archive']}\n" for entry in targets.values())
    (args.out / "SHA256SUMS").write_text(sums, encoding="utf-8", newline="\n")
    manifest = {
        "name": "guinea",
        "version": version,
        "tag": args.tag,
        "commit": commit,
        "source_date": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(stamp)),
        "rustc": {
            WINDOWS: subprocess.run(["rustc", "--version"], capture_output=True,
                                    text=True).stdout.strip(),
            # Written next to the binaries by scripts/release/linux-build.sh.
            LINUX: (args.linux_dir / "rustc.txt").read_text(encoding="utf-8").strip(),
        },
        "targets": targets,
    }
    (args.out / "release-manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(sums, end="")


if __name__ == "__main__":
    main()
