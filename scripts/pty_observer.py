"""Opt-in timing of the existing PTY reader, without changing its wait policy.

Output wait includes application/transport/scheduling; it cannot identify the
adapter queue on its own. Timers add overhead, so compare unprofiled runs too.
"""
import os
import select
import time

from pty_smoke import Session


class ProfiledSession(Session):
    def __init__(self, *args, **kwargs):
        self.observer_totals = dict.fromkeys(
            ("select_wait_ns", "receive_ns", "append_ns", "decode_feed_ns",
             "display_ns", "read_calls", "chunks", "bytes"), 0
        )
        super().__init__(*args, **kwargs)

    def read(self, duration=0.15, until=None):
        totals = self.observer_totals
        totals["read_calls"] += 1
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            started = time.perf_counter_ns()
            ready, _, _ = select.select(
                [self.fd], [], [], max(0, deadline - time.monotonic())
            )
            totals["select_wait_ns"] += time.perf_counter_ns() - started
            if not ready:
                break
            started = time.perf_counter_ns()
            try:
                chunk = self.fd.recv(65536) if os.name == "nt" else os.read(self.fd, 65536)
            except OSError:
                break
            finally:
                totals["receive_ns"] += time.perf_counter_ns() - started
            if not chunk:
                break
            totals["chunks"] += 1
            totals["bytes"] += len(chunk)
            started = time.perf_counter_ns()
            self.output += chunk
            totals["append_ns"] += time.perf_counter_ns() - started
            started = time.perf_counter_ns()
            self.stream.feed(self.decoder.decode(chunk))
            totals["decode_feed_ns"] += time.perf_counter_ns() - started
            if until is not None:
                started = time.perf_counter_ns()
                found = until in "\n".join(self.screen.display)
                totals["display_ns"] += time.perf_counter_ns() - started
                if found:
                    break


def observer_delta(before, after):
    """Return per-action counters; cumulative totals include unmeasured setup."""
    return {
        (key[:-3] + "_ms" if key.endswith("_ns") else key):
        (round((value - before[key]) / 1_000_000, 3)
         if key.endswith("_ns") else value - before[key])
        for key, value in after.items()
    }
