"""Final host custody: kernel writer generation, bounded frames and exact mapping.

Portable contract fixtures do not claim Wine integration. Linux fixtures exercise
the actual inherited socket, sender pidfds, detached host and sibling retirement.
"""
import json
import mmap
import os
import pathlib
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import ExitStack
from types import SimpleNamespace
from unittest.mock import Mock, patch

import ownership as o
import session as s


SID = 'b' * 32
FIELDS = dict(session=SID, scanner_sha256='a' * 64, module_sha256='c' * 64,
    bundle_manifest_sha256='c' * 64, implementation_source_manifest_sha256='d' * 64,
    mode='ap9-commercial', component_case='class:' + 'e' * 32, run_ordinal=1)
READY = dict(event='lifecycle', sequence=2, state='readiness_announced', **FIELDS)
MAPPED = dict(event='lifecycle', sequence=3, state='ap1_mapping_ready',
    mapping_count=1, connection_count=1, mapping_witness=True)
STARTED = dict(event='lifecycle', sequence=4, state='ap0_processing_thread_started', distinct_from_owner=True)


def status_file(directory, sid=SID):
    path = directory / 'ap12.status'
    path.write_bytes(b'LVFS' + struct.pack('<III', 2, 1024, 0) + bytes.fromhex(sid) + bytes(992))
    path.chmod(0o600)
    return path


class WriterFixture:
    def __init__(self, pid=123, start=456, proc_root=None, alive=True):
        self.pid, self.uid, self.proc_root = pid, os.getuid(), proc_root
        self.identity = (pid, start, (pid,), (1, 2))
        self.fd = pid
        self.live = alive
        self.closed = False
        self.checks = 0

    def same(self, other):
        return self.live and other.live and self.identity == other.identity

    def snapshot(self):
        self.checks += 1
        if not self.live:
            o.fail('final_host_writer_retired')
        return self.identity

    def duplicate(self):
        self.snapshot()
        return WriterFixture(self.pid, self.identity[1], self.proc_root, self.live)

    def close(self):
        self.closed = True


class FrameTests(unittest.TestCase):
    def setUp(self):
        self.parser = o.HostWriterLines()
        self.addCleanup(self.parser.close)
        self.rows = []

    def emit(self, line, writer):
        self.rows.append((line, writer.identity if writer else None))

    def test_split_and_multiple_records_keep_one_generation(self):
        first, second = WriterFixture(), WriterFixture()
        self.parser.feed(b'{"event":', first, self.emit)
        self.parser.feed(b'"lifecycle"}\ntext\n{"event":"next"}\n', second, self.emit)
        self.assertEqual([r[0] for r in self.rows], [b'{"event":"lifecycle"}', b'text', b'{"event":"next"}'])
        self.assertEqual([r[1] for r in self.rows], [first.identity, None, second.identity])
        self.assertTrue(first.closed and second.closed)
        self.assertIsNone(self.parser.writer)

    def test_interleaved_ordinary_text_and_dead_auxiliary_writer_are_diagnostics(self):
        first = WriterFixture(alive=False)
        self.parser.feed(b'delayed helper ', first, self.emit)
        second = WriterFixture(pid=124)
        self.parser.feed(b'text\n{"event":"lifecycle"}\n', second, self.emit)
        self.assertEqual(self.rows, [(b'delayed helper text', None), (b'{"event":"lifecycle"}', second.identity)])
        self.assertTrue(first.closed and second.closed)

    def test_mixed_protocol_is_refused_before_decoding(self):
        first, second = WriterFixture(), WriterFixture(pid=124)
        self.parser.feed(b'{"eve', first, self.emit)
        with self.assertRaisesRegex(RuntimeError, 'cross_sender_frame'):
            self.parser.feed(b'nt":"lifecycle"}\n', second, self.emit)
        self.assertEqual(self.rows, [])
        self.assertTrue(first.closed and second.closed)

    def test_reused_numeric_pid_cannot_complete_old_authoritative_frame(self):
        first, replacement = WriterFixture(alive=False), WriterFixture(start=457)
        self.parser.feed(b'{"event":', first, self.emit)
        with self.assertRaisesRegex(RuntimeError, 'cross_sender_frame'):
            self.parser.feed(b'"lifecycle"}\n', replacement, self.emit)
        self.assertTrue(first.closed and replacement.closed)

    def test_extent_emit_failure_and_eof_close_handles(self):
        oversized = WriterFixture()
        with self.assertRaisesRegex(RuntimeError, 'capacity'):
            self.parser.feed(b'x' * 65537, oversized, self.emit)
        self.assertTrue(oversized.closed)
        writer = WriterFixture()
        with self.assertRaisesRegex(ValueError, 'deliberate'):
            self.parser.feed(b'{"event":"x"}\n', writer, Mock(side_effect=ValueError('deliberate')))
        self.assertTrue(writer.closed)
        writer = WriterFixture(alive=False)
        self.parser.feed(b'trailing text', writer, self.emit)
        self.parser.finish(self.emit)
        self.assertEqual(self.rows[-1], (b'trailing text', None))
        self.assertTrue(writer.closed)
        writer = WriterFixture()
        self.parser.feed(b'{"event":', writer, self.emit)
        with self.assertRaisesRegex(RuntimeError, 'incomplete_frame'):
            self.parser.finish(self.emit)
        self.assertTrue(writer.closed)


