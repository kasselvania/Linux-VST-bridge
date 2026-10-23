import importlib.util
import pathlib
import types
import unittest


ROOT = pathlib.Path(__file__).parents[1]
SPEC = importlib.util.spec_from_file_location("shieldxl0_i2c_presence", ROOT / "i2c_presence.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FakeContext:
    def __init__(self, events, name):
        self.events = events
        self.name = name

    def __enter__(self):
        self.events.append(f"{self.name}-enter")
        return self

    def __exit__(self, *_args):
        self.events.append(f"{self.name}-exit")


class I2cPresenceTest(unittest.TestCase):
    def test_reset_is_deasserted_and_held_while_bounded_probe_runs(self):
        events = []
        captured = {}

        class FakeLineSettings:
            def __init__(self, **values):
                captured["settings"] = values

        def request_lines(chip, *, consumer, config):
            captured.update(chip=chip, consumer=consumer, config=config)
            return FakeContext(events, "gpio")

        class FakeBus:
            def __init__(self, number):
                captured["bus"] = number

            def __enter__(self):
                events.append("bus-enter")
                return self

            def write_quick(self, address):
                captured["address"] = address
                events.append("write-quick")

            def __exit__(self, *_args):
                events.append("bus-exit")

        gpio = types.SimpleNamespace(
            LineSettings=FakeLineSettings,
            line=types.SimpleNamespace(
                Direction=types.SimpleNamespace(OUTPUT="output"),
                Value=types.SimpleNamespace(ACTIVE="active"),
            ),
            request_lines=request_lines,
        )

        def sleep(seconds):
            captured["sleep"] = seconds
            events.append("sleep")

        MODULE.probe_codec(1, 0x48, gpio, FakeBus, sleep)

        self.assertEqual(captured["chip"], "/dev/gpiochip0")
        self.assertEqual(captured["consumer"], "shieldxl0-i2c-admission")
        self.assertEqual(list(captured["config"]), [17])
        self.assertEqual(
            captured["settings"],
            {"direction": "output", "output_value": "active"},
        )
        self.assertEqual(captured["bus"], 1)
        self.assertEqual(captured["address"], 0x48)
        self.assertEqual(captured["sleep"], 0.01)
        self.assertEqual(
            events,
            ["gpio-enter", "sleep", "bus-enter", "write-quick", "bus-exit", "gpio-exit"],
        )


if __name__ == "__main__":
    unittest.main()
