import unittest
from unittest.mock import patch
import governor_guard as guard


class GuardTests(unittest.TestCase):
    def test_only_fixed_order_is_accepted(self):
        class Fake:
            def __init__(self): self.writes = []
            def set(self, path, value): self.writes.append((path, value))
        settings = Fake()
        for index, command in enumerate(guard.TRANSITIONS):
            self.assertEqual(guard.transition(index, command, settings), index + 1)
        self.assertEqual(settings.writes, [(guard.GOVERNOR, 'performance'),
                                           (guard.GOVERNOR, 'ondemand')])
        for index, command in ((0, 'ondemand'), (1, 'performance'), (2, 'sh'),
                               (3, 'finish'), (0, 'performance; reboot')):
            with self.assertRaises(ValueError):
                guard.transition(index, command, settings)

    def test_restore_includes_failed_write_and_attempts_both_settings(self):
        values = {guard.GOVERNOR: 'ondemand', guard.SCHEDSTATS: '0'}
        with patch.object(guard, 'read_setting', side_effect=values.__getitem__):
            settings = guard.Settings()
        with patch.object(guard, 'write_setting', side_effect=OSError('refused')):
            with self.assertRaises(OSError): settings.set(guard.GOVERNOR, 'performance')
            with self.assertRaises(OSError): settings.set(guard.SCHEDSTATS, '1')
        with patch.object(guard, 'write_setting', side_effect=[OSError('refused'), None]) as write:
            errors = settings.restore()
        self.assertEqual(write.call_count, 2)
        self.assertEqual(len(errors), 1)
        self.assertEqual(write.call_args_list[-1].args, (guard.GOVERNOR, 'ondemand'))

    def test_refuses_different_starting_governor(self):
        with patch.object(guard, 'read_setting', side_effect=['performance', '0']):
            with self.assertRaises(RuntimeError): guard.Settings()

    def test_successful_cleanup_does_not_mean_completed_comparison(self):
        state = dict(phase='restored', restoration_errors=[],
                     stop_reason='finished', transition_count=3)
        self.assertEqual(guard.exit_status(state), 0)
        for reason in ('hard_timeout', 'signal_2', 'signal_14',
                       'error: Malformed phase request',
                       'error: Unexpected phase command'):
            with self.subTest(reason=reason):
                self.assertEqual(guard.exit_status(dict(state, stop_reason=reason)), 2)
        self.assertEqual(guard.exit_status(dict(state, transition_count=2)), 2)
        self.assertEqual(guard.exit_status(dict(state, phase='restore_failed',
                                               restoration_errors=['readback mismatch'])), 1)


if __name__ == '__main__':
    unittest.main()
