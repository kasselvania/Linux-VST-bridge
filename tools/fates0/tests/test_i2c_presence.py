import importlib.util
import pathlib
import unittest


ROOT = pathlib.Path(__file__).parents[1]
SPEC = importlib.util.spec_from_file_location("fates0_i2c_presence", ROOT / "i2c_presence.py")
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class FakeBus:
    def __init__(self, number):
        self.number = number
        self.address = None
        self.closed = False

    def __enter__(self):
        return self

    def write_quick(self, address):
        self.address = address

    def __exit__(self, *_args):
        self.closed = True


class I2cPresenceTest(unittest.TestCase):
    def test_probe_is_one_bounded_quick_transaction_at_wm8731_address(self):
        created = []

        def factory(number):
            bus = FakeBus(number)
            created.append(bus)
            return bus

        MODULE.probe_codec(1, 0x1A, factory)
        self.assertEqual(len(created), 1)
        self.assertEqual(created[0].number, 1)
        self.assertEqual(created[0].address, 0x1A)
        self.assertTrue(created[0].closed)


if __name__ == "__main__":
    unittest.main()
