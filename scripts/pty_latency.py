"""Measure native editor input-to-terminal-output latency on single-line Rust.

Build release first; install the same dev-only Python tools as pty_smoke.py.
This includes PTY transport and pyte parsing, not physical display/font latency.
No timing threshold is used as a flaky CI gate. Optional byte sizes as arguments.
"""
import json
import os
from pathlib import Path
import platform
import sys
import tempfile
import time

from pty_smoke import ROOT, Session
import psutil


def measure(size, samples=31):
    with tempfile.TemporaryDirectory(prefix="guineanote-latency-") as directory:
        path = Path(directory) / "long-line.rs"
        prefix, suffix = 'let value = "', '";'
        assert size >= len(prefix) + len(suffix)
        original = prefix + "a" * (size - len(prefix) - len(suffix)) + suffix
        path.write_bytes(original.encode())
        binary = ROOT / "target" / "release" / ("guineanote.exe" if os.name == "nt" else "guineanote")
        session = Session([str(binary), str(path)])
        try:
            session.wait("Opened")
            editor = next(process for process in psutil.Process(session.pid).children(recursive=True)
                          if process.name().lower().startswith("guineanote"))
            memory = editor.memory_info()
            rss_samples = [memory.rss]
            latencies = []
            for index in range(samples):
                start = time.perf_counter()
                session.send("x")
                session.wait(f"Ln 1, Col {index + 2} |")
                latencies.append((time.perf_counter() - start) * 1000)
                rss_samples.append(editor.memory_info().rss)
            session.send("\x13")
            session.wait("Saved")
            session.wait_file(path, "x" * samples + original)
            memory = editor.memory_info()
            rss_samples.append(memory.rss)
            peak = getattr(memory, "peak_wset", None)
            session.send("\x11")
            session.wait("TERM_STATE_OK")
            session.wait("CHILD_EXIT=0")
            latencies.sort()
            return {
                "bytes": size, "samples": samples, "workload": "type_at_start_of_single_line_rust",
                "p50_input_to_output_ms": round(latencies[samples // 2], 3),
                "p95_input_to_output_ms": round(latencies[samples * 95 // 100], 3),
                "max_input_to_output_ms": round(latencies[-1], 3),
                "max_sampled_rss_bytes": max(rss_samples),
                "os_peak_working_set_bytes": peak,
                "saved_bytes_and_terminal_cleanup": "PASS",
            }
        finally:
            (ROOT / "target" / f"pty-latency-{size}.log").write_bytes(session.output)
            session.close()


if __name__ == "__main__":
    sizes = [int(argument) for argument in sys.argv[1:]] or [1024, 1048576]
    print(json.dumps({
        "platform": platform.platform(), "python": platform.python_version(),
        "results": [measure(size) for size in sizes],
    }, indent=2))
