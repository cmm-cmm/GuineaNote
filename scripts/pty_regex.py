"""Native regex Find/Replace: Alt+R switches the panel to regular expressions,
an invalid pattern is reported instead of scanned, and Replace all expands
capture groups while keeping BOM/CRLF bytes. Only owned temporary files and
preferences change. Run after cargo build --release.
"""
import json
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")
CTRL_F, CTRL_S, CTRL_Q, CTRL_U = "\x06", "\x13", "\x11", "\x15"
ALT_R, ALT_A = "\x1br", "\x1ba"


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-regex-") as directory:
        root = Path(directory)
        source = root / "pairs.txt"
        original = "x=1\r\nViệt=22\r\nz=y\r\n"
        source.write_bytes(b"\xef\xbb\xbf" + original.encode())
        ui = Session([str(BINARY), str(source)], preferences_path=root / "preferences.toml")
        try:
            ui.resize(42, 148)
            ui.wait("Opened")
            ui.send(CTRL_F)
            ui.wait("FIND / REPLACE")
            ui.send(r"(\w+)=(\d+)")
            ui.wait("No matches")
            ui.send(ALT_R)
            ui.wait("[x] Regex")
            ui.wait("Match 1 / 2")
            ui.send("\t$2:$1")
            ui.wait("$2:$1│")
            ui.send(ALT_A)
            ui.wait("No matches")
            ui.send("\t" * 7)  # back to the query field
            ui.send(CTRL_F + CTRL_U + "(open")
            ui.wait("Regex: ")
            ui.send("\x1b")
            ui.wait_until(lambda lines: "FIND / REPLACE" not in "\n".join(lines))
            ui.send(CTRL_S)
            ui.wait("● Saved")
            expected = b"\xef\xbb\xbf" + "1:x\r\n22:Việt\r\nz=y\r\n".encode()
            assert source.read_bytes() == expected, source.read_bytes()
            ui.send(CTRL_Q)
            ui.wait("TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            ui.close()
            ui = None
        finally:
            if ui:
                ui.close()
    print(json.dumps({"regex": {"toggle": True, "invalid_reported": True, "groups": True,
                                "exact_bytes": True}}))


if __name__ == "__main__":
    main()
