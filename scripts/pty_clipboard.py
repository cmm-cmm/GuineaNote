"""Live system-clipboard round trip (opt-in; it replaces the real clipboard).

Copies a Unicode/CRLF document with Ctrl+A, Ctrl+C and reads it back from the
operating system clipboard, then places text on the OS clipboard and pastes it
with Ctrl+V and saves. The previous clipboard text is restored afterwards; the
probe refuses to run when the clipboard holds non-text data (an image or
files) it could not restore.

Windows: clipboard API through PowerShell. Linux: xclip (run under Xvfb or a
real X session), wl-clipboard on Wayland. macOS: pbcopy/pbpaste.

    python -X utf8 scripts/pty_clipboard.py --use-system-clipboard
"""
import argparse
import base64
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from pty_smoke import ROOT, Session

DOCUMENT = "alpha Việt é 👩‍💻\r\nsecond line\r\n"
INCOMING = "OS→Guinea Việt 👩‍💻"


def _powershell(script):
    return subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", script],
        check=True, capture_output=True, text=True, encoding="utf-8",
    ).stdout


def _tool(read):
    if sys.platform == "darwin":
        return ["pbpaste"] if read else ["pbcopy"]
    if os.environ.get("WAYLAND_DISPLAY") and shutil.which("wl-copy"):
        return ["wl-paste", "--no-newline"] if read else ["wl-copy"]
    return ["xclip", "-selection", "clipboard"] + (["-o"] if read else [])


def has_non_text():
    if os.name != "nt":
        return False
    return _powershell(
        "Add-Type -AssemblyName System.Windows.Forms;"
        "$d=[System.Windows.Forms.Clipboard]::GetDataObject();"
        "if ($d -and -not $d.GetDataPresent('UnicodeText') -and $d.GetFormats().Count -gt 0) { 'yes' }"
    ).strip() == "yes"


def get_text():
    """Clipboard text exactly (base64 through PowerShell keeps CRLF/Unicode)."""
    if os.name == "nt":
        encoded = _powershell(
            "Add-Type -AssemblyName System.Windows.Forms;"
            "$t=[System.Windows.Forms.Clipboard]::GetText();"
            "[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($t))"
        ).strip()
        return base64.b64decode(encoded).decode("utf-8")
    result = subprocess.run(_tool(True), capture_output=True)
    return result.stdout.decode("utf-8") if result.returncode == 0 else ""


def set_text(text):
    if os.name == "nt":
        encoded = base64.b64encode(text.encode("utf-8")).decode()
        _powershell(
            "Add-Type -AssemblyName System.Windows.Forms;"
            f"$t=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{encoded}'));"
            "if ($t.Length -eq 0) { [System.Windows.Forms.Clipboard]::Clear() }"
            " else { [System.Windows.Forms.Clipboard]::SetText($t) }"
        )
        return
    # xclip keeps serving the selection from a background process.
    subprocess.run(_tool(False), input=text.encode("utf-8"), check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def wait_clipboard(expected, timeout=5):
    deadline = time.monotonic() + timeout
    actual = None
    while time.monotonic() < deadline:
        actual = get_text()
        if actual == expected:
            return
        time.sleep(0.1)
    raise AssertionError(f"system clipboard {actual!r} != {expected!r}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--use-system-clipboard", action="store_true",
                        help="required: the probe replaces the clipboard (restored afterwards)")
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "target" / "release" / ("guinea.exe" if os.name == "nt" else "guinea"))
    args = parser.parse_args()
    if not args.use_system_clipboard:
        parser.error("pass --use-system-clipboard to allow replacing the clipboard")
    if has_non_text():
        print("SKIP: clipboard holds non-text data that could not be restored")
        return 2
    saved = get_text()
    try:
        with tempfile.TemporaryDirectory(prefix="guineanote-clipboard-") as directory:
            path = Path(directory) / "clipboard.txt"
            path.write_bytes(DOCUMENT.encode("utf-8"))
            config = Path(directory) / "config.toml"
            config.write_text('language = "en"\nmouse = false\n', encoding="utf-8")
            ui = Session([str(args.binary), "--config", str(config), str(path)],
                         env={"GUINEANOTE_CLIPBOARD": "system"})
            try:
                ui.wait("second line")
                # Copy: the document reaches the OS clipboard byte for byte.
                ui.write("\x01")
                ui.write("\x03")
                ui.wait("Copied to clipboard")
                wait_clipboard(DOCUMENT)
                print("copy: OK", repr(DOCUMENT))
                # Paste: text placed by another program arrives via Ctrl+V.
                set_text(INCOMING)
                ui.write("\x1b[1;5F")  # Ctrl+End
                ui.write("\x16")
                ui.wait("Guinea Việt")
                ui.write("\x13")
                ui.wait_file(path, DOCUMENT + INCOMING)
                print("paste: OK", repr(INCOMING))
                ui.write("\x11")
                ui.read(1.0)
            finally:
                ui.close()
    finally:
        set_text(saved)
    if get_text() != saved:
        print("WARNING: previous clipboard text was not restored exactly")
        return 1
    print("restored previous clipboard text")
    print("PTY_CLIPBOARD_OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
