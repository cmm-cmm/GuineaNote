"""Stage and pack the npm packages from the release archives (docs/RELEASE.md, step 6).

The platform packages carry the binaries extracted from the packed archives,
so npm users run byte-identical files (the SHA-256 is checked). The launcher
`guinea-note` comes from packaging/npm/guinea-note. Nothing is published:

    python -X utf8 scripts/release/npm.py --tag v0.1.0 --dist target/dist/v0.1.0
    # -> target/npm/v0.1.0/*.tgz and npm-manifest.json

Publish (owner's npm login, `@guinea-note` scope): platform packages first,
then the launcher, each with `npm publish <tgz> --access public`.
"""
import argparse
import hashlib
import io
import json
import shutil
import subprocess
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REPOSITORY = {"type": "git", "url": "git+https://github.com/cmm-cmm/GuineaNote.git"}
COMMON = {
    "license": "MIT",
    "homepage": "https://github.com/cmm-cmm/GuineaNote#readme",
    "repository": REPOSITORY,
    "bugs": {"url": "https://github.com/cmm-cmm/GuineaNote/issues"},
}
PLATFORMS = [
    ("win32-x64", "win32", "x64", "x86_64-pc-windows-msvc", ".zip", ".exe"),
    ("linux-x64", "linux", "x64", "x86_64-unknown-linux-musl", ".tar.gz", ""),
]


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def extract(archive, folder, names):
    data = {}
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as z:
            for name in names:
                data[name] = z.read(f"{folder}/{name}")
    else:
        with tarfile.open(archive, "r:gz") as t:
            for name in names:
                data[name] = t.extractfile(f"{folder}/{name}").read()
    return data


def npm(*args, cwd):
    exe = shutil.which("npm") or shutil.which("npm.cmd")
    return subprocess.run([exe, *args], cwd=cwd, check=True, capture_output=True,
                          text=True).stdout.strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    version = args.tag.removeprefix("v")
    out = args.out or ROOT / "target" / "npm" / args.tag
    if out.exists():
        raise SystemExit(f"{out} exists")
    release = json.loads((args.dist / "release-manifest.json").read_text(encoding="utf-8"))
    assert release["version"] == version
    stage = out / "stage"
    packed = {}
    optional = {}
    for suffix, os_name, cpu, target, extension, exe in PLATFORMS:
        name = f"@guinea-note/{suffix}"
        folder = f"guinea-{version}-{target}"
        entry = release["targets"][target]
        archive = args.dist / entry["archive"]
        assert sha256(archive.read_bytes()) == entry["sha256"], archive
        files = extract(archive, folder, [f"guinea{exe}", "LICENSE", "THIRD_PARTY_NOTICES.txt"])
        assert sha256(files[f"guinea{exe}"]) == entry["contents"][f"guinea{exe}"]["sha256"]
        directory = stage / suffix
        (directory / "bin").mkdir(parents=True)
        (directory / "bin" / f"guinea{exe}").write_bytes(files[f"guinea{exe}"])
        (directory / "LICENSE").write_bytes(files["LICENSE"])
        (directory / "THIRD_PARTY_NOTICES.txt").write_bytes(files["THIRD_PARTY_NOTICES.txt"])
        (directory / "README.md").write_text(
            f"# {name}\n\nThe `guinea` binary of Guinea Note {version} for {os_name}-{cpu} "
            f"(`{target}`), installed by the [`guinea-note`](https://www.npmjs.com/package/guinea-note) "
            "launcher. Same file as in the GitHub Release archive.\n", encoding="utf-8")
        manifest = {"name": name, "version": version,
                    "description": f"Guinea Note terminal text editor binary for {os_name}-{cpu}",
                    **COMMON, "os": [os_name], "cpu": [cpu],
                    "files": ["bin/", "LICENSE", "THIRD_PARTY_NOTICES.txt", "README.md"],
                    "preferUnplugged": True}
        (directory / "package.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        optional[name] = version
        packed[name] = {"binary_sha256": entry["contents"][f"guinea{exe}"]["sha256"],
                        "directory": directory}
    launcher = stage / "guinea-note"
    shutil.copytree(ROOT / "packaging" / "npm" / "guinea-note", launcher)
    shutil.copyfile(ROOT / "LICENSE", launcher / "LICENSE")
    manifest = {"name": "guinea-note", "version": version,
                "description": "Notepad++-style terminal text editor (Rust): tabs, regex Find/Replace, exact-byte saves",
                **COMMON,
                "keywords": ["editor", "text-editor", "terminal", "tui", "notepad", "cli"],
                "bin": {"guinea": "bin/guinea.js"},
                "files": ["bin/", "LICENSE", "README.md"],
                "engines": {"node": ">=18"},
                "optionalDependencies": optional}
    (launcher / "package.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    packed["guinea-note"] = {"directory": launcher}
    result = {"version": version, "tag": args.tag, "packages": {}}
    for name, info in packed.items():
        tgz = npm("pack", "--pack-destination", str(out), cwd=info["directory"]).splitlines()[-1]
        data = (out / tgz).read_bytes()
        result["packages"][name] = {"tarball": tgz, "sha256": sha256(data), "bytes": len(data),
                                    **({"binary_sha256": info["binary_sha256"]}
                                       if "binary_sha256" in info else {})}
    (out / "npm-manifest.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8",
                                           newline="\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
