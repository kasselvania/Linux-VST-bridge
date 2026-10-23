import importlib.util
import struct
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).parents[1] / "controls.py"
SPEC = importlib.util.spec_from_file_location("shieldxl0_controls", MODULE_PATH)
controls = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = controls
SPEC.loader.exec_module(controls)


class ControlsTest(unittest.TestCase):
    def test_encoder_event_is_typed_with_monotonic_timestamp(self):
        raw = controls.EVENT.pack(12, 345, controls.EV_REL, controls.REL_X, -1)
        event_type, code, value, timestamp = next(controls.decode_events(raw))
        event = controls.typed_event(controls.DEVICES[0], event_type, code, value, timestamp)
        self.assertEqual(
            event,
            {"type": "encoder", "index": 0, "delta": -1, "monotonic_ns": 12_000_345_000},
        )

    def test_button_press_and_release_are_both_preserved(self):
        device = controls.DEVICES[4]
        pressed = controls.typed_event(device, controls.EV_KEY, controls.BTN_0 + 1, 1, 10)
        released = controls.typed_event(device, controls.EV_KEY, controls.BTN_0 + 1, 0, 20)
        self.assertTrue(pressed["pressed"])
        self.assertFalse(released["pressed"])
        self.assertEqual(pressed["index"], 1)

    def test_partial_record_is_rejected(self):
        with self.assertRaises(ValueError):
            list(controls.decode_events(b"partial"))

    def test_repeat_and_wrong_codes_are_not_contract_events(self):
        device = controls.DEVICES[3]
        self.assertIsNone(controls.typed_event(device, controls.EV_KEY, controls.BTN_0, 2, 10))
        self.assertIsNone(controls.typed_event(device, controls.EV_KEY, controls.BTN_0 + 1, 1, 10))


if __name__ == "__main__":
    unittest.main()