class AncillaryTests(unittest.TestCase):
    def setUp(self):
        for name, value in [('SCM_CREDENTIALS', 2), ('MSG_CMSG_CLOEXEC', 0x40000000)]:
            self.enterContext(patch.object(o.socket, name, value, create=True))

    def fd(self):
        fd, writer = os.pipe()
        self.addCleanup(os.close, writer)
        return fd

    def assert_closed(self, *fds):
        for fd in fds:
            with self.assertRaises(OSError):
                os.fstat(fd)

    def receive(self, rows, flags=0, data=b'helper text\n'):
        channel = Mock()
        channel.recvmsg.return_value = (data, rows, flags, None)
        return o.receive_host_writer(channel)

    def test_dead_or_unresolvable_envelope_does_not_read_process_for_text(self):
        fd = self.fd()
        data, writer = self.receive([(socket.SOL_SOCKET, 2, struct.pack('3i', 0, os.getuid(), os.getgid())),
            (socket.SOL_SOCKET, o.SCM_PIDFD, struct.pack('i', fd))])
        self.assertEqual(data, b'helper text\n')
        self.assertIsNone(writer.identity)
        writer.close(); self.assert_closed(fd)

    def test_truncation_duplicate_and_unexpected_descriptors_are_closed(self):
        for flags, extra, data in [(socket.MSG_CTRUNC, False, b'x'), (0, True, b'x'), (0, True, b'')]:
            with self.subTest(flags=flags, extra=extra, data=data):
                fd, other = self.fd(), self.fd()
                rows = [(socket.SOL_SOCKET, 2, struct.pack('3i', 123, os.getuid(), os.getgid())),
                    (socket.SOL_SOCKET, o.SCM_PIDFD, struct.pack('i', fd)),
                    (socket.SOL_SOCKET, socket.SCM_RIGHTS if extra else o.SCM_PIDFD, struct.pack('i', other))]
                with self.assertRaisesRegex(RuntimeError, 'ancillary'):
                    self.receive(rows, flags, data)
                self.assert_closed(fd, other)

    def test_missing_credentials_closes_generation_and_eof_is_empty(self):
        fd = self.fd()
        with self.assertRaisesRegex(RuntimeError, 'ancillary'):
            self.receive([(socket.SOL_SOCKET, o.SCM_PIDFD, struct.pack('i', fd))])
        self.assert_closed(fd)
        self.assertEqual(self.receive([], data=b''), (b'', None))

    def test_kernel_eof_zero_credentials_name_no_writer_and_reject_other_ancillary(self):
        marker = (socket.SOL_SOCKET, 2, bytes(12))
        self.assertEqual(self.receive([marker], socket.MSG_CMSG_CLOEXEC, data=b''), (b'', None))
        for rows, flags in [([marker], socket.MSG_CTRUNC), ([marker], socket.MSG_TRUNC),
                ([marker, marker], 0), ([(socket.SOL_SOCKET, 2, struct.pack('3i', 123, 0, 0))], 0),
                ([(socket.SOL_SOCKET, 2, bytes(11))], 0)]:
            with self.subTest(rows=rows, flags=flags):
                with self.assertRaisesRegex(RuntimeError, 'ancillary'):
                    self.receive(rows, flags, data=b'')
        fd = self.fd()
        with self.assertRaisesRegex(RuntimeError, 'ancillary'):
            self.receive([marker, (socket.SOL_SOCKET, socket.SCM_RIGHTS, struct.pack('i', fd))], data=b'')
        self.assert_closed(fd)


class IdentityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.proc = pathlib.Path(self.tmp.name)
        (self.proc / '123/ns').mkdir(parents=True)
        (self.proc / '123/ns/pid').write_bytes(b'held namespace fixture')
        fd, writer = os.pipe()
        self.addCleanup(os.close, writer)
        self.writer = o.HostWriter(fd, 123, os.getuid(), self.proc)
        self.addCleanup(self.writer.close)
        (self.proc / 'self/fdinfo').mkdir(parents=True)
        self.fdinfo = self.proc / f'self/fdinfo/{fd}'
        self.fdinfo.write_text('Pid:\t123\nNSpid:\t123 9\n')
        self.status = self.proc / '123/status'
        self.status.write_text('Name:\tfixture\nNSpid:\t123 9\n')
        fields = ['S', '1', '123', '123', *['0'] * 15, '456']
        (self.proc / '123/stat').write_text('123 (fixture ) name) ' + ' '.join(fields))

    def test_generation_and_namespace_come_from_held_kernel_handle_and_proc(self):
        identity = self.writer.pin()
        self.assertEqual(identity[:3], (123, 456, (123, 9)))
        namespace = os.fstat(self.writer.namespace_fd)
        self.assertEqual(identity[3], (namespace.st_dev, namespace.st_ino))
        self.assertEqual(self.writer.snapshot(), identity)

    def test_receiver_proc_namespace_and_pid_mismatch_are_refused(self):
        self.fdinfo.write_text('Pid:\t124\nNSpid:\t124 9\n')
        with self.assertRaisesRegex(RuntimeError, 'writer_generation'):
            self.writer.pin()
        self.fdinfo.write_text('Pid:\t123\nNSpid:\t123 9\n')
        self.status.write_text('NSpid:\t123 8\n')
        with self.assertRaisesRegex(RuntimeError, 'writer_namespace'):
            self.writer.pin()

    def test_replaced_namespace_and_reused_pid_are_not_new_custody(self):
        self.writer.pin()
        namespace = self.proc / '123/ns/pid'
        namespace.rename(namespace.with_name('retained'))
        namespace.write_bytes(b'replacement namespace')
        with self.assertRaisesRegex(RuntimeError, 'writer_namespace'):
            self.writer.snapshot()
        with patch.object(self.writer, 'alive', return_value=False):
            with self.assertRaisesRegex(RuntimeError, 'writer_retired'):
                self.writer.pin()

    def test_process_generation_uid_and_bounded_bytes_are_enforced(self):
        self.writer.pin()
        path = self.proc / '123/stat'
        path.write_text(path.read_text().replace('456', '457'))
        with self.assertRaisesRegex(RuntimeError, 'writer_generation'):
            self.writer.snapshot()
        self.writer.uid = os.getuid() + 1
        with self.assertRaisesRegex(RuntimeError, 'writer_credentials'):
            self.writer.pin()
        self.writer.uid = os.getuid()
        path.write_bytes(b'x' * 4097)
        with self.assertRaisesRegex(RuntimeError, 'identity_extent'):
            self.writer.snapshot()


