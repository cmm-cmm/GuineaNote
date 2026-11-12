"""Native Mark All (Notepad++ Search › Mark): F2 mark-all paints every
occurrence of the caret word, mark-bookmark bookmarks those lines on a worker,
bookmark-remove-lines + Ctrl+S then save exact bytes, and mark-clear removes
the paint. Only owned temporary files/preferences change.
Run after cargo build --release.
"""
import json
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")
F2 = "\x1bOQ"
CTRL_S, CTRL_Q = "\x13", "\x11"


def run(ui, command, expect):
    ui.send(F2)
    ui.wait("Command (")
    ui.send(command + "\r")
    ui.wait(expect)


def reversed_cells(ui, row_text, word):
    """Whether every cell of `word` on the row containing `row_text` is reversed."""
    for y, line in enumerate(ui.screen.display):
        if row_text in line:
            x = line.index(word)
            row = ui.screen.buffer[y]
            return all(row[x + i].reverse for i in range(len(word)))
    raise AssertionError(f"row {row_text!r} not on screen")


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-mark-") as directory:
        root = Path(directory)
        source = root / "mark.txt"
        source.write_bytes("alpha Việt\r\nbeta\r\nsay alpha 👩‍💻\r\n".encode())
        ui = Session([str(BINARY), str(source)], preferences_path=root / "preferences.toml")
        try:
            ui.wait("beta")
            # The caret starts on "alpha": mark-all uses the caret word.
            run(ui, "mark-all", "Marked every “alpha”")
            ui.read(0.2)
            assert reversed_cells(ui, "say alpha", "alpha"), "second line mark"
            assert not reversed_cells(ui, "say alpha", "say")
            run(ui, "mark-bookmark", "Bookmarked 2 line(s) with marks")
            assert sum("◆" in line for line in ui.screen.display) == 2
            run(ui, "bookmark-remove-lines", "MODIFIED")
            ui.send(CTRL_S)
            ui.wait("● Saved")
            assert source.read_bytes() == b"beta\r\n", source.read_bytes()
            run(ui, "mark-clear", "Marks cleared")
            ui.send(CTRL_Q)
            ui.wait("TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            ui.close()
            ui = None
        finally:
            if ui:
                ui.close()
    print(json.dumps({"mark": {"paint": True, "bookmarks": 2, "exact_bytes": True}}))


if __name__ == "__main__":
    main()
