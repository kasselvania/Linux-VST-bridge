import unittest
from unittest.mock import patch
import ap8_preview


class PreviewEnvironment(unittest.TestCase):
    def test_accessibility_is_explicit_and_instance_scoped(self):
        base = {'WINEDLLOVERRIDES': 'winemenubuilder.exe=d', 'WINEPREFIX': 'owned'}
        desktop = {'DISPLAY': ':2'}
        with patch.object(ap8_preview.owner.runtime, 'controlled_environment', return_value=base):
            normal = ap8_preview.launch_environment(object(), desktop)
            selected = ap8_preview.launch_environment(object(), desktop,
                trace_delivery=True, disable_windows_accessibility=True)
            sibling = ap8_preview.launch_environment(object(), desktop)
        self.assertEqual(normal, {**base, **desktop})
        self.assertEqual(sibling, normal)
        self.assertEqual(selected['WINEDLLOVERRIDES'],
                         'winemenubuilder.exe=d;uiautomationcore=')
        self.assertEqual(selected['LVB_AP10_TRACE'], '1')
        self.assertEqual(base, {'WINEDLLOVERRIDES': 'winemenubuilder.exe=d', 'WINEPREFIX': 'owned'})
        self.assertEqual(desktop, {'DISPLAY': ':2'})


if __name__ == '__main__':
    unittest.main()
