import copy
import json
from pathlib import Path
import unittest
from common import Refusal, digest
from records import parse, generations, role
from classify import classify
from fixtures import source_records, log, request, exited, OP


class RecordTests(unittest.TestCase):
    def setUp(self):
        self.tx, self.result, self.identity, self.image = source_records()

    def evidence(self, lines):
        return parse(self.tx, self.result, {digest(b'log'): lines}, self.identity, self.image)

    def disposition(self, lines):
        return classify(self.identity, self.evidence(lines))

    def test_clean_renderer_and_empty_dropped_remain_unresolved(self):
        self.assertIn('UNRESOLVED', self.disposition(request('renderer')))
        self.tx['diagnostics']['runner']['dropped_records'] = 50000
        self.assertIn('UNRESOLVED', self.disposition(b''))

    def test_repeated_gpu_failure_positive_despite_loss(self):
        self.tx['diagnostics']['runner']['dropped_records'] = 50000
        self.assertEqual(self.disposition(log('GPU process exited unexpectedly: exit_code=34')),
                         'NAUI1_ELECTRON_GPU_FAILURE_SELECTED')
        self.assertIn('UNRESOLVED', self.disposition(log('GPU process exited unexpectedly: exit_code=0')))

    def test_sandbox_network_renderer_gdi(self):
        cases = [('Failed to launch child: error_code=39', 'child_process_launcher_helper.cc', 'ELECTRON_SANDBOX_CHILD_FAILURE_SELECTED'),
                 ('Network service crashed, restarting service.', 'network_service_instance_impl.cc', 'ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED'),
                 ('render-process-gone: crashed', 'electron_api_web_contents.cc', 'RENDERER_PROCESS_FAILURE_SELECTED')]
        for text, source, end in cases:
            self.assertEqual(self.disposition(log(text, source=source)), 'NAUI1_'+end)
        self.assertEqual(self.disposition(b'10.300:00c8:0002:err:gdi:alloc_handle out of GDI object handles\n'), 'NAUI1_FONT_GDI_FAILURE_SELECTED')

    def test_multiple_specific_causes_refuse_priority_guess(self):
        self.assertIn('UNRESOLVED', self.disposition(log('GPU process exited unexpectedly: exit_code=34')+
                      log('Failed to launch child: error_code=39', source='child_process_launcher_helper.cc')))

    def test_role_bound_to_exact_create_completion_and_exit(self):
        app = self.tx['windows_trace']['processes'][1]; app['self_exit'] = exited(5)
        self.assertEqual(self.disposition(request()), 'NAUI1_ELECTRON_GPU_FAILURE_SELECTED')
        self.assertEqual(self.disposition(request('renderer')), 'NAUI1_RENDERER_PROCESS_FAILURE_SELECTED')
        app['self_exit']['status'] = 0
        self.assertIn('UNRESOLVED', self.disposition(request()))

    def test_role_missing_request_completion_or_wrong_generation(self):
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        for lines in (request().splitlines(keepends=True)[0], request().splitlines(keepends=True)[1], request().replace(b'10.200', b'10.201'), request().replace(b'00c8', b'00c9')):
            self.assertIn('UNRESOLVED', self.disposition(lines))

    def test_unrelated_words_source_prefix_or_pid_refuse(self):
        for line in (b'GPU process exited unexpectedly: exit_code=34\n', b'quote '+log('GPU process exited unexpectedly: exit_code=34'),
                     log('GPU process exited unexpectedly: exit_code=34', source='other.cc'),
                     log('GPU process exited unexpectedly: exit_code=34', pid=201),
                     log('see https://example.test GPU process exited unexpectedly: exit_code=34')):
            self.assertIn('UNRESOLVED', self.disposition(line))

    def test_truncated_and_saturated_records(self):
        line = log('GPU process exited unexpectedly: exit_code=34')
        self.assertIn('UNRESOLVED', self.disposition(line[:-1]))
        self.assertIn('UNRESOLVED', self.disposition(b'x'*4096+line))
        with self.assertRaises(Refusal): self.evidence(b'x'*(2*1024*1024+1))

    def test_image_missing_changed_path_or_unrooted_never_selects(self):
        for field, value in [('image_identity', None), ('image_identity', {'sha256': 'd'*64}), ('image_request', 'C:\\other\\Native Access.exe'), ('target_tree', False)]:
            with self.subTest(field=field):
                app = self.tx['windows_trace']['processes'][1]; old = copy.deepcopy(app[field]); app[field] = value
                self.assertIn('UNRESOLVED', self.disposition(log('GPU process exited unexpectedly: exit_code=34')))
                app[field] = old

    def test_duplicate_reordered_and_reused_generations(self):
        rows = self.tx['windows_trace']['processes']; orig = copy.deepcopy(rows)
        for bad in ([*orig, orig[-1]], list(reversed(orig))):
            self.tx['windows_trace']['processes'] = bad
            with self.assertRaises(Refusal): self.evidence(b'')
        self.tx['windows_trace']['processes'] = orig
        other = copy.deepcopy(orig[-1]); other['creation_ordinal'] = 3; other['created_timestamp'] = '12.000'
        orig[-1]['self_exit'] = exited(0); orig.append(other)
        self.assertIn('UNRESOLVED', self.disposition(log('GPU process exited unexpectedly: exit_code=34')))
        orig[1]['self_exit'] = None
        with self.assertRaises(Refusal): self.evidence(b'')

    def test_linux_pid_collision_cannot_confer_windows_authority(self):
        self.tx['ledger']['processes'] = [{'pid': 201, 'images': [{'sha256': self.identity['files']['Native Access.exe']['sha256']}]}]
        self.assertIn('UNRESOLVED', self.disposition(log('GPU process exited unexpectedly: exit_code=34', pid=201)))
        self.assertFalse(self.evidence(b'')['linux_windows_join_performed'])

    def test_retired_parent_and_exit_domain_refuse(self):
        self.tx['windows_trace']['processes'][0]['self_exit'] = exited(0)
        self.tx['windows_trace']['processes'][0]['self_exit']['timestamp'] = '9.000'
        with self.assertRaises(Refusal): self.evidence(b'')
        self.tx['windows_trace']['processes'][0]['self_exit'] = None
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        self.tx['windows_trace']['processes'][1]['self_exit']['domain'] = 'linux_wait'
        with self.assertRaises(Refusal): self.evidence(request())

    def test_untrusted_command_fields_private(self):
        data = request().replace(b'app --type=gpu-process', b'app --type=gpu-process --token=SECRET https://private.test')
        e = self.evidence(data)
        self.assertNotIn('SECRET', str(e)); self.assertNotIn('private.test', str(e)); self.assertNotIn('C:\\', str(e))


