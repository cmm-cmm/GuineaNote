"""Native app crash/restart, exact draft recovery and deliberate Save As.

Only owned temporary source/preferences/recovery files are touched.
Requires the same optional PTY tools as pty_smoke.py and a release build.
"""
import json
import os
from pathlib import Path
import tempfile
import time
from pty_smoke import ROOT, Session

BINARY = ROOT / "target/release" / ("guineanote.exe" if os.name == "nt" else "guineanote")


def checkpoints(root):
    return list(root.glob("session-*/*.gnr"))


def wait_checkpoint(ui, root, payload, previous=()):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if any(payload in path.read_bytes() for path in checkpoints(root) if path not in previous):
            return
        ui.read(.02)
    raise AssertionError("No committed checkpoint containing the exact draft")


def crash(ui):
    import psutil
    editor = next(p for p in psutil.Process(ui.pid).children(recursive=True)
                  if p.name().lower().startswith("guineanote"))
    editor.kill()
    editor.wait(timeout=10)
    # The forced kill intentionally bypasses the editor's normal cleanup.
    ui.read(.15)
    ui.close()


def main():
    results = []
    with tempfile.TemporaryDirectory(prefix="guineanote-recovery-") as directory:
        root = Path(directory)
        for locale in ["en", "vi"]:
            case = root / locale
            case.mkdir()
            source = case / "Việt.txt"
            original = b"\xef\xbb\xbf" + "original e\u0301 👩‍💻\r\nsecond\n".encode()
            source.write_bytes(original)
            recovery = case / "recovery"
            preferences = case / "preferences.toml"
            env = {"GUINEANOTE_RECOVERY_DIR": str(recovery), "NO_COLOR": "1"}
            args = [str(BINARY), "--language", locale]
            ui = Session([*args, str(source)], preferences_path=preferences, env=env)
            try:
                ui.wait("original")
                ui.send("draft ")
                ui.wait("draft original")
                payload = b"draft " + original[3:]
                wait_checkpoint(ui, recovery, payload)
                assert source.read_bytes() == original
                crash(ui)
                ui = None
                changed = b"external changes after crash\r\n"
                source.write_bytes(changed)
                previous = set(checkpoints(recovery))
                # Crash a second time after the recovered draft has been handed off.
                ui = Session([*args, str(case)], preferences_path=preferences, env=env)
                ui.wait("draft original")
                ui.wait("Recovered" if locale == "en" else "Khôi phục")
                wait_checkpoint(ui, recovery, payload, previous)
                assert source.read_bytes() == changed
                crash(ui)
                ui = Session([*args, str(case)], preferences_path=preferences, env=env)
                ui.wait("draft original")
                saved = case / "saved.txt"
                ui.send("\x1bOQ")
                ui.wait("Command (" if locale == "en" else "Nhập lệnh")
                ui.send("save-as\r")
                ui.wait("Save to NEW path" if locale == "en" else "Lưu")
                ui.send(str(saved) + "\r")
                deadline = time.monotonic() + 15
                expected = b"\xef\xbb\xbf" + payload
                while not saved.exists() or saved.read_bytes() != expected:
                    if time.monotonic() > deadline:
                        raise AssertionError("Save As did not preserve exact BOM/Unicode/newlines")
                    ui.read(.02)
                assert source.read_bytes() == changed
                # Disk commit precedes delivery of the background-save receipt.
                # Quit is only accepted after the editor owns that receipt.
                ui.wait("● Saved" if locale == "en" else "● Đã lưu")
                ui.send("\x11")
                ui.wait("TERM_STATE_OK")
                ui.wait("CHILD_EXIT=0")
                ui.close()
                ui = None
                assert not checkpoints(recovery), "normal exit must retire acknowledged drafts"
                results.append({"locale": locale, "crashes": 2, "exact_bytes": True,
                                "source_untouched": True, "clean_exit": True})
            finally:
                if ui is not None:
                    (ROOT / f"target/pty-recovery-{locale}.log").write_bytes(ui.output)
                    ui.close()
    report = {"cases": results, "note": "Process kill after committed checkpoint; not power loss"}
    (ROOT / "target/recovery-validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
