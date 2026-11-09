"""Native explicit-session isolation, server grants and original read-only state."""
import json
import os
from pathlib import Path
import tempfile
from agent_smoke import ROOT, cli
from pty_agent import BINARY, data
from pty_smoke import Session


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-multi-") as directory:
        base = Path(directory)
        os.environ["GUINEANOTE_SESSION_DIR"] = str(base / "runtime")
        roots = [base / name for name in ("a", "b")]
        editors = []
        try:
            for index, root in enumerate(roots):
                root.mkdir()
                (root / "same.txt").write_text(f"editor-{index}", encoding="utf-8")
                grant = ["--agent-write"] if index == 0 else ["--readonly"]
                ui = Session([str(BINARY), "--agent", *grant, str(root), str(root / "same.txt")])
                editors.append(ui)
                ui.wait("Opened")
            sessions = [data(cli(root, {"op": "list_editor_sessions"}))["sessions"] for root in roots]
            assert all(len(found) == 1 for found in sessions), sessions
            ids = [found[0]["session"] for found in sessions]
            assert ids[0] != ids[1]
            tabs = [data(cli(root, {"op": "list_tabs"}, sid))["tabs"][0] for root, sid in zip(roots, ids)]
            for index, (root, sid, tab) in enumerate(zip(roots, ids, tabs)):
                page = data(cli(root, {"op": "read_document", "document": tab["document"]}, sid))
                assert page["text"] == f"editor-{index}"
                cross = cli(root, {"op": "read_document", "document": tabs[1-index]["document"]}, sid)
                assert cross["error"]["code"] == "NOT_FOUND"
                assert cli(root, {"op": "list_tabs"})["error"]["code"] == "SESSION_REQUIRED"
            assert cli(roots[1], {"op": "new_document"}, ids[1], True)["error"]["code"] == "READ_ONLY"
            assert data(cli(roots[1], {"op": "list_tabs"}, ids[1]))["tabs"][0]["readonly"] is True
            assert cli(roots[1], {"op": "read_document", "document": tabs[0]["document"]}, ids[0])["error"]["code"] == "OUT_OF_SCOPE"
            for ui in editors:
                ui.read(.05)
                ui.send("\x11")
                ui.wait("TERM_STATE_OK")
                ui.wait("CHILD_EXIT=0")
            assert not list((base / "runtime").glob("*.json"))
            print(json.dumps({"two_editors_same_names": "PASS", "session_handle_isolation": "PASS",
                              "explicit_selection_only": "PASS", "server_grant_and_readonly_restore": "PASS"}, indent=2))
        finally:
            for index, ui in enumerate(editors):
                (ROOT / f"target/pty-agent-session-{index}.log").write_bytes(ui.output)
                ui.close()


if __name__ == "__main__":
    main()
