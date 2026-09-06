"""Concurrent preview ownership; no Windows process or DAW is launched."""
import contextlib
import json
import pathlib
import shutil
import socket
import tempfile
import threading
import types
import unittest
from unittest.mock import patch
import ap4_preview as p

CLEAN = {'owned_descendants_zero': True, 'process_group_empty': True}


class IndependentOwners(unittest.TestCase):
    def test_sibling_failure_cleanup_and_capacity(self):
        started = [threading.Event() for _ in range(5)]
        finish = [threading.Event() for _ in range(5)]
        ended = [threading.Event() for _ in range(5)]
        peers = {}
        def run(peer):
            index = peers[peer.fileno()]
            started[index].set()
            self.assertTrue(finish[index].wait(5))
            ended[index].set()
            if index == 0:
                raise RuntimeError('one instance failed')
        sessions = p.Sessions(run)
        with contextlib.ExitStack() as stack:
            clients = []
            for index in range(4):
                a, b = socket.socketpair(); stack.enter_context(b)
                peers[a.fileno()] = index; clients.append(b)
                self.assertTrue(sessions.admit(a))
                self.assertTrue(started[index].wait(2))
            a, b = socket.socketpair(); stack.enter_context(b)
            self.assertFalse(sessions.admit(a)); self.assertEqual(b.recv(1), b'')
            # A failed/closing sibling does not revoke another connection.
            with patch('sys.stderr'):
                finish[0].set(); self.assertTrue(ended[0].wait(2))
                sessions.threads[0].join(2)
            self.assertEqual(clients[0].recv(1), b'')
            self.assertFalse(sessions.blocked.is_set())
            self.assertFalse(ended[1].is_set())
            a, b = socket.socketpair(); stack.enter_context(b); peers[a.fileno()] = 4
            self.assertTrue(sessions.admit(a)); self.assertTrue(started[4].wait(2))
            for event in finish: event.set()
            sessions.join()
            self.assertEqual(b.recv(1), b'')

    def test_uncertain_containment_refuses_new_work_without_stopping_sibling(self):
        release = threading.Event(); active = threading.Event()
        def run(peer):
            if peer.recv(1) == b'f':
                raise p.ContainmentError('owned cleanup failed; retained')
            active.set(); release.wait(5)
        sessions = p.Sessions(run)
        with contextlib.ExitStack() as stack:
            a, b = socket.socketpair(); stack.enter_context(b); b.sendall(b'h')
            self.assertTrue(sessions.admit(a)); self.assertTrue(active.wait(2))
            c, d = socket.socketpair(); stack.enter_context(d); d.sendall(b'f')
            with patch('sys.stderr'):
                self.assertTrue(sessions.admit(c)); sessions.threads[-1].join(2)
            self.assertTrue(sessions.blocked.is_set())
            b.setblocking(False)
            with self.assertRaises(BlockingIOError): b.recv(1)
            e, f = socket.socketpair(); stack.enter_context(f)
            self.assertFalse(sessions.admit(e)); self.assertEqual(f.recv(1), b'')
            release.set(); sessions.join()


