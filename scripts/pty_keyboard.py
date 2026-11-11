"""Keyboard-only TUI audit with mouse=false; owned temporary fixtures only.

Run after cargo build --workspace --release. Uses the existing PTY dev tools
and Pillow for snapshots. Every measured feedback >50 ms is retained.
"""
import hashlib
import json
import os
import tempfile
import time

from pty_smoke import ROOT, Session
from pty_responsiveness import action, RESULTS
from pty_interaction_latency import source_hash
from tui_preview import snapshot

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")


def wait_saved(ui):
    # A previous Saved message can remain visible while the current buffer is
    # dirty. Await current state before opening a Windows save destination.
    ui.wait_until(lambda rows: "● Saved" in (text := "\n".join(rows))
                  and "● MODIFIED" not in text and "Saving" not in text
                  and "Save to NEW path" not in text)


def main():
    out = ROOT / "target" / ("keyboard-" + str(time.time_ns()))
    out.mkdir()
    (ROOT / "target/keyboard-latest-path.txt").write_text(str(out), encoding="utf-8")
    completed = []
    with tempfile.TemporaryDirectory(prefix="guinea-keyboard-") as directory:
        from pathlib import Path
        root = Path(directory)
        original = b"\xef\xbb\xbf" + "Việt 👩‍💻\r\nViệt e\u0301\r\n".encode()
        alpha, beta = root / "alpha.txt", root / "beta.txt"
        alpha.write_bytes(original)
        beta.write_bytes(b"beta original\n")
        config = root / "keyboard.toml"
        config.write_text('language = "en"\nmouse = false\n', encoding="utf-8")
        ui = Session([str(BINARY), "--config", str(config), str(root), str(alpha), str(beta)], colors=True)
        try:
            ui.resize(42, 148)
            ui.wait("Tabs 2/2")
            action(ui, "tabs:alt-left", "\x1b[1;3D", "Tabs 1/2")
            action(ui, "tabs:alt-right", "\x1b[1;3C", "Tabs 2/2")
            action(ui, "tabs:f6-previous", "\x1b[17~", "Tabs 1/2")
            action(ui, "tabs:f7-next", "\x1b[18~", "Tabs 2/2")
            action(ui, "tabs:ctrl-pageup", "\x1b[5;5~", "Tabs 1/2")
            action(ui, "tabs:ctrl-pagedown", "\x1b[6;5~", "Tabs 2/2")
            action(ui, "tabs:f12-manager", "\x1b[24~", "OPEN TABS")
            ui.send("\x1b[Z\r")
            ui.wait("Tabs 1/2")
            action(ui, "tabs:ctrl-p-manager", "\x10", "OPEN TABS")
            ui.send("\t\r")
            ui.wait("Tabs 2/2")
            ui.send("\x10alpha\r")
            ui.wait("Tabs 1/2")
            completed.append("Tab manager filtering, Tab/Shift+Tab, F12, F6/F7, Alt arrows, Ctrl+PageUp/PageDown")

            action(ui, "search:unified-open", "\x06", "FIND / REPLACE")
            ui.wait("Replace with")
            ui.send("Việt")
            ui.wait("Match 1 / 2")
            ui.send("\tVietnam")
            ui.wait("Vietnam│")
            snapshot(ui, "keyboard-search-unified")
            # Replacement -> results -> Previous -> Next -> Go -> Replace -> All.
            ui.send("\t" * 6)
            ui.read(.03)
            ui.send("\r")
            ui.wait("No matches")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "FIND / REPLACE" not in "\n".join(rows))
            ui.send("\x1a")
            wait_saved(ui)
            assert alpha.read_bytes() == original
            completed.append("Unified fields and every action reached by Tab; Enter on Replace all; one Undo transaction")

            ui.send("\x06")
            ui.wait("FIND / REPLACE")
            ui.wait("Match 1 / 2")
            action(ui, "search:alt-a-feedback", "\x1ba", lambda text: "Searching" in text or "Replacing" in text or "No matches" in text)
            ui.wait("No matches")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "FIND / REPLACE" not in "\n".join(rows))
            ui.send("\x13")
            wait_saved(ui)
            expected = original.replace("Việt".encode(), b"Vietnam")
            assert alpha.read_bytes() == expected
            ui.send("\x1a")
            ui.wait("MODIFIED")
            ui.send("\x13")
            wait_saved(ui)
            assert alpha.read_bytes() == original
            completed.append("Alt+A replaces all; explicit saves preserve UTF-8 BOM/CRLF/NFD/ZWJ and Undo restores bytes")

            ui.resize(9, 30)
            ui.read(.08)
            ui.send("\x06")
            ui.wait("FIND / REPLACE")
            ui.send("\x1b[Z")  # Query -> Replace all, even in the smallest UI.
            ui.wait("Replace all")
            snapshot(ui, "keyboard-search-small")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "FIND / REPLACE" not in "\n".join(rows))
            ui.send("\x1bOP")
            ui.wait("KEYBOARD GUIDE")
            ui.send("\x1b[F")
            ui.wait("recovery")
            snapshot(ui, "keyboard-help-small-end")
            ui.send("\x1b[H")
            ui.wait("Guinea Note")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "KEYBOARD GUIDE" not in "\n".join(rows))
            ui.resize(42, 148)
            ui.read(.08)
            completed.append("30x9 focused action visibility and keyboard guide Home/End scrolling")

            action(ui, "goto:open", "\x0c", "Go to line")
            ui.send("999\x151\r")
            ui.wait_until(lambda rows: "Go to line[:column]" not in "\n".join(rows) and "Ln 1, Col 1 " in "\n".join(rows))
            action(ui, "save-as:alt-s", "\x1bs", "Save to NEW path")
            saved = root / "saved-copy.txt"
            ui.send(str(saved) + "\r")
            wait_saved(ui)
            assert saved.read_bytes() == original
            completed.append("Ctrl+L goto, Ctrl+U clear prompt, Alt+S Save As")

            ui.send("\x1bOQ")  # F2: navigate the action list, no typed command.
            ui.wait("Command (")
            ui.send("\x1b[B\x1b[A\r")
            ui.wait("PROJECT · Ctrl+E / Ctrl+G")
            ui.send("\t")
            ui.wait("Find text")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "PROJECT · Ctrl+E / Ctrl+G" not in "\n".join(rows))
            ui.send("\x0f")
            ui.wait("Open file or folder")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "Open file or folder" not in "\n".join(rows))
            for sequence in ["\x1bOS", "\x1bOS", "\x1b[23~", "\x1b[23~"]:  # F4, F11
                ui.send(sequence)
                ui.read(.02)
            ui.send("\x1b[20~")
            ui.wait("FILE DETAILS")
            ui.send("\x1b[6~\x1b[F\x1b[H\r")
            ui.wait_until(lambda rows: "FILE DETAILS" not in "\n".join(rows))
            ui.send("\x1b[19~")
            ui.wait("REVIEW OPTIONS")
            ui.send("\x1b[Z\r")
            ui.wait("Tabs 3/3")
            ui.send("\x17")
            ui.wait("Tabs 1/2")
            completed.append("F2 action-list navigation, project modes, file picker, pane/focus mode, file details and review chooser")

            ui.send("\x14draft")
            ui.wait("MODIFIED")
            ui.send("\x17")
            ui.wait("Discard unsaved changes")
            ui.send("\x1b[1;3C\x1b[24~\r")
            ui.wait_until(lambda rows: "UNSAVED CHANGES / CONFIRM" not in "\n".join(rows))
            ui.wait("Tabs 3/3")  # Alt+Right/F12 must not select Discard.
            ui.send("\x17")
            ui.wait("Discard unsaved changes")
            ui.send("\t\r")
            ui.wait("Tabs 2/2")
            completed.append("Dirty close keeps Cancel as default; tab shortcuts cannot select Discard; Tab+Enter explicitly discards")

            ui.send("\x1b[21~")
            ui.wait("SETTINGS")
            ui.send("\x1b[Z\r")
            ui.wait("SOẠN THẢO")
            ui.send("\x1bOP")
            ui.wait("HƯỚNG DẪN PHÍM TẮT")
            ui.send("\x1b[F")
            ui.wait("bản nháp")
            snapshot(ui, "keyboard-help-vi-end")
            ui.send("\x1b")
            ui.wait_until(lambda rows: "HƯỚNG DẪN PHÍM TẮT" not in "\n".join(rows))
            ui.send("\x11")
            ui.wait("TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            assert alpha.read_bytes() == original and saved.read_bytes() == original
            assert beta.read_bytes() == b"beta original\n"
            assert b"\x1b[?1000h" not in ui.output, "Mouse capture must stay disabled"
            completed.append("Vietnamese keyboard guide and settings BackTab, clean exit and unchanged source files")
        finally:
            (out / "terminal.log").write_bytes(ui.output)
            ui.close()
            report = {"completed": completed, "mouse": False, "samples": RESULTS,
                      "failures": [sample for sample in RESULTS if sample["ms"] > 50],
                      "binary_sha256": hashlib.sha256(BINARY.read_bytes()).hexdigest(), "source_sha256": source_hash()}
            (out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps({"report": str(out / "report.json"), "completed": completed, "investigations": report["failures"]}, indent=2))
        assert not report["failures"], "Keyboard feedback >50 ms requires investigation"


if __name__ == "__main__":
    main()
