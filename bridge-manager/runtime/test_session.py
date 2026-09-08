"""Installed supervision tests. Real child processes; no vendor qualification."""
import hashlib
import json
import os
import pathlib
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import ownership
import session


@unittest.skipUnless(sys.platform == "linux", "PID/start tracking uses Linux procfs")
class OwnershipTests(unittest.TestCase):
    def test_cleanup_preserves_independent_sibling(self):
        first = subprocess.Popen(["/bin/sleep", "30"], start_new_session=True)
        sibling = subprocess.Popen(["/bin/sleep", "30"], start_new_session=True)
        try:
            census = ownership.process_identities()
            owned = [(p['pid'], p['start_ticks']) for p in census if p['pid'] == first.pid]
            self.assertEqual(len(owned), 1)
            self.assertTrue(all(ownership.cleanup_process(first, owned).values()))
            self.assertIsNone(sibling.poll())
            self.assertFalse(any(p['pid'] == first.pid for p in ownership.process_identities()))
        finally:
            for child in (first, sibling):
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=5)

    def test_terminal_sdk_failure_does_not_wait_for_launcher_companion(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            sid = "ab" * 16
            directory = root / "compatdata/pfx/drive_c/bridge/sessions" / sid
            directory.mkdir(parents=True, mode=0o700)
            artifact = root / "host"
            artifact.write_bytes(b"exact-test-image")
            binding = {'path': str(artifact), 'sha256': hashlib.sha256(artifact.read_bytes()).hexdigest()}
            spec = {'registration': {'host': binding, 'module': binding,
                'environment': {'root': str(root), 'runner': {'files': []}}},
                'session': sid, 'directory': str(directory), 'report': str(root / "report.json"), 'inspect': True}
            program = "import time; print('{\"event\":\"lifecycle\",\"state\":\"ap8_inspection_closed\",\"exit_code\":90}',flush=True); time.sleep(30)"
            started = time.monotonic()
            with patch.object(session, "command", return_value=([sys.executable, "-c", program], b"")), \
                 patch.object(session, "environment", return_value=os.environ.copy()):
                result = session.run(spec)
            self.assertLess(time.monotonic() - started, 4)
            self.assertEqual(result['error'], 'Windows SDK host failed: 90')
            self.assertIsNone(result['exit_before_cleanup'])
            self.assertEqual(result['raw_exit'], -signal.SIGTERM)
            self.assertTrue(result['cleanup_confirmed'])
            self.assertTrue(result['transport_retired'])
            self.assertFalse(directory.exists())
            self.assertTrue((root / "report.json").exists())

    def test_early_failure_wakes_native_and_retires_only_after_release(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            sid = "cd" * 16
            directory = root / "compatdata/pfx/drive_c/bridge/sessions" / sid
            directory.mkdir(parents=True, mode=0o700)
            (directory / "ap1.control").touch()
            artifact = root / "host"
            artifact.write_bytes(b"image")
            binding = {'path': str(artifact), 'sha256': hashlib.sha256(b"image").hexdigest()}
            spec = {'registration': {'host': binding, 'module': binding,
                'environment': {'root': str(root), 'runner': {'files': []}}},
                'session': sid, 'directory': str(directory), 'report': str(root / "report.json"),
                'inspect': False, 'binding_sent': True}
            native, owner = socket.socketpair()
            observations = []
            def consumer():
                native.settimeout(5)
                observations.append(native.recv(1))
                observations.append(directory.exists())
                native.shutdown(socket.SHUT_WR)
                observations.append(native.recv(1))
            thread = threading.Thread(target=consumer)
            thread.start()
            try:
                with patch.object(session, "command", return_value=([sys.executable, "-c", "raise SystemExit(90)"], b"")), \
                     patch.object(session, "environment", return_value=os.environ.copy()):
                    result = session.run(spec, owner)
                thread.join(timeout=6)
                self.assertFalse(thread.is_alive())
                self.assertEqual(observations, [b'F', True, b'R'])
                self.assertTrue(result['transport_retired'])
                self.assertFalse(directory.exists())
            finally:
                native.close()
                owner.close()


class CensusTests(unittest.TestCase):
    def test_stat_only_parsing_and_descendant_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for pid, parent, start in [(100, 1, 123), (101, 100, 124), (102, 101, 125), (103, 1, 126)]:
                d = root / str(pid)
                d.mkdir()
                fields = ['S', str(parent), '100', '100'] + ['0'] * 15 + [str(start)]
                (d / 'stat').write_text(f'{pid} (name with ) parentheses) ' + ' '.join(fields))
            records = ownership.process_identities(root)
            children = ownership.descendant_identities(100, records)
            self.assertEqual([(p['pid'], p['start_ticks']) for p in children], [(101, 124), (102, 125)])
            # No command-line/name files exist; routine tracking cannot depend on them.

    def test_changed_artifact_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / 'image'
            p.write_bytes(b'actual')
            with self.assertRaisesRegex(RuntimeError, 'changed'):
                session.verify({'path': str(p), 'sha256': '0' * 64})


if __name__ == '__main__':
    unittest.main()
