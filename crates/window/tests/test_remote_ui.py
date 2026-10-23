"""Process/capture cleanup controls without starting a native application."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

import remote_ui as ui


class RemoteLifecycle(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="bridge-remote-harness-")
        self.addCleanup(directory.cleanup)
        self.addCleanup(patch.stopall)
        patch.object(ui, "ARTIFACTS", Path(directory.name)).start()
        self.app = ui.App("fixture")

    def test_normal_quit_does_not_require_another_framebuffer_capture(self):
        self.app.process = Mock(returncode=0)
        self.app.process.poll.return_value = None
        self.app.port = 1

        def request(route, **_parameters):
            if route != "/quit":
                raise AssertionError("Framebuffer is unavailable during teardown")
            return {"ok": True}

        self.app.request = Mock(side_effect=request)
        self.app.close()
        self.app.request.assert_called_once_with("/quit")
        self.app.process.wait.assert_called_once_with(timeout=10)
        self.app.process.terminate.assert_not_called()
        self.app.process.kill.assert_not_called()

    def test_forced_cleanup_cannot_pass_as_a_normal_quit(self):
        self.app.process = Mock(returncode=-15)
        self.app.process.poll.return_value = None
        self.app.process.wait.side_effect = [subprocess.TimeoutExpired("app", 10), -15]
        self.app.port = 1
        self.app.request = Mock(return_value={"ok": True})
        with self.assertRaisesRegex(RuntimeError, "did not exit after"):
            self.app.close()
        self.app.process.terminate.assert_called_once_with()

    def test_startup_failure_survives_cleanup_failure(self):
        original = AssertionError("Progress revision never drew")
        self.app.before_remote = Mock(side_effect=original)
        self.app.close = Mock(side_effect=RuntimeError("Quit failed"))
        with patch.object(ui.subprocess, "Popen"), patch.object(ui, "wait_until", return_value=1):
            with self.assertRaises(AssertionError) as caught:
                self.app.__enter__()
        self.assertIs(caught.exception, original)
        self.assertIn("Cleanup: Quit failed", original.__notes__)

    def test_body_failure_survives_cleanup_failure(self):
        original = AssertionError("Capture failed")
        self.app.close = Mock(side_effect=RuntimeError("Quit failed"))
        self.assertIsNone(self.app.__exit__(type(original), original, None))
        self.assertIn("Cleanup: Quit failed", original.__notes__)

    def test_capture_bytes_are_retained_with_the_uploaded_logs(self):
        source = self.app.directory.parent / "remote.png"
        source.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
        self.app.request = Mock(return_value={"png": str(source), "sz": [640, 400]})
        captured = Path(self.app.capture("progress"))
        self.assertEqual(captured.parent, self.app.directory)
        self.assertEqual(captured.read_bytes(), source.read_bytes())
        metadata = json.loads((self.app.directory / "progress.json").read_text())
        self.assertEqual(metadata["png"], str(captured))


if __name__ == "__main__":
    unittest.main(verbosity=2)
