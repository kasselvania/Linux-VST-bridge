import hashlib
import struct
import unittest
from pigments_preset_identity import CLASS, MODULE, PREFIX, identify


def envelope(name='Test Patch', other=None):
    def chunk(label):
        encoded = label.encode('utf-8')
        return PREFIX + str(len(encoded)).encode() + b' ' + encoded + b' 4 Test '
    a, b = chunk(name), chunk(other or name)
    payload = struct.pack('<IIII', len(a), len(b), 4446, 3) + a + b + bytes(4446 * 16)
    header = b'LVBSTATE' + struct.pack('<II', 3, 104) + CLASS + MODULE
    header += struct.pack('<II', len(payload), 0) + hashlib.sha256(payload).digest()
    return header + payload


class IdentityTests(unittest.TestCase):
    def test_matching_length_framed_names(self):
        self.assertEqual(identify(envelope()), dict(name='Test Patch', bank='Test'))
        self.assertEqual(identify(envelope('Écho Pad'))['name'], 'Écho Pad')

    def test_component_controller_disagreement(self):
        with self.assertRaises(ValueError):
            identify(envelope(other='Other Patch'))

    def test_truncation_and_damage(self):
        b = envelope()
        for bad in [b[:100], b[:-1], b[:-1] + b'X']:
            with self.subTest(size=len(bad)), self.assertRaises(ValueError):
                identify(bad)

    def test_wrong_module_or_envelope(self):
        b = envelope()
        for index in [0, 8, 16, 32, 68]:
            bad = bytearray(b)
            bad[index] ^= 1
            with self.subTest(index=index), self.assertRaises(ValueError):
                identify(bad)

    def test_control_characters_refused(self):
        with self.assertRaises(ValueError):
            identify(envelope('Test\nPatch'))


if __name__ == '__main__':
    unittest.main()
