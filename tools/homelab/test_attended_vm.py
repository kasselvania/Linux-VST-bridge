import fcntl
import json
from pathlib import Path
import socket
import tempfile
import threading
import unittest
from unittest.mock import patch

import attended_vm as vm


class AdmissionTests(unittest.TestCase):
    def test_memory_reserve_includes_the_complete_container_ceiling(self):
        config = {"NanoCpus": 2_000_000_000, "Memory": 7 * vm.GIB,
                  "MemorySwap": 7 * vm.GIB, "RestartPolicy": {"Name": "no"}}
        with self.assertRaisesRegex(RuntimeError, "memory reserve"):
            vm.require_capacity(config, 11 * vm.GIB - 1)
        vm.require_capacity(config, 11 * vm.GIB)
        with self.assertRaisesRegex(RuntimeError, "no extra swap"):
            vm.require_capacity(dict(config, MemorySwap=8 * vm.GIB), 16 * vm.GIB)
        with self.assertRaisesRegex(RuntimeError, "7 GiB ceiling"):
            vm.require_capacity(dict(config, Memory=5 * vm.GIB, MemorySwap=5 * vm.GIB), 16 * vm.GIB)

    def test_guest_memory_and_retained_disk_are_required(self):
        args = ["qemu-system-x86_64", "-m", "6144",
                "-drive", "file=/vm/test224-data.qcow2,if=none,format=qcow2,id=lvb-test224-blank",
                "-device", "usb-storage,drive=lvb-test224-blank,id=lvbtest224disk,serial=LVB-TEST224-BLANK,bus=xhci.0"]
        vm.require_arguments(args)
        with self.assertRaisesRegex(RuntimeError, "6 GiB"):
            vm.require_arguments([("4096" if arg == "6144" else arg) for arg in args])
        with self.assertRaisesRegex(RuntimeError, "disk is not attached"):
            vm.require_arguments(args[:3])

    def test_a_killed_vm_is_never_reported_as_shut_down(self):
        self.assertIn("VM stopped", vm.stopped_notice({"Running": False, "ExitCode": 0, "OOMKilled": False}))
        self.assertIn("memory ceiling", vm.stopped_notice({"Running": False, "ExitCode": 137, "OOMKilled": True}))
        self.assertIn("exit code 1", vm.stopped_notice({"Running": False, "ExitCode": 1, "OOMKilled": False}))

    def test_either_existing_lock_prevents_even_the_workload_census(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("heavy-build.lock", "vm.lock"):
                (root / name).touch()
            for name in ("heavy-build.lock", "vm.lock"):
                with (root / name).open("r+") as owner:
                    fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    with patch.object(vm, "BASE", root), \
                         patch.object(vm, "run", return_value="/mnt/Storage2") as run, \
                         patch.object(vm, "require_idle") as idle:
                        with self.assertRaises(BlockingIOError):
                            vm.main()
                        idle.assert_not_called()
                        self.assertEqual(run.call_count, 1)  # Only findmnt.

    def test_shutdown_error_does_not_force_stop_or_restore_a_container(self):
        running = {"State": {"Running": True}}
        stopped = {"State": {"Running": False}}
        with patch.object(vm, "inspect", side_effect=[running, running, stopped]), \
             patch.object(vm, "powerdown", side_effect=[OSError("not ready"), None]) as down, \
             patch.object(vm, "prepare_qmp"), \
             patch.object(vm.time, "sleep"), \
             patch.object(vm, "run") as command:
            vm.retire()
            self.assertEqual(down.call_count, 2)
            command.assert_not_called()

    def test_unowned_control_socket_is_prepared_again_before_shutdown_retry(self):
        running = {"State": {"Running": True}}
        stopped = {"State": {"Running": False}}
        calls = []

        def powerdown():
            calls.append("powerdown")
            if calls.count("powerdown") == 1:
                raise PermissionError("root-owned control socket")

        with patch.object(vm, "inspect", side_effect=[running, running, stopped]), \
             patch.object(vm, "powerdown", side_effect=powerdown), \
             patch.object(vm, "prepare_qmp", side_effect=lambda: calls.append("prepare")), \
             patch.object(vm.time, "sleep"):
            vm.retire()
        self.assertEqual(calls, ["powerdown", "prepare", "powerdown"])

    def test_broken_log_pipe_does_not_abandon_shutdown_observation(self):
        with patch.object(vm, "inspect", side_effect=[
            {"State": {"Running": True}}, {"State": {"Running": False}}
        ]) as inspect, patch.object(vm, "powerdown") as down, \
             patch.object(vm.time, "sleep"), \
             patch("builtins.print", side_effect=BrokenPipeError):
            vm.retire()
            self.assertEqual(inspect.call_count, 2)
            down.assert_called_once()

    def test_prepare_control_socket_requires_a_live_qmp_greeting(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "qmp.sock"
            with socket.socket(socket.AF_UNIX) as server:
                server.bind(str(path))
                server.listen()
                server.settimeout(3)

                def greet():
                    with server.accept()[0] as client:
                        client.sendall((json.dumps({"QMP": {"capabilities": []}}) + "\n").encode())

                worker = threading.Thread(target=greet)
                worker.start()
                try:
                    with patch.object(vm, "QMP", path), patch.object(vm, "run") as run:
                        vm.prepare_qmp()
                        self.assertEqual(run.call_args.args[:5],
                                         ("docker", "exec", vm.VM, "chown", "--no-dereference"))
                finally:
                    worker.join(3)
                self.assertFalse(worker.is_alive())


if __name__ == "__main__":
    unittest.main()
