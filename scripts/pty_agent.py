"""Native live Agent/MCP editing and AI read-only leases. Owned fixtures only.
Run --debug for functionality; release is used for final latency evidence.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid
from agent_smoke import AGENT, ROOT, PROFILE, SUFFIX, Mcp, cli
from pty_smoke import Session
from tui_preview import snapshot

BINARY = ROOT / "target" / PROFILE / ("guinea" + SUFFIX)


def data(response):
    assert response.get("error") is None, response.get("error")
    return response["data"]


def settle(session):
    session.read(.04)
    session.wait_until(lambda lines: "AI · READ ONLY" not in "\n".join(lines))


def launch(root, session_id, operation, request_id=None):
    args = [str(AGENT), "--root", str(root), "--session", session_id,
            "--access", "write", "request", "--file", "-"]
    process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, encoding="utf-8")
    process.stdin.write(json.dumps({"request_id": request_id or str(uuid.uuid4()), **operation}))
    process.stdin.close()
    return process


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-live-") as directory, tempfile.TemporaryDirectory(prefix="guineanote-runtime-") as runtime:
        root = Path(directory)
        os.environ["GUINEANOTE_SESSION_DIR"] = str(Path(runtime) / "sessions")
        source = root / "note.md"
        base = "# Title\r\nViệt 👩‍💻\r\n"
        source.write_bytes(b"\xef\xbb\xbf" + base.encode())
        (root / ".env").write_text("TOKEN=private-fixture", encoding="utf-8")
        ui = Session([str(BINARY), "--agent", "--agent-write", str(root), str(source)], colors=True)
        try:
            ui.resize(42, 148)
            ui.wait("Opened")
            sessions = data(cli(root, {"op": "list_editor_sessions"}))["sessions"]
            assert len(sessions) == 1
            session_id = sessions[0]["session"]
            tabs = data(cli(root, {"op": "list_tabs"}, session_id))["tabs"]
            original_tab = next(t for t in tabs if t["path"] and t["path"].endswith("note.md"))
            document = original_tab["document"]
            ui.send("draft ")
            ui.wait("MODIFIED")
            current = data(cli(root, {"op": "read_document", "document": document}, session_id))
            assert current["text"] == "draft " + base
            assert source.read_bytes() == b"\xef\xbb\xbf" + base.encode()
            assert cli(root, {"op": "read_document", "path": ".env"}, session_id)["error"]["code"] == "OUT_OF_SCOPE"
            assert cli(root, {"op": "get_file_info", "path": "../escape"}, session_id)["error"]["code"] == "OUT_OF_SCOPE"

            plan_args = {"op": "plan_document_patch", "document": document,
                         "expected_revision": current["revision"],
                         "edits": [{"start": 0, "end": 6, "text": "agent "}]}
            plan = data(cli(root, plan_args, session_id, True, "plan-first"))
            assert data(cli(root, plan_args, session_id, True, "plan-first"))["plan"] == plan["plan"]
            assert cli(root, {**plan_args, "expected_revision": 0}, session_id, True, "plan-first")["error"]["code"] == "IDEMPOTENCY_CONFLICT"
            assert data(cli(root, {"op": "read_document", "document": document}, session_id))["text"] == "draft " + base
            settle(ui)
            ui.send("user ")
            ui.wait("Ln 1, Col 12 ")
            assert cli(root, {"op": "apply_document_patch", "plan": plan["plan"]}, session_id, True)["error"]["code"] == "REVISION_CONFLICT"
            current = data(cli(root, {"op": "read_document", "document": document}, session_id))
            invalid = cli(root, {"op": "plan_document_patch", "document": document,
                                "expected_revision": current["revision"],
                                "edits": [{"start": 0, "end": 5, "text": "x"}, {"start": 2, "end": 3, "text": "y"}]}, session_id, True)
            assert invalid["error"]["code"] == "INVALID_ARGUMENT"
            plan = data(cli(root, {"op": "plan_document_patch", "document": document,
                                   "expected_revision": current["revision"],
                                   "edits": [{"start": 0, "end": 11, "text": "agent "}]}, session_id, True))
            applied = data(cli(root, {"op": "apply_document_patch", "plan": plan["plan"]}, session_id, True, "apply-once"))
            assert data(cli(root, {"op": "apply_document_patch", "plan": plan["plan"]}, session_id, True, "apply-once"))["revision"] == applied["revision"]
            assert data(cli(root, {"op": "read_document", "document": document}, session_id))["text"] == "agent " + base
            settle(ui)
            ui.send("\x1a")
            ui.read(.05)
            assert data(cli(root, {"op": "read_document", "document": document}, session_id))["text"] == "draft user " + base
            settle(ui)
            ui.send("\x19")
            ui.read(.05)
            current = data(cli(root, {"op": "read_document", "document": document}, session_id))
            data(cli(root, {"op": "save_document", "document": document,
                            "expected_revision": current["revision"]}, session_id, True, "save-exact"))
            assert source.read_bytes() == b"\xef\xbb\xbf" + ("agent " + base).encode()
            assert data(cli(root, {"op": "get_transaction", "transaction_id": "save-exact"}, session_id))["state"] == "completed"
            # A semantic no-op must not dirty or create a new revision.
            noop = data(cli(root, {"op": "plan_document_patch", "document": document,
                                  "expected_revision": current["revision"],
                                  "edits": [{"start": 0, "end": 6, "text": "agent "}]}, session_id, True))
            unchanged = data(cli(root, {"op": "apply_document_patch", "plan": noop["plan"]}, session_id, True))
            assert unchanged["changed"] is False and unchanged["revision"] == current["revision"]
            committed = source.read_bytes()
            source.write_bytes(b"external change")
            conflict = cli(root, {"op": "save_document", "document": document,
                                 "expected_revision": current["revision"]}, session_id, True)
            assert conflict["error"]["code"] == "DISK_CONFLICT"
            assert source.read_bytes() == b"external change"
            source.write_bytes(committed)
            settle(ui)
            ui.command("reload")
            ui.wait("Reload from disk")
            ui.send("y")
            ui.wait("Reloaded")

            for modern in (False, True):
                client = Mcp(root, modern, session_id, write=True)
                try:
                    assert data(client.tool("read_document", {"document": document}))["text"] == "agent " + base
                    new = data(client.tool("new_document", {"request_id": "mcp-new-" + str(modern)}))
                    assert new["document"] != document
                    assert data(client.tool("read_document", {"document": new["document"]}))["text"] == ""
                    resources = client.rpc("resources/list")["result"]["resources"]
                    resource = next(r for r in resources if r["name"] == "note.md")
                    packet = client.rpc("resources/read", {"uri": resource["uri"]})
                    assert json.loads(packet["result"]["contents"][0]["text"])["data"]["text"] == "agent " + base
                    data(client.tool("close_tab", {"tab": new["tab"], "request_id": "mcp-close-" + str(modern)}))
                finally:
                    client.close()

            size = 10 * 1024 * 1024
            large = root / "large.json"
            prefix, suffix = '{"BIGMARKER":[', '0]}'
            content = prefix + '0,' * ((size - len(prefix) - len(suffix)) // 2) + suffix
            content += ' ' * (size - len(content))
            large.write_bytes(content.encode())
            large_doc = data(cli(root, {"op": "open_document", "path": "large.json"}, session_id, True))
            ui.wait("BIGMARKER")
            process = launch(root, session_id, {"op": "analyze_document", "document": large_doc["document"]})
            ui.wait("AI · READ ONLY")
            ui.read(.015)
            snapshot(ui, "agent-ai-readonly")
            ui.send("SHOULD_NOT_APPEAR")
            ui.wait("AI is working in this tab")
            ui.send("\x1b[17~")
            ui.wait("agent # Title")
            ui.send("other ")
            ui.wait("MODIFIED")
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
            ui.wait_until(lambda lines: "AI 1" not in "\n".join(lines), timeout=15)
            tabs = data(cli(root, {"op": "list_tabs"}, session_id))["tabs"]
            assert all(not t["ai"] for t in tabs)
            assert not next(t for t in tabs if t["document"] == large_doc["document"])["readonly"]
            assert data(cli(root, {"op": "read_document", "document": large_doc["document"], "limit": 32}, session_id))["text"].startswith('{"BIGMARKER"')
            current = data(cli(root, {"op": "read_document", "document": document}, session_id))
            assert current["text"] == "other agent " + base
            data(cli(root, {"op": "save_document", "document": document,
                            "expected_revision": current["revision"]}, session_id, True))
            assert source.read_bytes() == b"\xef\xbb\xbf" + ("other agent " + base).encode()

            # Cancellation while parsing must release the live-document lease
            # and update a tab manager that was opened while the lease existed.
            client = Mcp(root, True, session_id)
            try:
                cancel_id = client.send("tools/call", {"name": "analyze_document", "arguments": {"document": large_doc["document"]}})
                ui.wait("AI 1")
                ui.send("\x10")
                ui.wait("OPEN TABS")
                client.notify("notifications/cancelled", {"requestId": cancel_id, "reason": "local cancellation probe"})
                ui.wait_until(lambda lines: "AI 1" not in "\n".join(lines) and "AI · READ ONLY" not in "\n".join(lines), timeout=20)
                assert client.tool("get_capabilities")["error"] is None
                assert not any(p.get("id") == cancel_id for p in client.history), "Response emitted after acknowledged cancellation"
                ui.send("\x1b")
                ui.wait_until(lambda lines: "OPEN TABS" not in "\n".join(lines))
            finally:
                client.close()

            # One heavy request holds the worker while eight more exercise the
            # MCP admission cap. Expected overload is a typed tool error.
            client = Mcp(root, True, session_id)
            try:
                heavy = client.send("tools/call", {"name": "analyze_document", "arguments": {"document": large_doc["document"]}})
                ui.wait("AI 1")
                queued = [client.send("tools/call", {"name": "get_capabilities", "arguments": {}}) for _ in range(8)]
                replies = [client.read(request) for request in queued]
                assert any(packet.get("result", {}).get("structuredContent", {}).get("error", {}).get("code") == "BUSY" for packet in replies if packet.get("result", {}).get("isError")), replies
                assert data(client.read(heavy)["result"]["structuredContent"])["format"] == "JSON"
            finally:
                client.close()

            data(cli(root, {"op": "activate_tab", "tab": large_doc["tab"]}, session_id))
            settle(ui)
            ui.send("\x1b[H\x1b[3~[")  # Replace one ASCII scalar, keeping exactly 10 MiB.
            ui.wait("MODIFIED")
            tabs = data(cli(root, {"op": "list_tabs"}, session_id))["tabs"]
            revision = next(t for t in tabs if t["tab"] == large_doc["tab"])["revision"]
            process = launch(root, session_id, {"op": "save_document", "document": large_doc["document"],
                                                "expected_revision": revision}, "save-client-kill")
            ui.wait("AI · READ ONLY")
            ui.send("\x1b[19~")
            ui.wait("SETTINGS")
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
            ui.send("\x1b")
            ui.wait_until(lambda lines: "SETTINGS" not in "\n".join(lines))
            ui.wait_until(lambda lines: "AI 1" not in "\n".join(lines), timeout=15)
            transaction = data(cli(root, {"op": "get_transaction", "transaction_id": "save-client-kill"}, session_id))
            observed = large.read_bytes()
            if transaction["response"]["error"] is None:
                assert observed == ("[" + content[1:]).encode()
            else:
                assert transaction["response"]["error"]["code"] == "CANCELLED"
                assert observed == content.encode()
                current = next(t for t in data(cli(root, {"op": "list_tabs"}, session_id))["tabs"] if t["tab"] == large_doc["tab"])
                data(cli(root, {"op": "save_document", "document": large_doc["document"],
                                "expected_revision": current["revision"]}, session_id, True))
            settle(ui)
            ui.send("\x11")
            ui.wait("TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            assert not list((Path(runtime) / "sessions").glob("*.json"))
            print(json.dumps({"live_unsaved_read": "PASS", "patch_stale_idempotency_one_undo": "PASS",
                              "noop_revision_and_external_disk_conflict": "PASS",
                              "unicode_bom_crlf_save": "PASS", "mcp_live_both_versions": "PASS",
                              "AI_badge_readonly_other_tab_edit": "PASS", "client_death_cleanup": "PASS",
                              "mcp_cancel_tab_manager_and_overload": "PASS",
                              "save_client_death_modal_and_transaction": "PASS"}, indent=2))
        finally:
            (ROOT / "target/pty-agent.log").write_bytes(ui.output)
            ui.close()


if __name__ == "__main__":
    main()
