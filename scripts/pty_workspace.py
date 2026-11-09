"""Native workspace/browser smoke: only temporary files and directories."""
import json
import os
from pathlib import Path
import tempfile

from pty_smoke import ROOT, Session


def run():
    with tempfile.TemporaryDirectory(prefix="guineanote-tree-") as directory, tempfile.TemporaryDirectory(prefix="guineanote-other-") as other:
        root = Path(directory)
        (root / "src").mkdir()
        source = root / "src" / "main.rs"
        original = 'fn main() { println!("workspace"); }\n'
        source.write_text(original, encoding="utf-8", newline="")
        readme = root / "README.md"
        readme.write_text("# workspace smoke\n", encoding="utf-8", newline="")
        binary = ROOT / "target" / "release" / ("guineanote.exe" if os.name == "nt" else "guineanote")
        session = Session([str(binary), str(root)], colors=True)
        try:
            session.wait("EXPLORER")
            session.wait("README.md")
            session.wait("START HERE")
            # SGR mouse input becomes native console records under ConPTY.
            # Match the dialog title, not the always-visible toolbar label.
            session.send("\x1b[<0;20;2M\x1b[<0;20;2m")
            session.wait("OPEN / SEARCH / SAVE")
            session.write("\x1b")
            session.wait("Cancelled")
            session.write("\x1b[H\x1b[C")  # Home, Right expands src.
            session.wait("main.rs")
            session.write("\x1b[B\r")
            session.wait("Opened")
            session.wait('fn main()')
            code_row, code_column = next((row, line.index("fn main()")) for row, line in enumerate(session.screen.display) if "fn main()" in line)
            pointer = f"{code_column + 2};{code_row + 1}"
            session.send(f"\x1b[<0;{pointer}M\x1b[<0;{pointer}m")
            session.wait("Ln 1, Col 2 ")
            session.write("\x1b[H")
            session.wait("Ln 1, Col 1 ")
            session.write("// edited ")
            session.write("\x13")
            session.wait("Saved")
            session.wait_file(source, "// edited " + original)
            session.write("\x1bOS")  # F4 returns to tree.
            session.write("\x1b[D\x1b[D")  # Parent, collapse.
            session.wait_until(lambda lines: "main.rs" not in "\n".join(line[:30] for line in lines))
            session.write("\x1b[B\r")
            session.wait("workspace smoke")
            session.write("\x02")  # Ctrl+B hides explorer.
            assert "EXPLORER" not in "\n".join(session.screen.display)
            session.write("\x02\x1bOS")  # Show explorer; F4 editor.
            session.write("dirty ")
            session.write("\x0b")  # Ctrl+K opens folder prompt.
            session.wait("Open folder")
            session.write(other + "\r")
            session.wait("This folder is empty")
            session.write("\x11")
            session.wait("Discard ALL")
            session.write("n")
            assert readme.read_text(encoding="utf-8") == "# workspace smoke\n"
            session.write("\x11y")
            session.wait("TERM_STATE_OK")
            session.wait("CHILD_EXIT=0")
            print(json.dumps({"workspace_pty": "PASS", "mouse_toolbar_click": "PASS", "mouse_editor_caret": "PASS", "directory_expand_collapse": "PASS", "file_open_save": "PASS", "dirty_tab_across_folder_change": "PASS", "toggle_and_focus": "PASS"}, indent=2))
        finally:
            (ROOT / "target" / "pty-workspace.log").write_bytes(session.output)
            session.close()


if __name__ == "__main__":
    run()
