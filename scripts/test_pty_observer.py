"""Reader profiling must preserve fragmented UTF-8 and terminal output."""
import codecs
import unittest
from unittest.mock import Mock, patch

from pty_observer import ProfiledSession, observer_delta
import pyte


class ObserverTests(unittest.TestCase):
    def session(self):
        session = object.__new__(ProfiledSession)
        session.fd = Mock()
        session.output = b""
        session.screen = pyte.Screen(20, 2)
        session.stream = pyte.Stream(session.screen)
        session.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        session.observer_totals = dict.fromkeys(
            ("select_wait_ns", "receive_ns", "append_ns", "decode_feed_ns",
             "display_ns", "read_calls", "chunks", "bytes"), 0
        )
        return session

    def test_fragmented_unicode_stops_at_visible_feedback(self):
        session = self.session()
        chunks = [b"\x1b[2J\xc3", b"\xa9OK", b"not consumed"]
        with patch("pty_observer.os.name", "nt"), patch(
            "pty_observer.select.select", return_value=([session.fd], [], [])
        ):
            session.fd.recv.side_effect = chunks
            before = dict(session.observer_totals)
            session.read(until="éOK")
        self.assertEqual(session.output, b"\x1b[2J\xc3\xa9OK")
        self.assertTrue(session.screen.display[0].startswith("éOK"))
        delta = observer_delta(before, session.observer_totals)
        self.assertEqual(delta["chunks"], 2)
        self.assertEqual(delta["bytes"], len(session.output))
        self.assertEqual(delta["read_calls"], 1)
        self.assertTrue(all(value >= 0 for value in delta.values()))

    def test_no_output_does_not_decode_or_change_screen(self):
        session = self.session()
        with patch("pty_observer.select.select", return_value=([], [], [])):
            session.read()
        self.assertEqual(session.output, b"")
        self.assertEqual(session.observer_totals["chunks"], 0)
        self.assertEqual(session.observer_totals["decode_feed_ns"], 0)

    def test_transport_error_preserves_already_observed_output(self):
        session = self.session()
        with patch("pty_observer.os.name", "nt"), patch(
            "pty_observer.select.select", return_value=([session.fd], [], [])
        ):
            session.fd.recv.side_effect = [b"OK", OSError("closed")]
            session.read()
        self.assertEqual(session.output, b"OK")
        self.assertEqual(session.observer_totals["chunks"], 1)


if __name__ == "__main__":
    unittest.main()
