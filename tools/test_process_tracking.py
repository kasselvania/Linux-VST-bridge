"""Identity-only tracking, exit detection and positive-cleanup regressions."""
import contextlib
import pathlib
import tempfile
import types
import unittest
from unittest.mock import MagicMock, patch

import ap4_preview as preview

tracking = preview.runtime


def identity(pid, parent, start, group=100, session=100):
    return dict(pid=pid, ppid=parent, pgrp=group, session=session, start_ticks=start)


def stat(record, comm="worker"):
    fields = ["S"] + ["0"] * 19
    for index, key in [(1, "ppid"), (2, "pgrp"), (3, "session"), (19, "start_ticks")]:
        fields[index] = str(record[key])
    return f"{record['pid']} ({comm}) " + " ".join(fields)


class TrackingTests(unittest.TestCase):
    def test_only_stat_is_needed_and_pid_start_identity_is_fresh(self):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp)
            for pid in [100, 101, 102]:
                (root / str(pid)).mkdir()
            first = identity(100, 1, 1234)
            (root / '100/stat').write_text(stat(first, 'worker ) with spaces'))
            (root / '101/stat').write_text('malformed')
            # 102 disappeared between directory enumeration and stat open.
            self.assertEqual(tracking.process_identities(root), [first])
            reused = identity(100, 20, 5678, 20, 20)
            (root / '100/stat').write_text(stat(reused))
            self.assertEqual(tracking.process_identities(root), [reused])
            (root / '100/stat').write_text(stat(identity(999, 1, 1234)))
            self.assertEqual(tracking.process_identities(root), [])

    def test_descendants_include_new_grandchildren_and_exclude_siblings(self):
        a, b, sibling = identity(101, 100, 1), identity(102, 101, 2), identity(201, 200, 3)
        with patch.object(tracking, 'process_identities', side_effect=[[sibling, a], [b, sibling, a]]):
            self.assertEqual(tracking.descendant_identities(100), [a])
            self.assertEqual(tracking.descendant_identities(100), [a, b])

    def test_unexpected_root_exit_keeps_all_observed_pid_start_pairs_for_cleanup(self):
        d = preview.runtime
        with tempfile.TemporaryDirectory() as temp, contextlib.ExitStack() as stack:
            env = types.SimpleNamespace(session=pathlib.Path(temp), run_id='a'*32, marker={'fixture':'again'})
            root = MagicMock(pid=100, returncode=23)
            root.poll.side_effect = [None, None, 23]
            selector = MagicMock(); selector.get_map.return_value = {}
            for name, value in {'verify_environment':None, 'verify_diagnostic_runner':{'launch_critical_manifest_sha256':'c'*64},
                'handshake':b'ready', 'command_vector':['inert'], 'controlled_environment':{},
                'process_identity':identity(100,1,1), 'protected_snapshot':{}}.items():
                stack.enter_context(patch.object(d, name, return_value=value))
            # A reused PID is a distinct observation; prior identities are retained.
            collect = stack.enter_context(patch.object(d, 'descendant_identities', side_effect=[
                [identity(101,100,2)], [identity(102,101,3)], [identity(101,100,4)]]))
            full = stack.enter_context(patch.object(d, 'descendants', side_effect=AssertionError('metadata census on playback loop')))
            cleanup = stack.enter_context(patch.object(d, 'cleanup_process', return_value={'owned_descendants_zero':True,'process_group_empty':True}))
            stack.enter_context(patch.object(d.subprocess, 'Popen', return_value=root))
            stack.enter_context(patch.object(d.selectors, 'DefaultSelector', return_value=selector))
            pump = stack.enter_context(patch.object(d, 'pump'))
            result = d.supervise(env, mode=d.PC0_MODE, session_override='b'*32,
                                 post_gate_seconds=None, stop_requested=lambda:False)
            self.assertEqual(result['raw_exit'], 23)
            self.assertNotEqual(result['classification'], 'scanner_completed')
            self.assertEqual(collect.call_count, 3)
            self.assertEqual(pump.call_count, 3)
            self.assertTrue(all(call.args[2] == d.POLL_SECONDS for call in pump.call_args_list))
            full.assert_not_called()
            cleanup.assert_called_once_with(root, [(100,1),(101,2),(101,4),(102,3)])

    def test_cleanup_does_not_signal_reused_pid_or_healthy_sibling(self):
        d = preview.runtime
        root = MagicMock(pid=100); root.poll.return_value = 0
        other = [identity(101,200,99,200,200)]
        with patch.object(d, 'process_identities', return_value=other), patch.object(d.os, 'killpg') as kill:
            result = d.cleanup_process(root, [(100,1),(101,2)])
        kill.assert_not_called()
        self.assertEqual(result, {'owned_descendants_zero':True,'process_group_empty':True})

    def test_cleanup_targets_only_owned_group_and_requires_positive_absence(self):
        d = preview.runtime
        root = MagicMock(pid=100); root.poll.side_effect = [None,0,0]
        child, sibling = identity(101,100,2), identity(201,200,3,200,200)
        with patch.object(d, 'process_identities', side_effect=[[child,sibling],[sibling],[sibling]]), patch.object(d.os, 'killpg') as kill:
            self.assertTrue(d.cleanup_process(root, [(100,1),(101,2)])['owned_descendants_zero'])
        kill.assert_called_once_with(100,d.signal.SIGTERM)
        # A known descendant escaping its original group still prevents success.
        escaped = identity(101,1,2,300,300); root.poll.return_value=0; root.poll.side_effect=None
        with patch.object(d, 'process_identities', return_value=[escaped,sibling]), patch.object(d.os, 'killpg'), patch.object(d, 'CLEANUP_SECONDS', 3):
            with self.assertRaisesRegex(RuntimeError, 'owned descendants survived cleanup'):
                d.cleanup_process(root, [(100,1),(101,2)])


if __name__ == '__main__':
    unittest.main()
