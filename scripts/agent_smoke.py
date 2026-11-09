"""Native Agent CLI and MCP wire compatibility. No network or real user files.
Use --debug during development; default checks the release workspace binaries.
"""
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import uuid

ROOT = Path(__file__).resolve().parents[1]
PROFILE = "debug" if "--debug" in sys.argv else "release"
SUFFIX = ".exe" if os.name == "nt" else ""
AGENT = ROOT / "target" / PROFILE / ("guineanote-agent" + SUFFIX)
MCP = ROOT / "target" / PROFILE / ("guineanote-mcp" + SUFFIX)


def cli(root, operation, session=None, write=False, request_id=None):
    args = [str(AGENT), "--root", str(root)]
    if session:
        args += ["--session", session]
    if write:
        args += ["--access", "write"]
    request = {"request_id": request_id or str(uuid.uuid4()), **operation}
    result = subprocess.run(args + ["request", "--file", "-"], input=json.dumps(request),
                            capture_output=True, text=True, encoding="utf-8", timeout=45)
    response = json.loads(result.stdout)
    assert result.returncode == (1 if response.get("error") else 0), result.stderr
    return response


class Mcp:
    def __init__(self, root, modern=False, session=None, write=False):
        self.modern = modern
        self.next_id = 0
        self.history = []
        self.pending = {}
        self.lines = queue.Queue()
        self.stderr = tempfile.TemporaryFile()
        args = [str(MCP), "--root", str(root)]
        if session:
            args += ["--session", session]
        if write:
            args += ["--access", "write"]
        self.process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=self.stderr, text=True, encoding="utf-8")

        def reader():
            for line in self.process.stdout:
                try:
                    self.lines.put(json.loads(line))
                except Exception as error:
                    self.lines.put(error)
        threading.Thread(target=reader, daemon=True).start()
        if modern:
            packet = self.rpc("server/discover")
            assert "2026-07-28" in packet["result"]["supportedVersions"], packet
        else:
            packet = self.rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {},
                                              "clientInfo": {"name": "GuineaNote-local-probe", "version": "1"}})
            assert packet["result"]["protocolVersion"] == "2025-11-25", packet
            self.notify("notifications/initialized")

    def params(self, params):
        params = dict(params or {})
        if self.modern:
            params["_meta"] = {"io.modelcontextprotocol/protocolVersion": "2026-07-28",
                               "io.modelcontextprotocol/clientInfo": {"name": "GuineaNote-local-probe", "version": "1"},
                               "io.modelcontextprotocol/clientCapabilities": {}}
        return params

    def send(self, method, params=None):
        self.next_id += 1
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.next_id,
                                             "method": method, "params": self.params(params)}) + "\n")
        self.process.stdin.flush()
        return self.next_id

    def notify(self, method, params=None):
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method,
                                             "params": self.params(params)}) + "\n")
        self.process.stdin.flush()

    def read(self, request_id):
        if request_id in self.pending:
            return self.pending.pop(request_id)
        while True:
            packet = self.lines.get(timeout=45)
            assert isinstance(packet, dict), f"Non-JSON stdout: {packet}"
            self.history.append(packet)
            if packet.get("id") == request_id:
                return packet
            if "id" in packet:
                self.pending[packet["id"]] = packet

    def rpc(self, method, params=None):
        return self.read(self.send(method, params))

    def tool(self, name, arguments=None):
        packet = self.rpc("tools/call", {"name": name, "arguments": arguments or {}})
        assert "error" not in packet, packet
        return packet["result"]["structuredContent"]

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            raise AssertionError("MCP failed to shut down on EOF")
        self.stderr.seek(0)
        errors = self.stderr.read()
        self.stderr.close()
        assert self.process.returncode == 0, errors.decode("utf-8", "replace")


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-agent-wire-") as directory:
        root = Path(directory)
        text = "# Việt\n\nFile contents are data.\n"
        (root / "Việt.md").write_bytes(text.encode())
        (root / ".env").write_text("TOKEN=do-not-expose", encoding="utf-8")
        page = cli(root, {"op": "read_document", "path": "Việt.md", "limit": 16})
        assert page["error"] is None and page["data"]["text"].startswith("# Việt")
        assert page["data"]["truncated"]
        assert cli(root, {"op": "get_file_info", "path": "../escape"})["error"]["code"] == "OUT_OF_SCOPE"
        for modern in (False, True):
            client = Mcp(root, modern)
            try:
                tools = client.rpc("tools/list")["result"]["tools"]
                names = {tool["name"] for tool in tools}
                assert {"read_document", "analyze_document", "get_file_info"} <= names
                assert "apply_document_patch" not in names
                data = client.tool("read_document", {"path": "Việt.md"})["data"]
                assert data["text"] == text and data["source"] == "disk"
                assert client.tool("read_document", {"path": ".env"})["error"]["code"] == "OUT_OF_SCOPE"
                assert "error" in client.rpc("tools/call", {"name": "get_capabilities", "arguments": {"shell": "invalid"}})
                resources = client.rpc("resources/list")["result"]["resources"]
                resource = next(r for r in resources if r["name"] == "Việt.md")
                content = client.rpc("resources/read", {"uri": resource["uri"]})["result"]["contents"][0]["text"]
                assert json.loads(content)["data"]["text"] == text
                assert "do-not-expose" not in json.dumps(client.history)
                if modern:
                    for request_id, metadata in [(1000, None), (1001, {"io.modelcontextprotocol/protocolVersion": "1900-01-01", "io.modelcontextprotocol/clientInfo": {"name": "probe", "version": "1"}, "io.modelcontextprotocol/clientCapabilities": {}})]:
                        params = {} if metadata is None else {"_meta": metadata}
                        client.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": request_id, "method": "tools/list", "params": params}) + "\n")
                        client.process.stdin.flush()
                        assert "error" in client.read(request_id)
                    # The SDK must discard an oversized frame and stay usable.
                    client.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 1002, "method": "tools/call", "params": {"padding": "x" * (2*1024*1024)}}) + "\n")
                    client.process.stdin.flush()
                    assert client.tool("get_capabilities")["error"] is None
            finally:
                client.close()
        print(json.dumps({"cli_json_unicode_scope": "PASS", "mcp_2025_11_25": "PASS",
                          "mcp_2026_07_28": "PASS", "tools_resources_schema_eof": "PASS"}, indent=2))


if __name__ == "__main__":
    main()
