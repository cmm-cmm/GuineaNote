"""Native tab controls, overflow picker, saves, close safety and UI latency.
Only owned temporary files/preferences change. Run after cargo build --release.
"""
import json
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session
from pty_responsiveness import action, RESULTS
from tui_preview import snapshot

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")


def click(session, label, row_text):
    row, line = next((i, line) for i, line in enumerate(session.screen.display) if row_text in line)
    column = line.index(label) + 1
    return f"\x1b[<0;{column+1};{row+1}M\x1b[<0;{column+1};{row+1}m"


def close_label(session, name):
    row, line = next((i, line) for i, line in enumerate(session.screen.display) if name in line and " x " in line)
    column = line.index(" x ", line.index(name)) + 1
    return f"\x1b[<0;{column+1};{row+1}M\x1b[<0;{column+1};{row+1}m"


def capture(session, name):
    session.read(.12)
    snapshot(session, name)


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-tabs-") as directory:
        root = Path(directory)
        paths = [root / name for name in ["alpha.txt", "beta.txt"] + [f"file{i:02}.txt" for i in range(2, 12)]]
        originals = {path: f"original-{path.stem}" for path in paths}
        for path, content in originals.items():
            path.write_bytes(content.encode())
        session = Session([str(BINARY), str(root), *map(str, paths)], colors=True)
        try:
            session.resize(42, 148)
            session.wait("Tabs 12/12")
            session.wait("original-file11")
            capture(session, "tab-toolbar-overflow")
            action(session, "tabs:mouse-new", click(session, " + New ", "Tabs "), "Tabs 13/13")
            session.send("draft Việt 👩‍💻")
            session.wait("MODIFIED")
            capture(session, "tab-dirty-document")
            action(session, "tabs:mouse-save-prompt", click(session, " Save ", "Tabs "), "Save to NEW path")
            saved = root / "draft.txt"
            action(session, "tabs:save-feedback", str(saved)+"\r", lambda s: "Saving " in s or "● Saved" in s)
            session.wait_file(saved, "draft Việt 👩‍💻")
            session.wait("● Saved")
            action(session, "tabs:mouse-previous", click(session, " < ", "Tabs "), "Tabs 12/13")
            action(session, "tabs:mouse-next", click(session, " > ", "Tabs "), "Tabs 13/13")
            action(session, "tabs:individual-close", close_label(session, "draft.txt"), "Tabs 12/12")
            assert saved.read_bytes() == "draft Việt 👩‍💻".encode()

            action(session, "tabs:mouse-all", click(session, " All ^P ", "Tabs "), "OPEN TABS · Ctrl+P")
            capture(session, "tab-manager-all")
            action(session, "tabs:filter-alpha", "alpha", "› alpha│")
            action(session, "tabs:activate-alpha", "\r", "Tabs 1/12")
            session.send("dirty ")
            session.wait("MODIFIED")
            session.send("\x10")
            session.wait("OPEN TABS")
            session.send("beta\r")
            session.wait("Tabs 2/12")
            action(session, "tabs:inactive-dirty-close", close_label(session, "alpha.txt"), "Discard unsaved changes")
            capture(session, "tab-close-confirmation")
            action(session, "tabs:cancel-dirty-close", "n", lambda s: "UNSAVED CHANGES / CONFIRM" not in s)
            assert "Tabs 2/12" in "\n".join(session.screen.display)
            assert paths[0].read_text(encoding="utf-8") == originals[paths[0]]
            session.send(close_label(session, "alpha.txt"))
            session.wait("Discard unsaved changes")
            action(session, "tabs:confirm-inactive-close", "y", "Tabs 1/11")

            action(session, "tabs:manager-open", "\x10", "OPEN TABS · Ctrl+P")
            action(session, "tabs:filter-hidden-tab", "file10", "› file10│")
            capture(session, "tab-manager-filtered")
            action(session, "tabs:mouse-switch-hidden", click(session, " Switch ", " Switch "), "Tabs 10/11")
            session.wait("original-file10")
            session.command("review-tab")
            session.wait("Tabs 12/12")
            session.send("\x10")
            session.wait("OPEN TABS")
            session.send("[Review]")
            session.wait("1 matching")
            action(session, "tabs:close-review-from-list", "\x17", "Tabs 10/11")
            session.wait("original-file10")
            # Closing the active linked review returns to its source.
            for count in range(11, 1, -1):
                action(session, f"tabs:close-clean-{count}", "\x17", lambda s, count=count: f"/{count-1}" in next(line for line in s.splitlines() if "Tabs " in line))
            session.send("\x17")
            session.wait("Tabs 1/1")
            session.wait("New buffer")
            session.send("\x1b[21~")
            session.wait("SETTINGS")
            session.send("v\r")
            session.wait("Thẻ 1/1")
            session.send("\x10")
            session.wait("QUẢN LÝ THẺ · Ctrl+P")
            capture(session, "tab-manager-vi")
            session.send("\x1b")
            session.wait_until(lambda lines: "QUẢN LÝ THẺ" not in "\n".join(lines))
            session.send("\x11")
            session.wait("TERM_STATE_OK")
            session.wait("CHILD_EXIT=0")
            for path, content in originals.items():
                assert path.read_bytes() == content.encode()
            report = {"sample_count": len(RESULTS), "max_ms": max(r["ms"] for r in RESULTS), "samples": RESULTS, "failures": [r for r in RESULTS if r["ms"] > 50]}
            (ROOT / "target/tab-responsiveness.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
            print(json.dumps({k:v for k,v in report.items() if k != "samples"}, indent=2))
            assert not report["failures"], "Tab response budget exceeded"
            print("PASS: mouse new/save/switch/close, overflow filter, dirty cancel, review close, last blank, VI/EN, disk integrity")
        finally:
            (ROOT / "target/pty-tabs.log").write_bytes(session.output)
            session.close()


if __name__ == "__main__":
    main()
