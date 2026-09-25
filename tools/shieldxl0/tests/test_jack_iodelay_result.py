import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).parents[1] / "jack_iodelay_result.py"
SPEC = importlib.util.spec_from_file_location("shieldxl0_jack_iodelay_result", MODULE_PATH)
parser = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = parser
SPEC.loader.exec_module(parser)


class JackIoDelayResultTest(unittest.TestCase):
    def test_parses_real_jack_iodelay_output_shape(self):
        values = parser.measurements(
            "  2069.697 frames     43.119 ms total roundtrip latency\n"
            "\textra loopback latency: 21 frames\n"
        )
        self.assertEqual(values, [(2069.697, 43.119)])

    def test_rejects_signal_below_threshold_without_measurement(self):
        self.assertEqual(parser.measurements("Signal below threshold...\n"), [])

    def test_accepts_multiple_measurements_and_preserves_order(self):
        values = parser.measurements(
            "  1045.250 frames 21.776 ms total roundtrip latency\n"
            "  1045.500 frames 21.781 ms total roundtrip latency\n"
        )
        self.assertEqual(values, [(1045.25, 21.776), (1045.5, 21.781)])


if __name__ == "__main__":
    unittest.main()
