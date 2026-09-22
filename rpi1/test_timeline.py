import json
from pathlib import Path
import tempfile
import unittest

from timeline import analyze, records
from xdamage_observer import exact_window


def phase(stamp, call, position, name, v1=0, v2=0, detail=0):
    return {"event": "rpi1_phase", "monotonic_ns": stamp,
            "callback_sequence": call, "bridge_position": position,
            "phase": name, "value_1": v1, "value_2": v2, "detail": detail}


class TimelineTests(unittest.TestCase):
    def test_queue_delay_is_separate_from_callback_duration_and_host_reply(self):
        rows = [phase(100, 1, 0, "callback_enter"),
                phase(110, 1, 0, "request_slot_inspect"),
                phase(120, 1, 0, "request_published", 7),
                phase(300, 1, 0, "worker_request_observed", 7),
                phase(320, 1, 0, "worker_prepared"),
                phase(330, 1, 0, "worker_sent"),
                phase(800, 1, 0, "worker_replied", 0, 250),
                phase(900, 1, 512, "callback_exit", 0, 0)]
        result = analyze(rows)["phase_distributions"]
        self.assertEqual(result["callback_duration_ns"]["max_ns"], 800)
        self.assertEqual(result["request_publication_ns"]["max_ns"], 10)
        self.assertEqual(result["worker_queue_delay_ns"]["max_ns"], 180)
        self.assertEqual(result["host_reply_ns"]["max_ns"], 470)
        self.assertEqual(result["windows_process_ns"]["max_ns"], 250)

    def test_missing_capture_is_reported_separately_from_audio_gap(self):
        rows = [phase(100, 1, 0, "callback_enter"),
                phase(200, 4, 0, "callback_enter"),
                phase(210, 4, 0, "callback_exit", 0, 0)]
        result = analyze(rows)
        self.assertEqual(result["capture_holes"], [
            {"first_missing": 2, "last_missing": 3, "count": 2}])
        self.assertEqual(result["longest_consecutive_missing_callbacks"], 0)

    def test_only_pi_monotonic_records_are_merged(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.jsonl"
            path.write_text("RPI1_THREAD " + json.dumps({
                "monotonic_seconds": 12.5, "tid": 5}) + "\n" +
                json.dumps({"event": "ap9_timing", "max_ns": 42}) + "\n")
            output = list(records(path))
            self.assertEqual(len(output), 1)
            self.assertEqual(output[0]["monotonic_ns"], 12_500_000_000)
            self.assertEqual(output[0]["event"], "rpi1_thread")

    def test_exact_editor_title_refuses_ambiguity(self):
        self.assertEqual(exact_window('0xab "Pigments": x'), 0xab)
        with self.assertRaises(ValueError):
            exact_window('0xab "Pigments": x\n0xcd "Pigments": y')


if __name__ == "__main__":
    unittest.main()
