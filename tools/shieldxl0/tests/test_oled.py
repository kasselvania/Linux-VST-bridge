import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).parents[1] / "oled_service.py"
SPEC = importlib.util.spec_from_file_location("shieldxl0_oled", MODULE_PATH)
oled = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = oled
SPEC.loader.exec_module(oled)


class OledTest(unittest.TestCase):
    def test_wire_shape_and_nibble_duplication(self):
        frame = oled.Frame()
        frame.set_pixel(0, 0, 0xA)
        wire = frame.wire_bytes()
        self.assertEqual(len(wire), 128 * 64)
        self.assertEqual(wire[0], 0xAA)

    def test_status_renderer_is_bounded(self):
        frame = oled.Frame()
        oled.render_request(
            frame,
            {
                "op": "status",
                "preset": "TEST",
                "macros": [0, 25, 50, 100],
                "cpu_percent": 12,
                "missing_frames": 0,
                "temperature_c": 52,
                "throttled": "NO",
            },
        )
        self.assertGreater(sum(frame.pixels), 0)
        self.assertEqual(len(frame.pixels), 8192)

    def test_invalid_gray_is_rejected(self):
        with self.assertRaises(ValueError):
            oled.render_request(oled.Frame(), {"op": "fill", "gray": 16})


if __name__ == "__main__":
    unittest.main()