class ClassifierIntegrityTests(unittest.TestCase):
    setUp = RecordTests.setUp
    evidence = RecordTests.evidence
    disposition = RecordTests.disposition

    def test_each_decisive_fact_field_mutation_and_removal(self):
        e = self.evidence(log('GPU process exited unexpectedly: exit_code=34'))
        self.assertEqual(len(e['facts']), 1)
        for key in e['facts'][0]:
            for delete in (True, False):
                bad = copy.deepcopy(e)
                if delete: del bad['facts'][0][key]
                else: bad['facts'][0][key] = 'untrusted'
                with self.subTest(key=key, delete=delete), self.assertRaises((Refusal, TypeError)):
                    classify(self.identity, bad)
        e['facts'] = []
        self.assertIn('UNRESOLVED', classify(self.identity, e))

    def test_privacy_fields_rejected(self):
        e = self.evidence(log('GPU process exited unexpectedly: exit_code=34'))
        for key in ('path', 'command', 'url', 'token', 'account', 'pid'):
            bad = copy.deepcopy(e); bad['facts'][0][key] = 'SECRET'
            with self.assertRaises(Refusal): classify(self.identity, bad)

    def test_screenshot_never_authority(self):
        self.assertIn('UNRESOLVED', self.disposition(b'blank white Native Access client area\n'))


