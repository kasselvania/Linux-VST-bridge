"""WD0 changed-owner tests: exact DAW cgroup and graceful GUI close request."""
import hashlib
import json
import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

import ownership
import session
from test_session import CgroupFixture


class DawWorkspaceRuntime(unittest.TestCase):
    def test_exact_daw_unit_cannot_impersonate_vendor_or_other_operation(self):
        with tempfile.TemporaryDirectory() as tmp:
            op = "ab" * 16

            class DawCgroup(CgroupFixture):
                group = "/user.slice/linux-vst-bridge-daw-fl-" + op + ".service"

            fixture = DawCgroup(pathlib.Path(tmp))
            scope = ownership.CompanionCgroup(
                proc_root=fixture.proc, cgroup_root=fixture.cg,
                supervisor=fixture.supervisor, daw_operation=op)
            self.assertEqual(scope.group, fixture.group)
            for wrong in ["cd" * 16, "not-an-operation", "a" * 31]:
                with self.assertRaises(RuntimeError):
                    ownership.CompanionCgroup(
                        proc_root=fixture.proc, cgroup_root=fixture.cg,
                        supervisor=fixture.supervisor, daw_operation=wrong)
            with self.assertRaises(RuntimeError):
                ownership.CompanionCgroup(
                    proc_root=fixture.proc, cgroup_root=fixture.cg,
                    supervisor=fixture.supervisor, daw_operation=op,
                    installer_operation=op)

    def test_daw_close_is_requested_without_forcing_the_cohort(self):
        with tempfile.TemporaryDirectory() as tmp:
            workspace = pathlib.Path(tmp).resolve() / "fl-studio"
            root = workspace / "environment"
            root.mkdir(parents=True, mode=0o700)
            (root / "operation.lock").touch()
            (root / "home").mkdir(mode=0o700)
            for name in ("preferences", "projects", "exports"):
                (workspace / name).mkdir(mode=0o700)
            image = root / "FL64.exe"
            image.write_bytes(b"source-owned fixture")
            artifact = {"path": str(image), "sha256": hashlib.sha256(image.read_bytes()).hexdigest()}
            op = "ab" * 16
            request = "cd" * 16
            close_file = root / "close.json"
            close_file.write_text(json.dumps({"operation_id": op, "request": request}))
            close_file.chmod(0o600)
            app = {"id": "fl_studio", "executable": artifact, "helpers": [],
                   **{name: str(workspace / name) for name in ("preferences", "projects", "exports")},
                   "environment": {"root": str(root), "runner": {"files": []}}}
            scope = Mock()
            scope.members.return_value = []
            scope.group = "/test-owned-daw"
            library = Mock()
            library.prctl.return_value = 0
            original_popen = subprocess.Popen
            launches = []

            def launched(argv, **kwargs):
                launches.append(kwargs["env"])
                return original_popen(
                    [sys.executable, "-c", "import time;time.sleep(.25)"],
                    stdin=kwargs["stdin"], stdout=kwargs["stdout"],
                    stderr=kwargs["stderr"], start_new_session=True, bufsize=0)

            handlers = [signal.getsignal(x) for x in (signal.SIGTERM, signal.SIGINT)]
            try:
                with patch.object(session, "CompanionCgroup", return_value=scope) as constructor, \
                     patch.object(session.ctypes, "CDLL", return_value=library), \
                     patch.object(session.os, "waitid", return_value=None), \
                     patch.object(session, "vendor_launch", return_value=(["fixed"], root)), \
                     patch.object(session, "environment", return_value={"DISPLAY": ":0"}), \
                     patch.object(session.subprocess, "Popen", side_effect=launched), \
                     patch.object(session, "vendor_focus", return_value="close_requested") as close:
                    self.assertTrue(session.vendor_application({
                        "kind": "daw_workspace_application", "application": app,
                        "report": str(root / "result.json"), "operation_id": op,
                        "mode": "normal"}))
                constructor.assert_called_once_with(daw_operation=op)
                close.assert_called_once_with(scope, app, close=True)
                report = json.loads((root / "result.json").read_text())
                self.assertEqual(report["close_result"], {"request": request, "result": "close_requested"})
                self.assertEqual(report["state"], "completed")
                self.assertTrue(report["cleanup_confirmed"])
                self.assertEqual(report["owned_live"], 0)
                self.assertEqual(launches[0]["HOME"], str(root / "home"))
                self.assertEqual(launches[0]["XDG_CONFIG_HOME"], str(workspace / "preferences"))
                self.assertNotIn("WINEDLLOVERRIDES", launches[0])
                self.assertNotIn("close.json", [p.name for p in root.iterdir()])
            finally:
                for sig, handler in zip((signal.SIGTERM, signal.SIGINT), handlers):
                    signal.signal(sig, handler)


if __name__ == "__main__":
    unittest.main()
