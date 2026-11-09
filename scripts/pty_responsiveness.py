"""Native PTY responsiveness budget (release binary, local development only).

Measures fresh screen changes, including PTY transport and pyte parsing. Fails
when any measured interactive response reaches 100 ms. I/O/render completion is
reported separately from initial UI feedback. Only owned temporary files change.
"""
import json
import os
import platform
from pathlib import Path
import sys
import tempfile
import time
from pty_smoke import ROOT, Session

BINARY = ROOT / "target" / "release" / ("guineanote.exe" if os.name == "nt" else "guineanote")
RESULTS = []
COMPLETIONS = []


def action(session, name, keys, expected):
    start = time.perf_counter()
    session.send(keys)
    if callable(expected):
        deadline = time.monotonic()+30
        while not expected("\n".join(session.screen.display)):
            assert time.monotonic() < deadline
            session.read(0.005)
    else:
        session.wait(expected, timeout=30)
    elapsed = round((time.perf_counter() - start) * 1000, 3)
    RESULTS.append({"action": name, "ms": elapsed})
    return elapsed


def editor(root, size, unicode=False):
    unit = "Việt e\u0301 界 👩‍💻 " if unicode else "a"
    repeats = size // len(unit.encode())
    original = unit * repeats
    path = root / ("unicode.rs" if unicode else "ascii.rs")
    path.write_bytes(original.encode())
    graphemes = repeats * (11 if unicode else 1)
    # The Unicode fixture has 11 extended graphemes per unit, including spaces.
    session = Session([str(BINARY), str(path)])
    label = "unicode" if unicode else "ascii"
    try:
        session.wait("Opened")
        for index in range(10):
            action(session, f"{label}:type-start", "x", f"Ln 1, Col {index+2} ")
        action(session, f"{label}:end", "\x1b[F", f"Ln 1, Col {graphemes+11} ")
        for index in range(5):
            action(session, f"{label}:left-at-end", "\x1b[D", f"Ln 1, Col {graphemes+10} ")
            action(session, f"{label}:right-at-end", "\x1b[C", f"Ln 1, Col {graphemes+11} ")
        action(session, f"{label}:insert-end", "z", f"Ln 1, Col {graphemes+12} ")
        action(session, f"{label}:undo-end", "\x1a", f"Ln 1, Col {graphemes+11} ")
        action(session, f"{label}:redo-end", "\x19", f"Ln 1, Col {graphemes+12} ")
        action(session, f"{label}:backspace-end", "\x7f", f"Ln 1, Col {graphemes+11} ")
        action(session, f"{label}:home", "\x1b[H", "Ln 1, Col 1 ")
        action(session, f"{label}:settings-open", "\x1b[19~", "SETTINGS · F8")
        action(session, f"{label}:language-vi-apply", "v\r", "SOẠN THẢO")
        action(session, f"{label}:settings-open-vi", "\x1b[19~", "THIẾT LẬP · F8")
        action(session, f"{label}:language-en-apply", "e\r", "EDITOR")
        action(session, f"{label}:file-info-open", "\x1b[20~", "FILE DETAILS · F9")
        action(session, f"{label}:file-info-close", "\x1b", lambda s: "FILE DETAILS · F9" not in s)
        action(session, f"{label}:help", "\x1bOP", "KEYBOARD GUIDE")
        action(session, f"{label}:dismiss-help", "\x1b", lambda screen: "KEYBOARD GUIDE" not in screen)
        session.send("\x13")
        session.wait_file(path, "x"*10 + original)
        session.wait("● Saved")
        session.send("\x11")
        session.wait("TERM_STATE_OK")
        session.wait("CHILD_EXIT=0")
    finally:
        (ROOT / "target" / f"pty-responsiveness-{label}.log").write_bytes(session.output)
        session.close()


def reviews(root):
    fixtures = [
        ("note.md", "# GuineaNote\n\n**Review** and `code`.\n\n- [x] done\n", "[x]"),
        ("data.json", '{"name":"Việt","count":42,"active":true}', "formatted view"),
        ("data.xml", '<root><item name="Việt">42</item></root>', "structure view"),
        ("data.csv", 'name,value\n"Việt, Nam",42\n', "1 data rows"),
        ("data.tsv", 'name\tvalue\nViệt\t42\n', "1 data rows"),
        ("data.toml", 'name = "Việt"\n', "formatted view"),
        ("broken.json", '{"invalid":}', "Invalid data"),
        ("config.yaml", 'name: Việt\nlist: [1,2]\n', "YAML · document"),
        ("config.json5", "{name:'Việt', count:0x10,}\n", "JSON5/JSONC · formatted"),
        (".env.local", "export HOST=localhost\nTOKEN='${HOME}'\n", "ENV · 2 fields"),
        ("settings.ini", "[server]\nhost=localhost\nport=8080\n", "INI · 2 fields"),
        ("app.properties", "name=GuineaNote\nname=Second\n", "Properties · 2 fields"),
        ("application.config", '<app><setting name="locale" value="vi"/></app>', "XML · structure"),
        (".prettierrc", "singleQuote: true\n", "YAML · document"),
        ("example.ipynb", json.dumps({"nbformat":4,"metadata":{},"cells":[{"cell_type":"markdown","source":["# NB review"]},{"cell_type":"code","source":["print('hello')"],"outputs":[]}]}), "Notebook · 2 cells"),
        ("algorithm.futurelang", "function main() { return 42; }\n", "Generic code · source"),
        ("script", "#!/usr/bin/env python3\nprint('hello')\n", "Python · source review"),
    ]
    for filename, text, expected in fixtures:
        path = root / filename
        path.write_bytes(text.encode())
        session = Session([str(BINARY), str(path)])
        try:
            session.wait("Opened")
            action(session, f"{filename}:review-chooser", "\x1b[18~", "REVIEW OPTIONS · F7")
            action(session, f"{filename}:split-feedback", "1", "REVIEW · F7")
            session.wait(expected)
            action(session, f"{filename}:tab-chooser", "\x1b[18~", "REVIEW OPTIONS · F7")
            action(session, f"{filename}:review-tab", "2", " REVIEW   Ln")
            session.send("must not edit")
            session.read(0.05)
            action(session, f"{filename}:source", "\x1b", " EDIT   Ln")
            assert path.read_bytes() == text.encode()
            session.send("\x11")
            session.wait("TERM_STATE_OK")
            session.wait("CHILD_EXIT=0")
        finally:
            (ROOT / "target" / f"pty-review-{filename}.log").write_bytes(session.output)
            session.close()


