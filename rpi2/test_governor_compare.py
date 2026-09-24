import io
import json
import unittest
from unittest.mock import Mock
from governor_compare import complete_tail, drained


class GrowingReader(io.BytesIO):
    def read(self, count=-1):
        if count < 0: raise AssertionError('Live reader must use a frozen extent')
        position = self.tell()
        self.seek(0, 2)
        self.write(b'2}\n{"later":3}\n')
        self.seek(position)
        return super().read(count)


class TailTests(unittest.TestCase):
    def test_stale_low_wait_observations_are_not_a_drained_start(self):
        check=dict(blocks=180, unmatched=0, max_wait_ns=2_000_000, observation_age_ns=200_000_000)
        self.assertTrue(drained(check))
        self.assertFalse(drained(dict(check, observation_age_ns=1_000_000_001)))
        self.assertFalse(drained(dict(check, observation_age_ns=-1)))
        self.assertFalse(drained(dict(check, max_wait_ns=21_210_774)))
        self.assertFalse(drained(dict(check, unmatched=1)))

    def test_old_newline_filter_can_misparse_a_record_suffix(self):
        record = b'{"bridge_position":0,"phase":"worker_request_observed"}\n'
        chunks = [record[:1], record[1:]]
        # Model EOF observed after the writer's first byte, then its next write.
        with self.assertRaisesRegex(json.JSONDecodeError, 'Extra data'):
            [json.loads(line) for line in chunks if line.endswith(b'\n')]
        path = Mock()
        path.open.return_value = io.BytesIO(record)
        self.assertEqual(complete_tail(path), [json.loads(record)])

    def test_append_during_read_cannot_turn_partial_record_into_suffix(self):
        path = Mock()
        path.open.return_value = GrowingReader(b'{"complete":1}\n{"partial":')
        self.assertEqual(complete_tail(path), [{'complete': 1}])

    def test_window_discards_both_partial_edges(self):
        path = Mock()
        path.open.return_value = io.BytesIO(b'{"earlier":0}\n{"complete":1}\n{"partial":')
        self.assertEqual(complete_tail(path, 30), [{'complete': 1}])

    def test_malformed_complete_record_is_not_hidden(self):
        path = Mock()
        path.open.return_value = io.BytesIO(b'wrong\n')
        with self.assertRaises(ValueError): complete_tail(path)


if __name__ == '__main__': unittest.main()
