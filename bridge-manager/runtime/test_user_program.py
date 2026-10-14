"""User-selected programs retain their own bytes and normal window lifecycle."""
import hashlib, json, os, pathlib, signal, subprocess, sys, tempfile, unittest
from unittest.mock import Mock, patch
import session as s


def selected(root, image):
    return {'schema': 3, 'id': 'selected', 'helpers': [],
            'executable': {'path': str(image), 'sha256': hashlib.sha256(image.read_bytes()).hexdigest()},
            'user_selection': {'working_directory': str(image.parent), 'arguments': [], 'previous': None},
            'environment': {'id': 'ab' * 16, 'root': str(root),
                            'runner': {'entry_point': '/runner/entry', 'proton': '/runner/proton', 'files': []}}}


class UserProgram(unittest.TestCase):
    def test_external_image_uses_selected_runtime_folder_and_exact_adapter(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = pathlib.Path(tmp).resolve(); root = folder / 'managed'; root.mkdir()
            image = folder / 'Portable program.exe'; image.write_bytes(b'MZ owned program')
            app = selected(root, image)
            request = root / 'program-launch.private'; launcher = {'path': '/selected/launch.exe'}
            argv, cwd = s.vendor_launch({'application': app, 'mode': 'normal', 'launcher': launcher}, request)
            self.assertEqual(argv[:5], ['/runner/entry', '--verb=run', '--', '/runner/proton', 'runinprefix'])
            self.assertEqual(argv[5], launcher['path'])
            self.assertEqual(argv[6:], [s.windows(request, root / 'compatdata/pfx')])
            self.assertEqual(cwd, folder)
            self.assertEqual(s.vendor_compatibility('normal', app), {'disable_windows_accessibility': False})
            with self.assertRaises(RuntimeError): s.vendor_compatibility('agent_probe', app)
            app['user_selection']['arguments'] = ['two words', '$(no shell)']
            with self.assertRaisesRegex(RuntimeError, 'arguments are not implemented'):
                s.vendor_launch({'application': app, 'mode': 'normal', 'launcher': launcher}, request)

    def test_atomic_replacement_keeps_old_mapped_process_identified_and_rejects_new_mapping(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = pathlib.Path(tmp).resolve(); image = folder / 'Portable.exe'; image.write_bytes(b'MZ build A')
            app = selected(folder, image); process = folder / 'proc' / '42'; process.mkdir(parents=True)
            scope = Mock(); scope.proc_root = folder / 'proc'; scope.identity.return_value = None
            retained = s.RendererImage({'artifact': app['executable'], 'size': image.stat().st_size})
            def mapped(metadata):
                (process / 'maps').write_text(f'1000-2000 r-xp 0000 {os.major(metadata.st_dev):x}:{os.minor(metadata.st_dev):x} {metadata.st_ino} {image}\n')
            try:
                mapped(image.stat()); replacement = folder / 'replacement'; replacement.write_bytes(b'MZ build B')
                replacement.replace(image)
                row = s.vendor_process_metadata(scope, {'pid': 42, 'ppid': 1}, app, selected_image=retained)
                self.assertEqual(row['registered_images'], [{'role': 'main', **app['executable']}])
                mapped(image.stat())
                row = s.vendor_process_metadata(scope, {'pid': 42, 'ppid': 1}, app, selected_image=retained)
                self.assertEqual(row['registered_images'], [], 'Replacement bytes cannot be called build A')
                with self.assertRaises(ValueError): retained.check()
            finally: retained.close()

    def test_normal_close_reaches_exact_owner_without_force_cleanup_or_installer_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp).resolve(); image = root / 'Portable.exe'; image.write_bytes(b'MZ owned program')
            app = selected(root, image); operation = 'cd' * 16; request = 'ef' * 16
            adapter = root / 'launch.exe'; adapter.write_bytes(b'MZ exact application adapter')
            launcher = {'path': str(adapter), 'sha256': hashlib.sha256(adapter.read_bytes()).hexdigest()}
            close_file = root / 'close.json'; close_file.write_text(json.dumps({'operation_id': operation, 'request': request})); close_file.chmod(0o600)
            scope = Mock(); scope.members.return_value = []; scope.group = '/owned-program'
            library = Mock(); library.prctl.return_value = 0
            handlers = [signal.getsignal(sig) for sig in (signal.SIGTERM, signal.SIGINT)]
            try:
                with patch.object(s, 'CompanionCgroup', return_value=scope) as owner, \
                     patch.object(s.ctypes, 'CDLL', return_value=library), \
                     patch.object(s.os, 'waitid', return_value=None), \
                     patch.object(s, 'vendor_launch', return_value=([sys.executable, '-c', 'import time;time.sleep(.1)'], root)), \
                     patch.object(s, 'renderer_launch_software', return_value={'installer_launch': launcher}), \
                     patch.object(s, 'environment', return_value=os.environ.copy()), \
                     patch.object(s, 'vendor_focus', return_value='close_requested') as close:
                    self.assertTrue(s.vendor_application({'application': app, 'report': str(root / 'result.json'), 'operation_id': operation, 'mode': 'normal', 'launcher': launcher}))
                binding = {'environment': app['environment']['id'], 'executable': app['executable'], 'user_selection': app['user_selection']}
                selector = hashlib.sha256(json.dumps(binding, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
                owner.assert_called_once_with(vendor_application=selector)
                self.assertTrue(close.call_args.kwargs['close'])
                self.assertEqual(close.call_args.kwargs['selected_image'].path, image)
                scope.cleanup.assert_not_called()
                report = json.loads((root / 'result.json').read_text())
                self.assertEqual(report['close_result'], {'request': request, 'result': 'close_requested'})
                self.assertEqual(report['state'], 'completed'); self.assertTrue(report['cleanup_confirmed'])
                self.assertFalse(report['windows_accessibility_disabled'])
            finally:
                for sig, handler in zip((signal.SIGTERM, signal.SIGINT), handlers): signal.signal(sig, handler)


if __name__ == '__main__': unittest.main()