def large_review(root, size):
    path = root / "large.json"
    path.write_bytes(b'{"payload":"' + b'a'*(size-14) + b'"}')
    session = Session([str(BINARY), str(path)])
    try:
        session.wait("Opened")
        started = time.perf_counter()
        action(session, "large-json:review-chooser", "\x1b[18~", "REVIEW OPTIONS · F7")
        action(session, "large-json:review-feedback", "1", "REVIEW · F7")
        for _ in range(3):
            action(session, "large-json:settings-open", "\x1b[19~", "SETTINGS · F8")
            action(session, "large-json:vi-during-render", "v\r", "SOẠN THẢO")
            action(session, "large-json:settings-open-vi", "\x1b[19~", "THIẾT LẬP · F8")
            action(session, "large-json:en-during-render", "e\r", "EDITOR")
        session.wait("JSON · formatted view", timeout=30)
        COMPLETIONS.append({"operation": "10MiB JSON review completion including locale changes",
                            "ms": round((time.perf_counter()-started)*1000, 3)})
        session.send("\x11")
        session.wait("TERM_STATE_OK")
        session.wait("CHILD_EXIT=0")
    finally:
        (ROOT / "target" / "pty-review-large.log").write_bytes(session.output)
        session.close()


def workflow(root):
    first, second = root / "alpha.txt", root / "beta.txt"
    first.write_text("alpha-marker", encoding="utf-8")
    second.write_text("beta-marker", encoding="utf-8")
    (root / "nested").mkdir()
    (root / "nested" / "child.txt").write_text("child", encoding="utf-8")
    session = Session([str(BINARY), str(first), str(second)])
    try:
        session.wait("beta-marker")
        action(session, "workflow:next-tab", "\x1b[17~", "alpha-marker")
        session.send("\x01")
        session.read(0.01)
        action(session, "workflow:copy", "\x03", "Copied to internal clipboard")
        action(session, "workflow:new-tab", "\x0e", "New buffer")
        action(session, "workflow:paste", "\x16", "alpha-marker")
        action(session, "workflow:undo", "\x1a", "Ln 1, Col 1 ")
        action(session, "workflow:redo", "\x19", "Ln 1, Col 13 ")
        action(session, "workflow:dirty-close", "\x17", "Discard unsaved changes")
        action(session, "workflow:cancel-close", "n", lambda s: "UNSAVED CHANGES / CONFIRM" not in s)
        action(session, "workflow:open-prompt", "\x0f", "Open file or folder")
        action(session, "workflow:dismiss-prompt", "\x1b", lambda s: "OPEN FILE" not in s)
        action(session, "workflow:find-prompt", "\x06", "Find literal")
        action(session, "workflow:find-response", "alpha\r", lambda s: "Finding" in s or "Match selected" in s)
        session.wait("Match selected")
        action(session, "workflow:show-tree", "\x02", "EXPLORER")
        action(session, "workflow:focus-editor", "\x1bOS", " EDIT   Ln")
        action(session, "workflow:hide-tree", "\x02", lambda s: "EXPLORER" not in s)
        action(session, "workflow:folder-prompt", "\x0b", "Open folder")
        action(session, "workflow:folder-response", str(root)+"\r", "Workspace opened")
        session.wait("nested")
        action(session, "workflow:expand-tree", "\x1b[H\x1b[C", "child.txt")
        action(session, "workflow:collapse-tree", "\x1b[D", lambda s: "child.txt" not in s)
        action(session, "workflow:quit-prompt", "\x11", "Discard ALL")
        session.send("y")
        session.wait("TERM_STATE_OK")
        session.wait("CHILD_EXIT=0")
    finally:
        (ROOT / "target" / "pty-workflow-budget.log").write_bytes(session.output)
        session.close()


if __name__ == "__main__":
    size = int(sys.argv[1]) if len(sys.argv) > 1 else 10*1024*1024
    try:
        with tempfile.TemporaryDirectory(prefix="guineanote-budget-") as directory:
            root = Path(directory)
            editor(root, size)
            editor(root, size, unicode=True)
            reviews(root)
            large_review(root, size)
            workflow(root)
    finally:
        report = {"budget_ms": 100, "input_bytes_max": size, "platform": platform.platform(),
                  "sample_count": len(RESULTS), "samples": RESULTS,
                  "background_completion": COMPLETIONS,
                  "max_ms": max((result["ms"] for result in RESULTS), default=None),
                  "failures": [result for result in RESULTS if result["ms"] >= 100]}
        output = json.dumps(report, indent=2, ensure_ascii=False)
        (ROOT / "target" / "responsiveness.json").write_text(output, encoding="utf-8")
        print(json.dumps({key: value for key, value in report.items() if key != "samples"}, indent=2))
    assert not report["failures"], "Interactive response budget exceeded; see target/responsiveness.json"
