import os
from pathlib import Path
import tempfile
import unittest
from isolation import compositor_environment, runner_environment


class IsolationTests(unittest.TestCase):
    def test_no_ambient_display_or_authorization(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d).resolve()
            env = compositor_environment(root)
            self.assertNotIn('DISPLAY', env)
            self.assertNotIn('WAYLAND_DISPLAY', env)
            self.assertNotIn('DBUS_SESSION_BUS_ADDRESS', env)
            self.assertNotEqual(env['HOME'], os.environ['HOME'])
            for k in ('HOME', 'XDG_RUNTIME_DIR', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME'):
                self.assertEqual(Path(env[k]).stat().st_mode & 0o777, 0o700)
            for patch in ({}, {'WAYLAND_DISPLAY': 'wayland-0'}, {'XDG_RUNTIME_DIR': '/run/user/1000'}):
                bad = dict(env, DISPLAY=':99', WAYLAND_DISPLAY='uir1-isolated', DBUS_SESSION_BUS_ADDRESS='unix:path=private')
                bad.update(patch)
                if patch:
                    with self.assertRaises(RuntimeError): runner_environment(root, bad)
            env.update(DISPLAY=':99', WAYLAND_DISPLAY='uir1-isolated', DBUS_SESSION_BUS_ADDRESS='unix:path=private')
            r = runner_environment(root, env)
            self.assertEqual(r['STEAM_COMPAT_DATA_PATH'], str(root / 'compatdata'))
            self.assertNotIn('LVB_EVENT_OUTPUT_POLICY', r)

    def test_foreign_authority_refused(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d).resolve(); env = compositor_environment(root)
            env.update(DISPLAY=':99', WAYLAND_DISPLAY='uir1-isolated', DBUS_SESSION_BUS_ADDRESS='unix:path=private', XAUTHORITY='/tmp/operator-auth')
            with self.assertRaises(RuntimeError): runner_environment(root, env)

if __name__ == '__main__': unittest.main()
