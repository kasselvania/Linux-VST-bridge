import copy
import json
import pathlib
import unittest
from check_fixture import check


class FixtureCustody(unittest.TestCase):
    def setUp(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        self.cases = json.loads((root/'evidence/lc2/generated.json').read_text())['cases']

    def test_actual_retained_cases(self):
        for case in self.cases:
            self.assertEqual(check(case), case['result'])

    def test_exit_without_cleanup_never_passes(self):
        for case in copy.deepcopy(self.cases):
            case['cohort_empty'] = False
            with self.assertRaisesRegex(ValueError, 'cleanup'):
                check(case)

    def test_removal_without_correlated_restart_never_passes(self):
        case = copy.deepcopy(self.cases[0])
        for key, value in [('session_match', False), ('actual_sequence', 104685), ('actual_epoch', 2)]:
            changed = copy.deepcopy(case)
            next(r for r in changed['rows'] if r.get('actual_kind') == 10)[key] = value
            with self.assertRaises(ValueError):check(changed)

    def test_activation_only_must_not_start_worker_or_advance(self):
        case = copy.deepcopy(self.cases[1])
        summary = [r for r in case['rows'] if r['state']=='ap3_processing_summary'][1]
        summary['intervals'] = 1
        with self.assertRaisesRegex(ValueError, 'activation-only'):check(case)

    def test_fault_case_cannot_be_reported_as_normal_close(self):
        case = copy.deepcopy(self.cases[2])
        case['rows'].append({'state':'ap1_endpoint_closed','mapping_unmapped':True})
        with self.assertRaisesRegex(ValueError, 'refusal'):check(case)


if __name__ == '__main__':unittest.main()