class TraceCompletenessTests(unittest.TestCase):
    setUp = RecordTests.setUp
    evidence = RecordTests.evidence

    def diagnostics(self):
        return (
            ('gpu_exit', log('GPU process exited unexpectedly: exit_code=34')),
            ('sandbox_child', log('Failed to launch child: error_code=39', source='child_process_launcher_helper.cc')),
            ('gdi_exhaustion', b'10.300:00c8:0002:err:gdi:alloc_handle out of GDI object handles\n'),
            ('renderer_gone', log('render-process-gone: crashed', source='electron_api_web_contents.cc')),
        )

    def test_windows_drops_unbind_each_pid_association(self):
        self.tx['windows_trace']['dropped_observations'] = 1
        for category, lines in self.diagnostics():
            with self.subTest(category=category):
                e = self.evidence(lines)
                self.assertEqual(e['facts'], [])
                self.assertEqual(e['unbound_diagnostic_categories'][category], 1)
                self.assertIn('UNRESOLVED', classify(self.identity, e))

    def test_classifier_independently_gates_retained_positive_facts(self):
        for category, lines in self.diagnostics():
            with self.subTest(category=category):
                e = self.evidence(lines)
                self.assertEqual(len(e['facts']), 1)
                self.assertIn('SELECTED', classify(self.identity, e))
                e['windows_dropped_observations'] = 1
                self.assertIn('UNRESOLVED', classify(self.identity, e))

    def test_role_self_exit_has_separate_conservative_authority(self):
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        for role_name in ('gpu-process', 'renderer'):
            e = self.evidence(request(role_name))
            self.assertEqual(e['facts'][0]['authority'], 'wine_exact_role_and_self_exit_generation')
            self.assertIn('SELECTED', classify(self.identity, e))
            e['windows_dropped_observations'] = 1
            self.assertIn('UNRESOLVED', classify(self.identity, e))
        self.tx['windows_trace']['dropped_observations'] = 1
        e = self.evidence(request())
        self.assertEqual(e['processes'][0]['role'], 'gpu')
        self.assertEqual(e['facts'], [])
        self.assertEqual(e['unbound_diagnostic_categories']['gpu_abnormal_exit'], 1)

    def test_complete_windows_trace_survives_runner_tail_loss(self):
        self.tx['diagnostics']['runner']['dropped_records'] = 48667
        self.tx['diagnostics']['runner']['dropped_bytes'] = 4571384
        for _, lines in self.diagnostics():
            self.assertIn('SELECTED', classify(self.identity, self.evidence(lines)))
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        self.assertIn('SELECTED', classify(self.identity, self.evidence(request())))

    def test_retained_repository_observation_stays_unresolved(self):
        # Reclassify already public bytes locally; never invoke the physical owner.
        path = Path(__file__).resolve().parents[2] / 'evidence/naui1/observation.json'
        retained = json.loads(path.read_bytes())['result']
        self.assertEqual(retained['evidence']['windows_dropped_observations'], 0)
        self.assertEqual(retained['evidence']['processes'], [])
        self.assertEqual(retained['evidence']['facts'], [])
        self.assertEqual(classify(retained['identity'], retained['evidence']),
                         'NAUI1_ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED')

    def test_completeness_closed_extent_at_both_owners(self):
        e = self.evidence(b'')
        for value in (None, True, -1, 1.0, '0', 2**32):
            with self.subTest(value=value):
                bad = copy.deepcopy(e); bad['windows_dropped_observations'] = value
                with self.assertRaises(Refusal): classify(self.identity, bad)
                self.tx['windows_trace']['dropped_observations'] = value
                with self.assertRaises(Refusal): self.evidence(b'')
        del self.tx['windows_trace']['dropped_observations']
        with self.assertRaises(Refusal): self.evidence(b'')
        del e['windows_dropped_observations']
        with self.assertRaises(Refusal): classify(self.identity, e)

    def test_process_schema_each_field_and_privacy(self):
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        e = self.evidence(request())
        for key in e['processes'][0]:
            for delete in (True, False):
                bad = copy.deepcopy(e)
                if delete: del bad['processes'][0][key]
                else: bad['processes'][0][key] = 'untrusted'
                with self.subTest(key=key, delete=delete), self.assertRaises(Refusal):
                    classify(self.identity, bad)
        for value in (None, {}, [None], e['processes']*513):
            bad = copy.deepcopy(e); bad['processes'] = value
            with self.assertRaises(Refusal): classify(self.identity, bad)
        bad = copy.deepcopy(e); bad['processes'][0]['command'] = 'PRIVATE'
        with self.assertRaises(Refusal): classify(self.identity, bad)

    def test_process_extents_nullable_pairs_and_generation_order(self):
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        e = self.evidence(request())
        for key, value in (('epoch', True), ('epoch', 1), ('ordinal', True), ('ordinal', 0),
                           ('ordinal', 513), ('exit_status', True), ('exit_status', -1),
                           ('exit_status', 2**32), ('exit_status', None), ('exit_domain', None),
                           ('role_request_sha256', None), ('role', 'unknown')):
            bad = copy.deepcopy(e); bad['processes'][0][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(Refusal):
                classify(self.identity, bad)
        other = copy.deepcopy(e['processes'][0]); other['ordinal'] = 3
        for rows in (e['processes']*2, [other, *e['processes']]):
            bad = copy.deepcopy(e); bad['processes'] = rows
            with self.assertRaises(Refusal): classify(self.identity, bad)

    def test_authority_cannot_be_relabelled(self):
        for _, lines in self.diagnostics():
            e = self.evidence(lines)
            bad = copy.deepcopy(e)
            bad['facts'][0]['authority'] = 'wine_exact_role_and_self_exit_generation'
            with self.assertRaises(Refusal): classify(self.identity, bad)
        self.tx['windows_trace']['processes'][1]['self_exit'] = exited(5)
        e = self.evidence(request())
        for key, value in (('role', 'renderer'), ('exit_status', 0),
                           ('image_digest_at_request', None)):
            bad = copy.deepcopy(e); bad['processes'][0][key] = value
            with self.assertRaises(Refusal): classify(self.identity, bad)


if __name__ == '__main__': unittest.main()
