import fcntl
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import attended_vm as vm


class AdmissionTests(unittest.TestCase):
    def test_memory_reserve_includes_the_complete_container_ceiling(self):
        config = {"NanoCpus": 2_000_000_000, "Memory": 5 * vm.GIB,
                  "MemorySwap": 5 * vm.GIB, "RestartPolicy": {"Name": "no"}}
        with self.assertRaisesRegex(RuntimeError, "memory reserve"):
            vm.require_capacity(config, 9 * vm.GIB - 1)
        vm.require_capacity(config, 9 * vm.GIB)
        with self.assertRaisesRegex(RuntimeError, "no extra swap"):
            vm.require_capacity(dict(config, MemorySwap=6 * vm.GIB), 12 * vm.GIB)

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
             patch.object(vm.time, "sleep"), \
             patch.object(vm, "run") as command:
            vm.retire()
            self.assertEqual(down.call_count, 2)
            command.assert_not_called()

    def test_broken_log_pipe_does_not_abandon_shutdown_observation(self):
        with patch.object(vm, "inspect", side_effect=[
            {"State": {"Running": True}}, {"State": {"Running": False}}
        ]) as inspect, patch.object(vm, "powerdown") as down, \
             patch.object(vm.time, "sleep"), \
             patch("builtins.print", side_effect=BrokenPipeError):
            vm.retire()
            self.assertEqual(inspect.call_count, 2)
            down.assert_called_once()


if __name__ == "__main__":
    unittest.main()