class ReportingFailures(unittest.TestCase):
    def test_reporting_failures_do_not_decide_containment_or_retirement(self):
        for complete in (True, False):
            for fault in ('checkpoint', 'native_read', 'native_stat', 'before_retirement', 'all_writes', 'serialization'):
                with self.subTest(complete=complete, fault=fault):
                    self.session_case(dict(CLEAN, process_group_empty=complete), fault)

    def test_failed_post_retirement_report_still_admits_new_work(self):
        self.session_case(CLEAN, 'after_retirement')

    def test_retirement_failure_blocks_admission_even_when_reports_fail(self):
        self.session_case(CLEAN, 'all_writes', primary_failure=True, retirement_failure=True)

    def test_primary_and_reporting_failures_survive_separately(self):
        for complete in (True, False):
            with self.subTest(complete=complete):
                self.session_case(dict(CLEAN, process_group_empty=complete), 'all_writes', primary_failure=True)

    def test_missing_cleanup_confirmation_cannot_admit_new_work(self):
        for cleanup in ({}, {'owned_descendants_zero': True}, None):
            with self.subTest(cleanup=cleanup):
                self.session_case(cleanup, 'native_read')

    def session_case(self, cleanup, fault, *, primary_failure=False, retirement_failure=False):
        """Exercise real greeting, serve_connected disposition and Sessions admission."""
        with tempfile.TemporaryDirectory() as temp, contextlib.ExitStack() as stack:
            root = pathlib.Path(temp)
            output = root / 'results'; output.mkdir()
            environments = []
            for name in ('healthy', 'failed', 'next'):
                stage = root / name; stage.mkdir()
                (stage / 'ap1.control').write_bytes(b'ready')
                (stage / 'ap3-gui-report.jsonl').write_text('{"gain":0.25}\n')
                environments.append(types.SimpleNamespace(run_id=name, session=stage))
            healthy, failed, following = environments
            peers = {}; errors = {}; retired = []
            active = threading.Event(); release = threading.Event()
            real_write = p.write_atomic
            real_read = pathlib.Path.read_text
            real_stat = pathlib.Path.stat
            real_json = p.canonical_json

            def retire(env):
                retired.append(env.run_id)
                if env is failed and retirement_failure:
                    raise OSError('injected retirement failure')
                shutil.rmtree(env.session)

            def supervise(env, **kwargs):
                if env is healthy:
                    active.set()
                    if not release.wait(5):
                        raise RuntimeError('test sibling release timed out')
                    if kwargs['stop_requested']():
                        raise RuntimeError('healthy sibling was stopped')
                observed = {'cleanup': cleanup if env is failed else CLEAN}
                # Checkpoint I/O must not interrupt the supervisor's cleanup.
                kwargs['checkpoint']('supervisor_cleanup', {'cleanup': {}})
                kwargs['checkpoint']('supervisor_containment', observed)
                if env is failed and primary_failure:
                    raise RuntimeError('injected primary failure')
                return observed

            def write(target, data):
                if target.name == 'failed.json':
                    record = json.loads(data)
                    checkpoint = 'stage' in record
                    if (fault == 'all_writes' or fault == 'checkpoint' and checkpoint
                        or fault == 'before_retirement' and not checkpoint and not record['retired']
                        or fault == 'after_retirement' and record.get('retired')):
                        raise OSError('injected report write failure')
                real_write(target, data)

            def serialize(record):
                if fault == 'serialization' and record.get('run_id') == 'failed':
                    raise ValueError('injected report serialization failure')
                return real_json(record)

            def read(path, *args, **kwargs):
                if fault == 'native_read' and path == failed.session / 'ap3-gui-report.jsonl':
                    raise OSError('injected native report read failure')
                return real_read(path, *args, **kwargs)

            def stat(path, *args, **kwargs):
                if fault == 'native_stat' and path == failed.session / 'ap3-gui-report.jsonl':
                    raise PermissionError('injected native report stat failure')
                return real_stat(path, *args, **kwargs)

            def run(peer):
                env = peers[peer.fileno()]
                try:
                    p.serve_connected(peer, lambda: env, retire, output, lambda: False)
                except Exception as error:
                    errors[env.run_id] = error
                    raise

            sessions = p.Sessions(run)
            def start(env):
                server, client = socket.socketpair(); stack.enter_context(client)
                client.settimeout(2)
                peers[server.fileno()] = env
                admitted = sessions.admit(server)
                if admitted:
                    client.sendall(b'AP4\n')
                    size = int.from_bytes(client.recv(2), 'little')
                    reply = b''
                    while len(reply) < size:
                        chunk = client.recv(size - len(reply))
                        self.assertTrue(chunk)
                        reply += chunk
                    self.assertEqual(reply.decode().split('\n')[1], str(env.session))
                return admitted, client

            stack.enter_context(patch.object(p.runtime, 'supervise', side_effect=supervise))
            stack.enter_context(patch.object(p, 'write_atomic', side_effect=write))
            stack.enter_context(patch.object(p, 'canonical_json', side_effect=serialize))
            stack.enter_context(patch.object(pathlib.Path, 'read_text', read))
            stack.enter_context(patch.object(pathlib.Path, 'stat', stat))
            stderr = stack.enter_context(patch('sys.stderr'))
            complete = isinstance(cleanup, dict) and all(cleanup.get(key) is True for key in CLEAN)
            blocked = not complete or retirement_failure
            try:
                admitted, sibling = start(healthy)
                self.assertTrue(admitted); self.assertTrue(active.wait(2))
                sibling_thread = sessions.threads[-1]
                admitted, failed_client = start(failed)
                self.assertTrue(admitted)
                sessions.threads[-1].join(2)
                self.assertFalse(sessions.threads[-1].is_alive())
                self.assertEqual(failed_client.recv(1), b'')
                self.assertEqual(retired.count('failed'), int(complete))
                self.assertEqual(failed.session.exists(), blocked)
                self.assertEqual(sessions.blocked.is_set(), blocked)

                error = errors['failed']
                self.assertIsInstance(error, p.SessionError)
                self.assertEqual(isinstance(error, p.ContainmentError), blocked)
                record = error.record
                self.assertEqual(record['containment_confirmed'], complete)
                self.assertEqual(record['retired'], not blocked)
                self.assertEqual(record['error'], 'injected primary failure' if primary_failure else None)
                self.assertEqual(bool(record['retirement_error']), retirement_failure)
                self.assertTrue(record['reporting_errors'])
                self.assertTrue(all('injected' in item['error'] for item in record['reporting_errors']))
                # Even if every write failed, stderr carries distinct details.
                printed = ''.join(str(call.args[0]) for call in stderr.write.call_args_list)
                self.assertIn('reporting (', printed)
                self.assertIn(f'containment_confirmed={complete}, retired={not blocked}', printed)
                if primary_failure:
                    self.assertIn('primary: injected primary failure', printed)
                    self.assertEqual([item['stage'] for item in record['reporting_errors']],
                        ['supervisor_cleanup', 'supervisor_containment', 'before_retirement']
                        + (['after_retirement'] if complete else []))
                if retirement_failure:
                    self.assertIn('retirement: OSError: injected retirement failure', printed)
                if fault in ('checkpoint', 'native_read', 'native_stat', 'before_retirement') and complete:
                    self.assertEqual(json.loads(real_read(output / 'failed.json')), record)

                self.assertTrue(sibling_thread.is_alive())
                self.assertTrue(healthy.session.exists()); self.assertNotIn('healthy', retired)
                sibling.setblocking(False)
                with self.assertRaises(BlockingIOError): sibling.recv(1)
                admitted, next_client = start(following)
                self.assertEqual(admitted, not blocked)
                if blocked:
                    self.assertEqual(next_client.recv(1), b'')
            finally:
                release.set()
                for thread in sessions.threads:
                    thread.join(2)
                    self.assertFalse(thread.is_alive())
            self.assertEqual(set(errors), {'failed'})
            self.assertEqual(retired.count('healthy'), 1)
            self.assertFalse(healthy.session.exists())
            self.assertEqual(retired.count('next'), int(not blocked))
            self.assertTrue(json.loads(real_read(output / 'healthy.json'))['retired'])


if __name__ == '__main__': unittest.main()
