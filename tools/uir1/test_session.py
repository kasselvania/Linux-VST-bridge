"""Offline helper-custody checks; no process, Proton, display or input access."""
import copy
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

import session


GOOD = dict(exit=0, overflow=0,
            cleanup=dict(owned_descendants_zero=True, process_group_empty=True))


class SessionTests(unittest.TestCase):
    def run_session(self, failed_label=None, result=None):
        state = types.SimpleNamespace(moves=0, ups=0, command=0, phase_reads=0)
        helpers = []

        class Helper:
            def __init__(self, runtime, args, env, target, log, seconds):
                self.label = log.stem
                self.closed = False
                self.process = types.SimpleNamespace(poll=lambda: 0)
                helpers.append(self)
                if self.label == 'fixture':
                    (target / 'fixture.status').write_bytes(bytes(4096))
                elif self.label == 'observer':
                    (target / 'observe.status').write_bytes(bytes(8192))
                elif self.label == 'census':
                    log.write_text(json.dumps(dict(type='x11_binding', hwnd=1, xid=2)))

            def poll(self): pass

            def finish(self):
                self.closed = True
                return copy.deepcopy(result if self.label == failed_label else GOOD)

        class Mapping:
            map = b'UIR1'

            def __init__(self, *args): pass
            def close(self): pass
            def write(self, offset, value): state.command = value

            def read(self, offset):
                if offset == 8: return 2
                if offset == 64: return state.command
                if offset == 72:
                    if state.command != 2: return 1
                    if state.moves >= 402:
                        state.phase_reads += 1
                        if state.phase_reads > 1: return 3
                    return 2
                if offset == 96: return state.moves
                if offset == 128: return state.ups
                if offset == 272: return state.moves
                return 1

        class Observer:
            def __init__(self, *args): self.closed = False
            def status(self): return dict(ready=1, closed=int(self.closed))
            def action(self, *args): pass
            def clock(self): return {}
            def take(self): return []
            def stop(self): self.closed = True
            def close(self): pass

        class X11:
            def __init__(self, *args): pass
            def activate(self): pass
            def pointer(self): return dict(active=[2])
            def settle_pointer(self): pass
            def move(self, point): state.moves += 1
            def button(self, down):
                if not down: state.ups += 1
            def close(self): pass

        class Recorder:
            def __init__(self, *args):
                self.records = []; self.dropped = self.unparsed = 0
            def poll(self): pass
            def close(self): pass

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'runner.json').write_text(json.dumps(dict(entry_point='unused', proton='unused')))
            (root / 'package.json').write_text('{}')
            clock = iter(range(0, 100000, 10))
            with patch.multiple(session, Helper=Helper, Mapping=Mapping, Observer=Observer,
                                X11=X11, Recorder=Recorder, runner_environment=lambda *args: {}), \
                    patch.object(session.time, 'monotonic', side_effect=lambda: next(clock)), \
                    patch.object(session.time, 'monotonic_ns', return_value=0), \
                    patch.object(session.time, 'sleep'), \
                    patch.object(session.os, 'umask'):
                code = session.run(root, root)
            report = json.loads((root / 'result-private.json').read_text())
        self.assertTrue(all(h.closed for h in helpers))
        return code, report

    def test_all_four_positive_results_complete(self):
        code, report = self.run_session()
        self.assertEqual(code, 0)
        self.assertIs(report['completed'], True)
        for key in ('initialization', 'census_cleanup', 'observer_cleanup', 'fixture_cleanup'):
            self.assertEqual(report[key], GOOD)

    def test_each_helper_failure_prevents_completion_and_retains_result(self):
        bad_results = []
        for key in ('exit', 'overflow'):
            bad = copy.deepcopy(GOOD); bad[key] = 1; bad_results.append(bad)
        for key in ('owned_descendants_zero', 'process_group_empty'):
            for value in (False, None, 1):
                bad = copy.deepcopy(GOOD); bad['cleanup'][key] = value; bad_results.append(bad)
            bad = copy.deepcopy(GOOD); del bad['cleanup'][key]; bad_results.append(bad)
        bad_results.append(dict(exit=0, overflow=0))
        for label, key in (('initialize', 'initialization'), ('census', 'census_cleanup'),
                           ('observer', 'observer_cleanup'), ('fixture', 'fixture_cleanup')):
            for bad in bad_results:
                with self.subTest(helper=label, result=bad):
                    code, report = self.run_session(label, bad)
                    self.assertEqual(code, 1)
                    self.assertIs(report['completed'], False)
                    self.assertEqual(report[key], bad)
                    self.assertIn(key, report['error'])


if __name__ == '__main__': unittest.main()
