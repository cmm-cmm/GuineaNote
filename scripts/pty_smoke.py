"""Native PTY smoke and terminal-mode check; dev tooling, not a runtime dependency.

Windows: python -m pip install --target target/pty-tools pywinpty==3.1.2 psutil==7.2.2 pyte==0.8.2
Unix: python -m pip install --target target/pty-tools psutil==7.2.2 pyte==0.8.2
Run after cargo build --release:
    python scripts/pty_smoke.py
Panic cleanup (after cargo build -p guinea-tui --example panic_cleanup):
    python scripts/pty_smoke.py --panic target/debug/examples/panic_cleanup.exe
"""
import argparse
import codecs
import ctypes
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "target" / "pty-tools"))


def terminal_state():
    if os.name == "nt":
        api = ctypes.WinDLL("kernel32", use_last_error=True)
        api.GetStdHandle.restype = ctypes.c_void_p
        api.GetConsoleMode.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
        states = []
        for stream in (-10, -11):
            mode = ctypes.c_ulong()
            if not api.GetConsoleMode(api.GetStdHandle(stream), ctypes.byref(mode)):
                raise ctypes.WinError(ctypes.get_last_error())
            states.append(mode.value)
        return states
    import termios
    return termios.tcgetattr(0)


def probe_child(arguments):
    before = terminal_state()
    result = subprocess.run(arguments, check=False)
    after = terminal_state()
    print("\nTERM_STATE_OK" if before == after else "\nTERM_STATE_MISMATCH", flush=True)
    print("CHILD_EXIT=" + str(result.returncode), flush=True)
    return 0 if before == after else 1