class LaunchTests(unittest.TestCase):
    def runtime(self, spec):
        runtime = object.__new__(s.NativeProtonSession)
        runtime.spec = spec
        runtime.reg = spec['registration']
        runtime.client = '/verified/runtime/client'
        runtime.endpoint = '/private/keeper/socket'
        runtime.find_keeper = Mock(); runtime.verify_tools = Mock()
        runtime.control = runtime.host_custody = runtime.host_lines = None
        runtime.remote_identity = None
        self.addCleanup(runtime.close)
        return runtime

    def test_missing_kernel_capability_refuses_before_any_child_is_started(self):
        runtime = self.runtime(dict(inspect=False, registration={'environment': {'root': '/private/environment'}}))
        with patch.object(s, 'host_writer_credentials', side_effect=RuntimeError('final_host_writer_capability_unavailable')), patch.object(s.subprocess, 'Popen') as spawn:
            with self.assertRaisesRegex(RuntimeError, 'capability_unavailable'):
                runtime.spawn(['entry', '--verb=run', '--', '/verified/proton', 'runinprefix'], {})
        spawn.assert_not_called()
        self.assertIsNone(runtime.control)

    def test_inspection_vendor_and_keeper_routes_preserve_bootstrap_and_stdout(self):
        for flags in [dict(inspect=True), dict(inspect=False, vendor_access=True), dict(inspect=False, keeper=True)]:
            with self.subTest(flags=flags):
                runtime = self.runtime(dict(**flags, registration={'environment': {'root': '/private/environment'}}))
                with patch.object(s, 'host_writer_credentials') as credentials, patch.object(s.subprocess, 'Popen') as spawn:
                    runtime.spawn(['entry', '--verb=run', '--', '/verified/proton', 'runinprefix'], {'HOME': '/preserved/home'})
                credentials.assert_not_called()
                self.assertIsNone(runtime.host_custody)
                self.assertNotIn('--credential-stdout', spawn.call_args.args[0])
                runtime.close()

    def test_preadmission_failure_cannot_report_successful_cleanup(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = pathlib.Path(tmp).resolve(); directory.chmod(0o700)
            status_file(directory)
            spec = dict(inspect=False, session=SID, directory=str(directory), report=str(directory / 'result.json'),
                registration={'environment': {'root': str(directory), 'runner': {'files': []}}, 'host': {}, 'module': {}})
            for problem in ('mapping failure', 'startup hang', 'channel loss'):
                with self.subTest(problem=problem):
                    runtime = self.runtime(spec)
                    runtime.host_custody = o.FinalHostCustody(directory, SID, FIELDS)
                    runtime.host_custody.observe(READY, WriterFixture(proc_root=directory), SimpleNamespace(owned=set()), SimpleNamespace())
                    runtime.remote_identity = (123, 456)
                    runtime.bind = Mock(side_effect=RuntimeError(problem))
                    root = SimpleNamespace(pid=123, returncode=0, poll=lambda: 0)
                    root.stdout, root.stderr = tempfile.TemporaryFile(), tempfile.TemporaryFile()
                    self.addCleanup(root.stdout.close); self.addCleanup(root.stderr.close)
                    runtime.spawn = Mock(return_value=root)
                    tracker = SimpleNamespace(update=lambda: set())
                    visibility = Mock(); visibility.suspect = None; visibility.snapshot.return_value = {}
                    with ExitStack() as mocks:
                        for override in (
                            patch.object(s, 'session_directories', return_value=(directory, directory)),
                            patch.object(s, 'command', return_value=([], b'fixture')), patch.object(s, 'verify'),
                            patch.object(s, 'environment', return_value={}), patch.object(s, 'managed_home'),
                            patch.object(s, 'transport_environment'), patch.object(s, 'delivery_trace'),
                            patch.object(s.signal, 'signal'), patch.object(s.ResultStatus, 'create'),
                            patch.object(s, 'FaultStatus', return_value=visibility), patch.object(s, 'AudioScheduling', return_value=None),
                            patch.object(s.NativeProtonSession, 'selected', return_value=runtime),
                            patch.object(s, 'ProcessTracker', return_value=tracker),
                            patch.object(s, 'cleanup_process', return_value={'owned_descendants_zero': True, 'process_group_empty': True})):
                            mocks.enter_context(override)
                        retire = mocks.enter_context(patch.object(s, 'retire_native_transport'))
                        outcome = s.run_owned(spec, None, [False])
                    self.assertFalse(outcome['cleanup_confirmed'])
                    self.assertIn('custody incomplete', outcome['error'])
                    self.assertEqual(outcome['final_windows_host']['availability'], 'candidate')
                    retire.assert_not_called()

    def test_immediate_failure_or_stop_after_mapping_retires_exact_host_without_render_start(self):
        for stop in (False, True):
            with self.subTest(stop=stop), tempfile.TemporaryDirectory() as tmp:
                directory = pathlib.Path(tmp).resolve(); directory.chmod(0o700)
                status = status_file(directory).stat()
                proc = directory / 'proc'; (proc / '123').mkdir(parents=True)
                (proc / '123/maps').write_text(f'1000-2000 rw-s 00000000 {os.major(status.st_dev):02x}:{os.minor(status.st_dev):02x} {status.st_ino} /ignored-path\n')
                spec = dict(inspect=False, session=SID, directory=str(directory), report=str(directory / 'result.json'),
                    registration={'environment': {'root': str(directory), 'runner': {'files': []}}, 'host': {}, 'module': {}})
                runtime = self.runtime(spec)
                runtime.host_custody = o.FinalHostCustody(directory, SID, FIELDS)
                runtime.host_lines = o.HostWriterLines(); runtime.control = Mock()
                runtime.remote_identity = (999, 5); runtime.bind = Mock()
                root = SimpleNamespace(pid=999, returncode=0, poll=lambda: None)
                root.stdout, root.stderr = tempfile.TemporaryFile(), tempfile.TemporaryFile()
                self.addCleanup(root.stdout.close); self.addCleanup(root.stderr.close)
                runtime.spawn = Mock(return_value=root)
                tracker = SimpleNamespace(owned={(999, 5)}); tracker.update = lambda: tracker.owned
                sibling = WriterFixture(pid=124, start=457)
                writer = WriterFixture(proc_root=proc)
                data = b''.join(json.dumps(row, separators=(',', ':')).encode() + b'\n' for row in (READY, MAPPED))
                if not stop: data += b'{"event":malformed}\n'
                stop_requested = [False]
                def receive(_):
                    stop_requested[0] = stop
                    return data, writer
                retired = []
                def cleanup(current_root, owned):
                    retired.extend(owned)
                    o.signal_final_host(current_root, owned, signal.SIGTERM)
                    return {'owned_descendants_zero': True, 'process_group_empty': True}
                selector = Mock(); selector.select.return_value = [(SimpleNamespace(data='host', fileobj=runtime.control), 1)]
                visibility = Mock(); visibility.suspect = None; visibility.snapshot.return_value = {}
                scheduling = Mock(); scheduling.pending = 0; scheduling.value.return_value = {'requests': 0}
                with ExitStack() as mocks:
                    for override in (
                        patch.object(s, 'session_directories', return_value=(directory, directory)),
                        patch.object(s, 'command', return_value=([], b'fixture')), patch.object(s, 'verify'),
                        patch.object(s, 'environment', return_value={}), patch.object(s, 'managed_home'),
                        patch.object(s, 'transport_environment'), patch.object(s, 'delivery_trace'),
                        patch.object(s.signal, 'signal'), patch.object(s.ResultStatus, 'create'),
                        patch.object(s, 'FaultStatus', return_value=visibility), patch.object(s, 'AudioScheduling', return_value=scheduling),
                        patch.object(s.NativeProtonSession, 'selected', return_value=runtime),
                        patch.object(s, 'ProcessTracker', return_value=tracker), patch.object(s.selectors, 'DefaultSelector', return_value=selector),
                        patch.object(s, 'receive_host_writer', side_effect=receive), patch.object(s, 'cleanup_process', side_effect=cleanup)):
                        mocks.enter_context(override)
                    send = mocks.enter_context(patch.object(o.signal, 'pidfd_send_signal', create=True))
                    retire = mocks.enter_context(patch.object(s, 'retire_native_transport', return_value={'transport_retired': True}))
                    outcome = s.run_owned(spec, None, stop_requested)
                self.assertTrue(outcome['cleanup_confirmed'], outcome['error'])
                self.assertEqual(set(retired), {(999, 5), (123, 456)})
                self.assertNotIn(sibling.identity[:2], retired)
                send.assert_called_once_with(writer.fd, signal.SIGTERM)
                self.assertTrue(sibling.live); self.assertFalse(sibling.closed)
                scheduling.started.assert_not_called()
                retire.assert_called_once()
                self.assertEqual(outcome['final_windows_host']['availability'], 'owned')
                self.assertTrue(outcome['final_windows_host']['status_mapped'])
                self.assertEqual(bool(outcome['error']), not stop)

    def test_final_cleanup_drain_cannot_admit_detached_host_after_last_signal(self):
        for candidate_before_stop in (False, True):
            with self.subTest(candidate_before_stop=candidate_before_stop), tempfile.TemporaryDirectory() as tmp:
                directory = pathlib.Path(tmp).resolve(); directory.chmod(0o700)
                status = status_file(directory).stat()
                proc = directory / 'proc'; (proc / '123').mkdir(parents=True)
                (proc / '123/maps').write_text(f'1000-2000 rw-s 00000000 {os.major(status.st_dev):02x}:{os.minor(status.st_dev):02x} {status.st_ino} /detached-host-status\n')
                spec = dict(inspect=False, session=SID, directory=str(directory), report=str(directory / 'result.json'),
                    crash_capture={'id': 'fixture', 'directory': str(directory / 'capture')},
                    registration={'environment': {'root': str(directory), 'runner': {'files': []}}, 'host': {}, 'module': {}})
                runtime = self.runtime(spec)
                runtime.host_custody = o.FinalHostCustody(directory, SID, FIELDS)
                runtime.host_lines = o.HostWriterLines(); channel = runtime.control = Mock()
                runtime.remote_identity = (999, 5); runtime.bind = Mock()
                root = SimpleNamespace(pid=999, returncode=0, poll=lambda: 0, wait=Mock(return_value=0))
                root.stdout, root.stderr = tempfile.TemporaryFile(), tempfile.TemporaryFile()
                self.addCleanup(root.stdout.close); self.addCleanup(root.stderr.close)
                runtime.spawn = Mock(return_value=root)
                tracker = SimpleNamespace(owned={(999, 5)}); tracker.update = lambda: tracker.owned
                first, last = WriterFixture(proc_root=proc), WriterFixture(proc_root=proc)
                def encode(rows):
                    return b''.join(json.dumps(row, separators=(',', ':')).encode() + b'\n' for row in rows)
                initial = encode([READY]) if candidate_before_stop else b'ordinary startup text\n'
                late = encode([MAPPED] if candidate_before_stop else [READY, MAPPED]) + b'late diagnostic retained\n'
                stop_requested = [False]; order = []; reads = 0
                def receive(_):
                    nonlocal reads
                    reads += 1
                    if reads == 1:
                        stop_requested[0] = True
                        return initial, first
                    order.append('late mapping')
                    return late, last
                selector = Mock(); selector.get_map.return_value = {}
                event = [(SimpleNamespace(data='host', fileobj=channel), 1)]
                selector.select.side_effect = [event, [], event]
                signal_host = o.signal_final_host
                def signal_and_record(current_root, owned, signum):
                    order.append(signum)
                    return signal_host(current_root, owned, signum)
                visibility = Mock(); visibility.suspect = None; visibility.snapshot.return_value = {}
                capture = Mock(); capture.path = directory / 'capture'; capture.counts = {}
                with ExitStack() as mocks:
                    for override in (
                        patch.object(s, 'session_directories', return_value=(directory, directory)),
                        patch.object(s, 'command', return_value=([], b'fixture')), patch.object(s, 'verify'),
                        patch.object(s, 'environment', return_value={}), patch.object(s, 'managed_home'),
                        patch.object(s, 'transport_environment'), patch.object(s, 'delivery_trace'),
                        patch.object(s.signal, 'signal'), patch.object(s.ResultStatus, 'create'),
                        patch.object(s, 'FaultStatus', return_value=visibility), patch.object(s, 'AudioScheduling', return_value=None),
                        patch.object(s, 'IncidentCapture', return_value=capture), patch.object(s, 'vendor_diagnostic_environment', return_value={}),
                        patch.object(s.NativeProtonSession, 'selected', return_value=runtime),
                        patch.object(s, 'ProcessTracker', return_value=tracker), patch.object(s.selectors, 'DefaultSelector', return_value=selector),
                        patch.object(s, 'receive_host_writer', side_effect=receive), patch.object(o, 'process_identities', return_value=[]),
                        patch.object(o, 'signal_final_host', side_effect=signal_and_record)):
                        mocks.enter_context(override)
                    send = mocks.enter_context(patch.object(o.signal, 'pidfd_send_signal', create=True))
                    retire = mocks.enter_context(patch.object(s, 'retire_native_transport'))
                    outcome = s.run_owned(spec, None, stop_requested)
                self.assertGreater(order.index('late mapping'), order.index(signal.SIGKILL))
                self.assertFalse(outcome['cleanup_confirmed'], outcome['error'])
                self.assertIn('custody incomplete', outcome['error'])
                self.assertEqual(tracker.owned, {(999, 5)})
                self.assertFalse(hasattr(root, 'lvb_host_pidfd'))
                self.assertFalse(outcome['final_windows_host']['status_mapped'])
                self.assertEqual(outcome['final_windows_host']['availability'], 'candidate' if candidate_before_stop else 'unavailable')
                self.assertIn('late diagnostic retained', outcome['vendor_stdout'])
                self.assertTrue(first.closed and last.closed)
                channel.close.assert_called_once()
                send.assert_not_called(); retire.assert_not_called()
                self.assertFalse(json.loads(pathlib.Path(spec['report']).read_text())['cleanup_confirmed'])


class MappingTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.directory = pathlib.Path(self.tmp.name) / 'session'; self.directory.mkdir(mode=0o700)
        self.path = status_file(self.directory)
        self.proc = pathlib.Path(self.tmp.name) / 'proc'
        (self.proc / '123').mkdir(parents=True)
        self.writer = WriterFixture(proc_root=self.proc)
        self.custody = o.FinalHostCustody(self.directory, SID, FIELDS)
        self.addCleanup(self.custody.close)
        self.tracker, self.root = SimpleNamespace(owned={(999, 5)}), SimpleNamespace()
        self.map_status()

    def map_status(self, inode=None):
        st = self.path.stat()
        (self.proc / '123/maps').write_text(f'1000-2000 rw-s 00000000 {os.major(st.st_dev):02x}:{os.minor(st.st_dev):02x} {st.st_ino if inode is None else inode} /ignored-path\n')

    def observe(self, row, writer=None):
        self.custody.observe(row, writer or self.writer, self.tracker, self.root)

    def test_exact_readiness_is_candidate_only_until_mapped(self):
        self.observe(READY)
        self.assertEqual(self.tracker.owned, {(999, 5)})
        self.assertFalse(hasattr(self.root, 'lvb_host_pidfd'))
        self.assertEqual(self.custody.value()['availability'], 'candidate')
        self.assertFalse(self.custody.value()['status_mapped'])
        self.observe(MAPPED)
        self.assertEqual(self.tracker.owned, {(999, 5), (123, 456)})
        self.assertEqual(self.root.lvb_host_pidfd[0], (123, 456))
        self.assertTrue(self.custody.value()['status_mapped'])
        self.assertNotIn('audio_ready', self.custody.value())

    def test_wrong_session_origin_and_dead_readiness_never_adopt(self):
        for row in [{**READY, 'session': 'f' * 32}, {**READY, 'scanner_sha256': '0' * 64},
                    {**READY, 'run_ordinal': True}, {**READY, 'extra': 'unsupported'}]:
            with self.subTest(row=row), self.assertRaisesRegex(RuntimeError, 'readiness_binding'):
                self.observe(row)
        self.writer.live = False
        with self.assertRaisesRegex(RuntimeError, 'writer_retired'):
            self.observe(READY)
        self.assertEqual(self.tracker.owned, {(999, 5)})

    def test_keeper_sibling_and_reused_generation_cannot_supply_mapping(self):
        self.observe(READY)
        for writer in [WriterFixture(pid=124), WriterFixture(start=457)]:
            with self.assertRaisesRegex(RuntimeError, 'sender_changed'):
                self.observe(MAPPED, writer)
        self.map_status(inode=self.path.stat().st_ino + 1)
        with self.assertRaisesRegex(RuntimeError, 'mapping_unassigned'):
            self.observe(MAPPED)
        self.assertFalse(self.custody.admitted)
        self.assertEqual(self.tracker.owned, {(999, 5)})
        self.assertFalse(hasattr(self.root, 'lvb_host_pidfd'))

    def test_every_render_restart_revalidates_mapping_without_losing_cleanup_custody(self):
        self.observe(READY); self.observe(MAPPED); self.observe(STARTED)
        self.map_status(inode=self.path.stat().st_ino + 1)
        with self.assertRaisesRegex(RuntimeError, 'mapping_unassigned'):
            self.observe({**STARTED, 'sequence': 8})
        self.assertTrue(self.custody.admitted)
        self.assertFalse(self.custody.value()['status_mapped'])
        self.assertEqual(self.root.lvb_host_pidfd[0], (123, 456))

    def test_replaced_status_path_header_and_session_directory_refuse(self):
        self.observe(READY); self.observe(MAPPED)
        original = self.path.read_bytes()
        self.path.write_bytes(b'BAD!' + original[4:])
        with self.assertRaisesRegex(RuntimeError, 'status_changed'):
            self.observe(STARTED)
        self.path.write_bytes(original)
        self.path.rename(self.path.with_suffix('.retained'))
        status_file(self.directory)
        with self.assertRaisesRegex(RuntimeError, 'status_changed'):
            self.observe(STARTED)
        self.directory.rename(self.directory.with_name('retained'))
        self.directory.mkdir(mode=0o700); status_file(self.directory)
        with self.assertRaisesRegex(RuntimeError, 'status_changed'):
            self.observe(STARTED)

    def test_status_symlink_and_nonprivate_file_refuse_before_launch(self):
        self.path.chmod(0o644)
        with self.assertRaisesRegex(RuntimeError, 'status_binding'):
            o.FinalHostCustody(self.directory, SID, FIELDS)
        self.path.chmod(0o600); self.path.rename(self.path.with_suffix('.actual'))
        self.path.symlink_to('ap12.actual')
        with self.assertRaises(OSError):
            o.FinalHostCustody(self.directory, SID, FIELDS)

    def test_candidate_alone_is_not_retirement_authority(self):
        self.observe(READY)
        with patch.object(o.signal, 'pidfd_send_signal', create=True) as send:
            o.signal_final_host(self.root, self.tracker.owned, signal.SIGTERM)
        send.assert_not_called()
        self.observe(MAPPED)
        with patch.object(o.signal, 'pidfd_send_signal', create=True) as send:
            o.signal_final_host(self.root, self.tracker.owned, signal.SIGTERM)
            self.assertEqual(send.call_args.args, (self.custody.candidate.fd, signal.SIGTERM))
            with self.assertRaisesRegex(RuntimeError, 'retirement_custody'):
                o.signal_final_host(self.root, {(999, 5)}, signal.SIGKILL)


@unittest.skipUnless(sys.platform == 'linux', 'Linux kernel sender pidfds')
class KernelTests(unittest.TestCase):
    def pair(self):
        parent, child = socket.socketpair()
        self.addCleanup(parent.close); self.addCleanup(child.close)
        o.host_writer_credentials(parent)
        parent.settimeout(3)
        return parent, child

    def test_owner_and_native_render_thread_have_same_pinned_process(self):
        parent, child = self.pair()
        code = 'import os,threading,time; os.write(1,b"owner\\n"); t=threading.Thread(target=lambda:os.write(1,b"render\\n")); t.start(); t.join(); time.sleep(3)'
        process = subprocess.Popen([sys.executable, '-c', code], stdout=child, stderr=subprocess.DEVNULL)
        try:
            rows = []
            parser = o.HostWriterLines()
            def emit(line, writer):
                rows.append(line)
            while len(rows) < 2:
                data, writer = o.receive_host_writer(parent)
                self.assertEqual(writer.pid, process.pid)
                self.assertEqual(writer.pin()[0], process.pid)
                parser.feed(data, writer, emit)
            self.assertEqual(rows, [b'owner', b'render'])
            parser.close()
        finally:
            process.kill(); process.wait()

    def test_sender_exit_before_receive_is_diagnostic_but_cannot_be_pinned(self):
        parent, child = self.pair()
        process = subprocess.Popen([sys.executable, '-c', 'import os;os.write(1,b"delayed helper\\n")'], stdout=child)
        process.wait(timeout=3)
        data, writer = o.receive_host_writer(parent)
        try:
            self.assertEqual(data, b'delayed helper\n')
            with self.assertRaisesRegex(RuntimeError, 'retired|credentials'):
                writer.pin()
            parser = o.HostWriterLines(); rows = []
            parser.feed(data, writer, lambda line, source: rows.append((line, source)))
            self.assertEqual(rows, [(b'delayed helper', None)])
            parser.close()
        finally:
            writer.close()

    def test_real_mixed_sender_fragments_are_refused_and_all_received_fds_close(self):
        parent, child = self.pair()
        parser = o.HostWriterLines(); processes = []
        try:
            first = subprocess.Popen([sys.executable, '-c', 'import os,time;os.write(1,b\'{"event":\');time.sleep(3)'], stdout=child)
            processes.append(first)
            data, writer = o.receive_host_writer(parent)
            self.assertEqual(writer.pid, first.pid)
            parser.feed(data, writer, lambda *_: self.fail('partial frame emitted'))
            second = subprocess.Popen([sys.executable, '-c', 'import os,time;os.write(1,b\'"lifecycle"}\\n\');time.sleep(3)'], stdout=child)
            processes.append(second)
            data, writer = o.receive_host_writer(parent)
            received_fd = writer.fd
            with self.assertRaisesRegex(RuntimeError, 'cross_sender_frame'):
                parser.feed(data, writer, lambda *_: self.fail('mixed frame emitted'))
            self.assertIsNone(parser.writer)
            with self.assertRaises(OSError):os.fstat(received_fd)
        finally:
            parser.close()
            for process in processes:
                process.kill(); process.wait()

    def test_actual_truncated_ancillary_rights_do_not_leak_descriptors(self):
        parent, child = self.pair()
        fd = os.open('/dev/null', os.O_RDONLY)
        try:
            before = len(list(pathlib.Path('/proc/self/fd').iterdir()))
            for _ in range(16):
                child.sendmsg([b'x'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, struct.pack('3i', fd, fd, fd))])
                with self.assertRaisesRegex(RuntimeError, 'ancillary'):
                    o.receive_host_writer(parent)
            self.assertEqual(len(list(pathlib.Path('/proc/self/fd').iterdir())), before)
        finally:
            os.close(fd)

    def launch_host(self, directory, sid=SID, mode='detached'):
        path = status_file(directory, sid)
        parent, child = self.pair()
        nonce = 'a' * 64
        fields = {**FIELDS, 'session': sid}
        ready = {**READY, 'session': sid}
        program = r'''
import json,mmap,os,pathlib,signal,sys,threading,time
path,ready,mode=sys.argv[1:]
if mode in ('detached','wrong-mapping','hang','channel-loss'):
    if os.fork():os._exit(0)
    os.setsid()
signal.signal(signal.SIGTERM,signal.SIG_IGN)
if mode=='backpressure':
    for _ in range(2048):os.write(1,b'x'*1024+b'\n')
os.write(1,(ready+'\n').encode())
if mode=='channel-loss':
    os.close(1);time.sleep(30);os._exit(0)
if mode=='hang':time.sleep(30)
source=open(path if mode!='wrong-mapping' else path+'.wrong','r+b')
mapping=mmap.mmap(source.fileno(),1024)
os.write(1,(json.dumps(dict(event='lifecycle',sequence=3,state='ap1_mapping_ready',mapping_count=1,connection_count=1,mapping_witness=True))+'\n').encode())
thread=threading.Thread(target=lambda:os.write(1,(json.dumps(dict(event='lifecycle',sequence=4,state='ap0_processing_thread_started',distinct_from_owner=True))+'\n').encode()))
thread.start();thread.join()
if mode=='eof':
    release=pathlib.Path(path+'.release')
    end=time.monotonic()+10
    while not release.exists() and time.monotonic()<end:time.sleep(.01)
    os.write(1,b'{"event":"lifecycle","sequence":5,"state":"scanner_completed"}\n')
else:time.sleep(30)
'''
        if mode == 'wrong-mapping':
            wrong = pathlib.Path(str(path) + '.wrong'); wrong.write_bytes(path.read_bytes()); wrong.chmod(0o600)
        remote = subprocess.Popen([sys.executable, '-I', '-c', s.NATIVE_COMMAND_CHILD,
            str(child.fileno()), nonce, '--credential-stdout', '--', sys.executable, '-c',
            program, str(path), json.dumps(ready), mode], pass_fds=(child.fileno(),),
            start_new_session=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        child.close()
        client = subprocess.Popen(['/bin/sleep', '30'], start_new_session=True)
        runtime = object.__new__(s.NativeProtonSession)
        runtime.control, runtime.nonce, runtime.remote_identity = parent, nonce, None
        runtime.host_custody = o.FinalHostCustody(directory, sid, fields)
        runtime.host_lines = o.HostWriterLines()
        tracker = o.ProcessTracker(client.pid)
        runtime.bind(client, tracker, lambda _: None)
        rows = []
        def emit(line, writer):
            if writer is not None:
                record = json.loads(line)
                runtime.host_custody.observe(record, writer, tracker, client)
                rows.append(record)
        def cleanup():
            # Test-created detached children are retired through their observed
            # kernel handles; this does not grant candidate cleanup to product.
            if runtime.host_custody.candidate is not None:
                try:signal.pidfd_send_signal(runtime.host_custody.candidate.fd, signal.SIGKILL)
                except ProcessLookupError:pass
            runtime.close()
            for process in (remote, client):
                if process.poll() is None:process.kill()
                process.wait()
            remote.stdout.close(); remote.stderr.close()
        self.addCleanup(cleanup)
        def read_until(predicate, timeout=5):
            end = time.monotonic() + timeout
            while not predicate():
                if time.monotonic() > end:raise TimeoutError('fixture host output')
                data, writer = o.receive_host_writer(parent)
                if not data:
                    runtime.host_lines.finish(emit)
                    return False
                runtime.host_lines.feed(data, writer, emit)
            return True
        return runtime, tracker, client, remote, rows, read_until, path

    def test_detached_final_host_retirement_does_not_retire_independent_sibling(self):
        with tempfile.TemporaryDirectory() as tmp:
            first, second = pathlib.Path(tmp) / 'one', pathlib.Path(tmp) / 'two'
            first.mkdir(mode=0o700); second.mkdir(mode=0o700)
            a = self.launch_host(first)
            b = self.launch_host(second, sid='f' * 32)
            for runtime, _, _, _, rows, read, _ in (a, b):
                self.assertTrue(read(lambda: len(rows) == 3))
                self.assertTrue(runtime.host_custody.admitted)
            runtime, tracker, client, remote, _, _, _ = a
            host = runtime.host_custody.candidate
            self.assertNotEqual(host.pid, remote.pid)
            self.assertNotEqual(os.getpgid(host.pid), runtime.remote_identity[0])
            remote.wait(timeout=3)
            with patch.object(o, 'CLEANUP_SECONDS', 3.5):
                result = o.cleanup_process(client, sorted(tracker.owned))
            self.assertEqual(result, {'owned_descendants_zero': True, 'process_group_empty': True})
            self.assertFalse(host.alive())
            self.assertTrue(b[0].host_custody.candidate.alive())
            self.assertIsNone(b[2].poll())

    def test_bootstrap_stdout_remains_open_after_duplicate_fd_closes_and_eof_is_drained(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = pathlib.Path(tmp); directory.chmod(0o700)
            runtime, _, _, remote, rows, read, path = self.launch_host(directory, mode='eof')
            self.assertTrue(read(lambda: len(rows) == 3))
            self.assertTrue(runtime.host_custody.admitted)
            pathlib.Path(str(path) + '.release').touch()
            remote.wait(timeout=3)
            self.assertFalse(read(lambda: len(rows) > 4))
            self.assertEqual(rows[-1]['state'], 'scanner_completed')
            self.assertEqual(remote.stdout.read(), b'')
            self.assertIsNone(runtime.host_lines.writer)

    def test_backpressure_preserves_host_frames_with_bounded_diagnostic_buffer(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = pathlib.Path(tmp); directory.chmod(0o700)
            runtime, _, _, _, rows, read, _ = self.launch_host(directory, mode='backpressure')
            self.assertTrue(read(lambda: len(rows) == 3, timeout=8))
            self.assertTrue(runtime.host_custody.admitted)
            self.assertLessEqual(len(runtime.host_lines.pending), 65536)

    def test_detached_preadmission_mapping_failure_hang_and_channel_loss_remain_unowned(self):
        with tempfile.TemporaryDirectory() as tmp:
            for mode in ('wrong-mapping', 'hang', 'channel-loss'):
                with self.subTest(mode=mode):
                    directory = pathlib.Path(tmp) / mode; directory.mkdir(mode=0o700)
                    runtime, tracker, _, _, rows, read, _ = self.launch_host(directory, mode=mode)
                    if mode == 'wrong-mapping':
                        with self.assertRaisesRegex(RuntimeError, 'mapping_unassigned'):
                            read(lambda: len(rows) == 3)
                    else:
                        self.assertTrue(read(lambda: len(rows) == 1))
                        if mode == 'channel-loss':
                            self.assertFalse(read(lambda: len(rows) > 1))
                        else:
                            runtime.control.settimeout(.05)
                            with self.assertRaises(socket.timeout):
                                o.receive_host_writer(runtime.control)
                            runtime.control.settimeout(3)
                    self.assertFalse(runtime.host_custody.admitted)
                    self.assertNotIn(runtime.host_custody.candidate.identity[:2], tracker.owned)
                    self.assertTrue(runtime.host_custody.candidate.alive())


if __name__ == '__main__':
    unittest.main()
