"""Measure fresh UI feedback while an Agent analyzes a 10 MiB live buffer.
Release binaries, owned fixtures, no network. API completion is separate.
"""
import json
import argparse
import os
import platform
from pathlib import Path
import tempfile
import threading
import time
from agent_smoke import ROOT, Mcp, cli
from pty_agent import BINARY, data, settle
from pty_responsiveness import action, RESULTS
from pty_smoke import Session


def ai_active(ui):
    screen = "\n".join(ui.screen.display)
    return "AI 1" in screen or "AI · READ ONLY" in screen


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--report-dir',type=Path,default=ROOT/'target')
    parser.add_argument('--record-only',action='store_true')
    args=parser.parse_args();args.report_dir.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="guineanote-ai-perf-") as directory, tempfile.TemporaryDirectory(prefix="guineanote-ai-runtime-") as runtime:
        root = Path(directory)
        os.environ["GUINEANOTE_SESSION_DIR"] = str(Path(runtime) / "sessions")
        size = 10 * 1024 * 1024
        text = '[0' + ',0' * ((size - 3) // 2) + ']'
        text += ' ' * (size - len(text))
        source = root / "large.json"
        source.write_bytes(text.encode())
        ui = Session([str(BINARY), "--agent", "--agent-write", str(source)])
        client = None
        try:
            ui.wait("Opened")
            session = data(cli(root, {"op": "list_editor_sessions"}))["sessions"][0]["session"]
            document = data(cli(root, {"op": "list_tabs"}, session))["tabs"][0]["document"]
            client = Mcp(root, True, session)
            completion = []
            started = time.perf_counter()
            request = client.send("tools/call", {"name": "analyze_document", "arguments": {"document": document, "limit": 1}})

            def receive():
                try:
                    response = client.read(request)
                    assert response["result"]["structuredContent"]["error"] is None, response
                    completion.append({"ms": round((time.perf_counter()-started)*1000, 3)})
                except Exception as error:
                    completion.append({"error": str(error)})
            worker = threading.Thread(target=receive)
            worker.start()
            ui.wait("AI · READ ONLY")

            def measure(name, keys, expected):
                before = ai_active(ui) and not completion
                action(ui, name, keys, expected)
                RESULTS[-1]["AI_active_at_input"] = before

            for index in range(10):
                measure("ai:navigate-locked-source", "\x1b[C", f"Ln 1, Col {index+2} ")
            measure("ai:blocked-edit-feedback", "x", "AI is working in this tab")
            measure("ai:settings-open", "\x1b[21~", "SETTINGS · F10")
            measure("ai:settings-cancel", "\x1b", lambda s: "SETTINGS · F10" not in s)
            measure("ai:tabs-open", "\x10", "OPEN TABS · Ctrl+P")
            measure("ai:tabs-cancel", "\x1b", lambda s: "OPEN TABS · Ctrl+P" not in s)
            measure("ai:new-other-tab", "\x14", "Tabs 2/2")
            for index in range(10):
                measure("ai:type-other-tab", "a", f"Ln 1, Col {index+2} ")
            measure("ai:settings-on-other-tab", "\x1b[21~", "SETTINGS · F10")
            measure("ai:cancel-other-tab-settings", "\x1b", lambda s: "SETTINGS · F10" not in s)
            worker.join(timeout=45)
            assert not worker.is_alive() and completion and "error" not in completion[0], completion
            ui.wait_until(lambda lines: "AI 1" not in "\n".join(lines))
            active = [r for r in RESULTS if r["AI_active_at_input"]]
            assert len(active) >= 20, f"Insufficient concurrent samples: {len(active)}"
            report = {"platform": platform.platform(), "profile": "release", "file_bytes": size,
                      "workload": "MCP analyze live JSON array while human navigates, opens dialogs and edits another tab",
                      "samples": len(RESULTS), "concurrent_samples": len(active),
                      "max_ms": max(r["ms"] for r in RESULTS), "investigation_threshold_ms": 50,
                      "agent_analysis_completion_ms": completion[0]["ms"], "results": RESULTS}
            (args.report_dir / "responsiveness-agent.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
            assert data(cli(root, {"op": "read_document", "document": document, "limit": 16}, session))["text"].startswith('[0,0'), "Human changed the locked source"
            settle(ui)
            ui.send("\x11")
            ui.wait("UNSAVED CHANGES")
            ui.send("y")
            ui.wait("TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            print(json.dumps(report, indent=2))
            if not args.record_only:
                assert report["max_ms"] <= 50, "UI response >50 ms requires investigation"
        finally:
            if client:
                client.close()
            (args.report_dir / "pty-agent-latency.log").write_bytes(ui.output)
            ui.close()


if __name__ == "__main__":
    main()
