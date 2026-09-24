import unittest
from multicore_readback import preference, disabled_preference


class PreferenceEdit(unittest.TestCase):
    def test_only_selected_value_bytes_change(self):
        before=b'<rootnode>\n <param name="Unrelated" value="1.000000"/>\n <param name="Multicore2" value="1.000000"/>\n</rootnode>'
        expected=before.replace(b'name="Multicore2" value="1.000000"',b'name="Multicore2" value="0.000000"')
        self.assertEqual(disabled_preference(before),expected)
        self.assertEqual(preference(expected),0)

    def test_missing_duplicate_or_nonboolean_setting_refused(self):
        for body in [b'',b'<param name="Multicore2" value="2"/>',
                     b'<param name="Multicore2" value="1"/><param name="Multicore2" value="1"/>']:
            with self.subTest(body=body), self.assertRaises(ValueError):
                disabled_preference(b'<rootnode>'+body+b'</rootnode>')


if __name__=='__main__':unittest.main()
