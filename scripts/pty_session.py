"""Native session restore: tabs, grapheme caret and active tab survive a
normal exit, a bare `guinea` reopens them, files named on the command line
or `session = false` skip the restore, and missing files are reported.
Only owned temporary files/preferences change. Run after cargo build --release.
"""
import json
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")
F6 = "\x1b[17~"
CTRL_L, CTRL_S, CTRL_Q = "\x0c", "\x13", "\x11"


def quit_cleanly(ui):
    ui.send(CTRL_Q)
    ui.wait("TERM_STATE_OK")
    ui.wait("CHILD_EXIT=0")
    ui.close()


def main():
    results = []
    with tempfile.TemporaryDirectory(prefix="guineanote-session-") as directory:
        root = Path(directory)
        prefs = root / "preferences.toml"
        session_file = root / "session.toml"
        alpha, beta, gamma = root / "alpha.txt", root / "Việt beta.txt", root / "gamma.txt"
        alpha_bytes = "first line\r\nsécond 👩‍💻 line\r\n".encode()
        alpha.write_bytes(alpha_bytes)
        beta.write_bytes(b"beta content\n")
        gamma.write_bytes(b"gamma content\n")

        # 1. Open two files, place the caret in alpha (1-based grapheme column 3
        #    of line 2, right after the NFD "é"), leave alpha active and quit.
        ui = Session([str(BINARY), str(alpha), str(beta)], preferences_path=prefs)
        try:
            ui.wait("beta content")
            ui.send(F6)
            ui.wait("first line")
            ui.send(CTRL_L)
            ui.wait("Go to line")
            ui.send("2:3\r")
            ui.wait("Moved to line")
            quit_cleanly(ui)
            ui = None
        finally:
            if ui:
                ui.close()
        stored = session_file.read_text(encoding="utf-8")
        assert "alpha.txt" in stored and "Việt beta.txt" in stored, stored
        results.append({"case": "save-on-exit", "ok": True})

        # 2. Bare launch: both tabs reopen, alpha is active, the caret is where
        #    it was (typed text lands after "sé"), bytes/CRLF stay intact.
        ui = Session([str(BINARY)], preferences_path=prefs)
        try:
            ui.wait("Previous session reopened: 2 file(s)")
            ui.wait("Tabs 1/2")
            ui.send("Z")
            ui.wait("Zcond")
            ui.send(CTRL_S)
            ui.wait("● Saved")
            quit_cleanly(ui)
            ui = None
        finally:
            if ui:
                ui.close()
        expected = alpha_bytes.replace("séc".encode(), "séZc".encode())
        assert alpha.read_bytes() == expected, alpha.read_bytes()
        assert beta.read_bytes() == b"beta content\n"
        results.append({"case": "restore-tabs-caret-active", "ok": True})

        # 3. Files on the command line win over the session.
        ui = Session([str(BINARY), str(gamma)], preferences_path=prefs)
        try:
            ui.wait("gamma content")
            ui.wait("Tabs 1/1")
            assert "Previous session" not in "\n".join(ui.screen.display)
            quit_cleanly(ui)
            ui = None
        finally:
            if ui:
                ui.close()
        assert "gamma.txt" in session_file.read_text(encoding="utf-8")
        assert "alpha.txt" not in session_file.read_text(encoding="utf-8")
        results.append({"case": "named-files-skip-restore", "ok": True})

        # 4. A missing session file is skipped and reported.
        gamma.unlink()
        ui = Session([str(BINARY)], preferences_path=prefs)
        try:
            ui.wait("skipped 1 missing or unreadable file(s)")
            quit_cleanly(ui)
            ui = None
        finally:
            if ui:
                ui.close()
        results.append({"case": "missing-file-reported", "ok": True})

        # 5. session = false neither restores nor rewrites the session.
        session_file.write_text(
            f"version = 1\nactive = 0\n[[files]]\npath = {json.dumps(str(beta))}\nline = 0\ncolumn = 0\n",
            encoding="utf-8",
        )
        before = session_file.read_bytes()
        config = root / "off.toml"
        config.write_text("session = false\n", encoding="utf-8")
        ui = Session([str(BINARY), "--config", str(config)], preferences_path=prefs)
        try:
            ui.wait("Ready")
            ui.read(0.5)
            assert "beta content" not in "\n".join(ui.screen.display)
            quit_cleanly(ui)
            ui = None
        finally:
            if ui:
                ui.close()
        assert session_file.read_bytes() == before
        results.append({"case": "disabled-by-config", "ok": True})
    print(json.dumps({"session": results}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
