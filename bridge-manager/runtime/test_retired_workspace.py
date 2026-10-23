"""Retired launch plans cannot adopt, stop or rewrite an old FL Studio workspace."""
import pathlib
import tempfile
import unittest
from unittest.mock import patch

import ownership
import session
from test_session import CgroupFixture


class RetiredWorkspace(unittest.TestCase):
    def test_retired_launch_and_uninstall_specs_refuse_before_any_process_or_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            report = root / "result.json"
            report.write_bytes(b'{"cleanup_confirmed":false,"owned_live":1}')
            project = root / "song.flp"
            project.write_bytes(b"retained project fixture")
            before = {p.name: p.read_bytes() for p in root.iterdir()}
            for kind in ("daw_workspace_application", "daw_workspace_uninstall"):
                with self.subTest(kind=kind), \
                     patch.object(session.subprocess, "Popen") as launch, \
                     patch.object(session, "CompanionCgroup") as scope:
                    with self.assertRaisesRegex(RuntimeError, "application owner kind unsupported"):
                        session.vendor_application({"kind": kind, "report": str(report)})
                    launch.assert_not_called()
                    scope.assert_not_called()
                self.assertEqual({p.name: p.read_bytes() for p in root.iterdir()}, before)

    def test_current_vendor_owner_cannot_adopt_a_retired_workspace_unit(self):
        with tempfile.TemporaryDirectory() as tmp:
            class OldWorkspace(CgroupFixture):
                group = "/user.slice/linux-vst-bridge-daw-fl-" + "ab" * 16 + ".service"

            fixture = OldWorkspace(pathlib.Path(tmp))
            fixture.record(101, 1001, 1)
            fixture.set_members([101])
            members = (fixture.unit / "cgroup.procs").read_bytes()
            for selector in (None, "cd" * 32):
                with self.subTest(selector=selector), \
                     self.assertRaisesRegex(RuntimeError, "exact dedicated cgroup"):
                    ownership.CompanionCgroup(proc_root=fixture.proc, cgroup_root=fixture.cg,
                        supervisor=fixture.supervisor, vendor_application=selector)
            self.assertEqual((fixture.unit / "cgroup.procs").read_bytes(), members)


if __name__ == "__main__":
    unittest.main()