class Session:
    def __init__(self, arguments, colors=False, preferences_path=None, env=None):
        command = [sys.executable, str(Path(__file__).resolve()), "--probe-child", *arguments]
        self.preference_directory = tempfile.TemporaryDirectory(prefix="guineanote-preferences-")
        environment = dict(os.environ)
        environment["GUINEANOTE_PREFERENCES"] = str(preferences_path or (Path(self.preference_directory.name) / "preferences.toml"))
        if env:
            environment.update(env)
        if colors:
            environment.pop("NO_COLOR", None)
            environment["TERM"] = "xterm-256color"
            environment["COLORTERM"] = "truecolor"
        if os.name == "nt":
            from winpty import PtyProcess
            self.process = PtyProcess.spawn(command, dimensions=(28, 100), env=environment)
            self.fd = self.process.fileobj
            self.pid = self.process.pid
        else:
            import fcntl
            import pty
            import termios
            master, slave = pty.openpty()
            # A controlling terminal and foreground process group are needed
            # for /dev/tty and SIGWINCH, not just isatty() on redirected FDs.
            self.process = subprocess.Popen(
                command, stdin=slave, stdout=slave, stderr=slave, close_fds=True,
                start_new_session=True,
                preexec_fn=lambda: fcntl.ioctl(0, termios.TIOCSCTTY, 0),
                env=dict(environment, TERM="xterm-256color"),
            )
            os.close(slave)
            self.fd = master
            self.pid = self.process.pid
            self.resize(28, 100)
        self.output = b""
        import pyte
        session = self
        class TerminalScreen(pyte.Screen):
            def write_process_input(self, data):
                # pyte's default is a no-op. ConPTY can request cursor status
                # while restoring cooked input; a real terminal replies. Ignoring
                # the request adds a host timeout (~500 ms) to exit measurements.
                session.send(data)

            @property
            def display(self):
                from wcwidth import wcwidth
                rows = []
                for y in range(self.lines):
                    out, skip = [], False
                    for x in range(self.columns):
                        if skip:
                            skip = False
                            continue
                        text = self.buffer[y][x].data
                        if not text:  # pyte can leave a stub after a ZWJ overwrite.
                            out.append(" ")
                            continue
                        skip = wcwidth(text[0]) == 2
                        out.append(text)
                    rows.append("".join(out))
                return rows
        self.screen = TerminalScreen(100, 28)
        self.stream = pyte.Stream(self.screen)
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def read(self, duration=0.15, until=None):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            ready, _, _ = select.select([self.fd], [], [], max(0, deadline - time.monotonic()))
            if not ready:
                break
            try:
                chunk = self.fd.recv(65536) if os.name == "nt" else os.read(self.fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            self.output += chunk
            self.stream.feed(self.decoder.decode(chunk))
            if until is not None and until in "\n".join(self.screen.display):
                break

    def wait(self, text, timeout=10):
        deadline = time.monotonic() + timeout
        while text not in "\n".join(self.screen.display):
            if time.monotonic() > deadline:
                raise AssertionError(f"Timeout waiting for {text!r}; tail={self.output[-2000:]!r}")
            self.read(until=text)

    def send(self, text):
        if os.name == "nt":
            self.process.write(text)
        else:
            os.write(self.fd, text.encode())

    def wait_until(self, predicate, timeout=10):
        deadline = time.monotonic() + timeout
        while not predicate(self.screen.display):
            if time.monotonic() >= deadline:
                raise AssertionError("Timed out waiting for screen predicate")
            self.read(0.005)

    def write(self, text):
        self.send(text)
        self.read(0.25)

    def command(self, name):
        self.write("\x1bOQ")
        self.wait("Command (")
        self.write(name + "\r")

    def wait_file(self, path, expected, timeout=10):
        deadline = time.monotonic() + timeout
        while True:
            actual = path.read_bytes() if path.exists() else None
            if actual == expected.encode("utf-8"):
                return
            if time.monotonic() > deadline:
                raise AssertionError(f"Expected saved {expected!r}, got {actual!r}")
            self.read()

    def resize(self, rows, cols):
        if hasattr(self, "screen"):
            self.screen.resize(lines=rows, columns=cols)
        if os.name == "nt":
            self.process.setwinsize(rows, cols)
        else:
            import fcntl
            import struct
            import termios
            fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    def close(self):
        if os.name == "nt":
            # The probe prints CHILD_EXIT just before its Python wrapper exits.
            # Let that normal exit finish before pywinpty attempts TerminateProcess;
            # Windows may deny termination during this short exit race.
            if b"CHILD_EXIT=" in self.output:
                deadline = time.monotonic() + 2
                while self.process.isalive() and time.monotonic() < deadline:
                    time.sleep(0.01)
            try:
                self.process.close(force=True)
            except PermissionError:
                if self.process.isalive():
                    raise
        else:
            if self.process.poll() is None:
                self.process.terminate()
                self.process.wait(timeout=5)
            os.close(self.fd)
        self.preference_directory.cleanup()


def run(binary, panic):
    with tempfile.TemporaryDirectory(prefix="guineanote-pty-") as directory:
        path = Path(directory) / "Việt file.txt"
        arguments = [str(binary)] + ([] if panic else [str(path)])
        session = Session(arguments)
        metrics = {}
        try:
            if not panic:
                session.wait("Opened")
                session.read(0.3)  # Drain the rest of the completed first frame.
                # Measure only the editor process CPU during a real idle PTY.
                try:
                    import psutil
                    processes = psutil.Process(session.pid).children(recursive=True)
                    editor = next(p for p in processes if p.name().lower().startswith("guineanote"))
                    before = editor.cpu_times()
                    start = time.monotonic()
                    output_before = len(session.output)
                    session.read(2.0)
                    after = editor.cpu_times()
                    metrics["idle_wall_seconds"] = round(time.monotonic() - start, 3)
                    metrics["idle_cpu_seconds"] = round(after.user + after.system - before.user - before.system, 6)
                    metrics["idle_output_bytes"] = len(session.output) - output_before
                    metrics["rss_bytes"] = editor.memory_info().rss
                except (ImportError, StopIteration) as error:
                    metrics["cpu_sampling"] = "NOT TESTED: " + str(error)
                session.write("hello Việt 👩‍💻")
                session.write("\x13")  # Ctrl+S
                session.wait("Saved")
                session.wait_file(path, "hello Việt 👩‍💻")
                session.command("find")
                session.wait("FIND / REPLACE")
                session.write("Việt")
                session.wait("Match 1 / 1")
                session.write("\x1b")
                session.command("replace")
                session.wait("FIND / REPLACE")
                session.write("\tViệt Nam")
                # Standard terminal Ctrl+R replaces the selected occurrence.
                session.write("\x12")
                session.wait("Replaced 1")
                session.write("\x1b")
                session.write("\x13")
                session.wait("Saved")
                session.wait_file(path, "hello Việt Nam 👩‍💻")
                # Internal copy, new tab, paste, undo/redo; unsaved confirmation.
                session.write("\x01\x03\x0e\x16")
                session.write("\x1a\x19")
                copied = Path(directory) / "copied.txt"
                session.command("save-as")
                session.wait("Save to NEW path")
                session.write(str(copied) + "\r")
                session.wait("Saved")
                session.wait_file(copied, "hello Việt Nam 👩‍💻")
                session.write("!")
                session.write("\x11")
                session.wait("Discard ALL")
                session.write("n")
                session.resize(14, 55)
                session.read(0.3)
                # F2 command fallback, abort prompt; F1 help and Escape.
                session.write("\x1bOQ")
                session.wait("Command (")
                session.write("\x1b")
                session.write("\x1bOP")
                session.wait("keyboard help")
                session.write("\x1b")
                session.write("\x11")
                session.write("y")
            session.wait("TERM_STATE_OK")
            session.wait("CHILD_EXIT=")
            if panic:
                assert b"intentional terminal cleanup probe" in session.output
                assert b"CHILD_EXIT=0" not in session.output
            else:
                assert b"CHILD_EXIT=0" in session.output
            print(json.dumps({"pty": "PASS", "panic": panic, **metrics}, indent=2))
        finally:
            (ROOT / "target" / ("pty-panic.log" if panic else "pty-smoke.log")).write_bytes(session.output)
            session.close()


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--probe-child":
        raise SystemExit(probe_child(sys.argv[2:]))
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "release" / ("guinea.exe" if os.name == "nt" else "guinea"))
    parser.add_argument("--panic", type=Path)
    args = parser.parse_args()
    run((args.panic or args.binary).resolve(), args.panic is not None)
