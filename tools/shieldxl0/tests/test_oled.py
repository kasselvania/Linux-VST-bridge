import importlib.util
import sys
import unittest
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).parents[1] / "oled_service.py"
SPEC = importlib.util.spec_from_file_location("shieldxl0_oled", MODULE_PATH)
oled = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = oled
SPEC.loader.exec_module(oled)


class OledTest(unittest.TestCase):
    class FakeLines:
        def __init__(self):
            self.values = []

        def set_value(self, offset, value):
            self.values.append((offset, value))

    class FakeSpi:
        def __init__(self):
            self.transfers = []

        def xfer2(self, values):
            self.transfers.append(values)

    class FakeLineValue:
        ACTIVE = "active"
        INACTIVE = "inactive"

    class FakeLine:
        Value = None

    class FakeCommandGpiod:
        line = None

    class FakeChip:
        opened = []

        def __init__(self, path):
            self.path = path
            self.opened.append(path)

        def get_info(self):
            return type("Info", (), {"label": "pinctrl-rp1"})()

        def close(self):
            pass

    class FakeGpiod:
        Chip = None

    def setUp(self):
        self.FakeChip.opened = []
        self.FakeGpiod.Chip = self.FakeChip
        self.FakeLine.Value = self.FakeLineValue
        self.FakeCommandGpiod.line = self.FakeLine

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

    def test_command_parameters_are_transmitted_as_data(self):
        display = object.__new__(oled.Ssd1322)
        display.gpiod = self.FakeCommandGpiod
        display.lines = self.FakeLines()
        display.spi = self.FakeSpi()

        display.command(0x15, 0x1C, 0x5B)

        self.assertEqual(
            display.lines.values,
            [(5, self.FakeLineValue.INACTIVE), (5, self.FakeLineValue.ACTIVE)],
        )
        self.assertEqual(display.spi.transfers, [[0x15], [0x1C, 0x5B]])

    def test_gpiochip_compatibility_symlink_is_deduplicated(self):
        aliases = {
            "/dev/gpiochip0": "/dev/gpiochip0",
            "/dev/gpiochip4": "/dev/gpiochip0",
        }
        with (
            mock.patch.object(oled.glob, "glob", return_value=list(aliases)),
            mock.patch.object(oled.os.path, "realpath", side_effect=aliases.__getitem__),
        ):
            selected = oled.Ssd1322._find_gpiochip(self.FakeGpiod)
        self.assertEqual(selected, "/dev/gpiochip0")
        self.assertEqual(self.FakeChip.opened, ["/dev/gpiochip0"])

    def test_distinct_admitted_gpiochips_are_refused(self):
        candidates = ["/dev/gpiochip0", "/dev/gpiochip1"]
        with (
            mock.patch.object(oled.glob, "glob", return_value=candidates),
            mock.patch.object(oled.os.path, "realpath", side_effect=lambda path: path),
        ):
            with self.assertRaisesRegex(RuntimeError, "expected one admitted Pi GPIO chip"):
                oled.Ssd1322._find_gpiochip(self.FakeGpiod)


if __name__ == "__main__":
    unittest.main()
