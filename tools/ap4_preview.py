#!/usr/bin/env python3
"""Private, bounded multi-instance AGain preview owner for ordinary desktop Bitwig.

Uses the existing pinned host/fixture, disposable stage, supervisor and cleanup.
No DAW launcher, public service, vendor selection or general broker API.
"""
import argparse
import json
import os
import pathlib
import secrets
import signal
import socket
import stat
import struct
import sys
import time
import tempfile
import subprocess
import importlib.util
import threading

ROOT = pathlib.Path(__file__).resolve().parents[1]
# These are the same six frozen dependencies used by AP4's existing launcher.
# Keep their private import directory alive for this owner process. The current
# artifact verifier is the existing successor used for newer host manifests.
_helpers = tempfile.TemporaryDirectory(prefix='ap4-launcher-')
for _name in ('common', 'artifacts', 'environment', 'normalize', 'supervise', 'run'):
    _bytes = subprocess.check_output(['git', 'show',
        '309b8918c128c0b9e6701d0453dc841a111d5ac5:tools/wf0-factory-census/' + _name + '.py'], cwd=ROOT)
    (pathlib.Path(_helpers.name) / (_name + '.py')).write_bytes(_bytes)
sys.path.insert(0, _helpers.name)
import common as _common
# The frozen helpers normally live inside a checkout. Their repository reads
# belong to this owner checkout, not the temporary Python import directory.
def _repository():
    observed = subprocess.check_output(['git', 'rev-parse', '--show-toplevel'], cwd=ROOT, text=True).strip()
    if pathlib.Path(observed).resolve() != ROOT.resolve():
        raise RuntimeError('preview source checkout differs')
    return ROOT
_common.repo_root = _repository
_spec = importlib.util.spec_from_file_location('ap0_artifacts', ROOT / 'tools/wf0-factory-census/artifacts.py')
artifacts = importlib.util.module_from_spec(_spec)
sys.modules['ap0_artifacts'] = artifacts
_spec.loader.exec_module(artifacts)
import ap4_worker_support as profile
import pc0_diagnostic_primitives as runtime
from common import real_home, canonical_json, write_atomic, dx0_deck_fixture_parent, DX0_AGAIN_BUNDLE_MANIFEST_SHA256


def private_directory(path):
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    meta = path.lstat()
    if not stat.S_ISDIR(meta.st_mode) or meta.st_uid != os.getuid() or meta.st_mode & 0o077:
        raise RuntimeError('preview directory must be private and owned')


class Connection:
    """Native lifetime is independent of the screenshare and SSH connection."""
    def __init__(self, peer, stopping):
        self.peer = peer
        self.stopping = stopping
        self.disconnected_at = None
        peer.setblocking(False)

    def stopped(self):
        if self.stopping():
            return True
        if self.disconnected_at is None:
            try:
                data = self.peer.recv(1)
            except BlockingIOError:
                return False
            except OSError:
                data = b''
            # No further client messages exist. Any data is a protocol error.
            self.disconnected_at = time.monotonic() if not data else 0
        # Let normal Windows close finish; bounded cleanup follows a crash.
        return time.monotonic() - self.disconnected_at >= 2


def greeting(peer, expected=b'AP4\n'):
    end = time.monotonic() + 5
    data = b''
    while len(data) < len(expected):
        remaining = end - time.monotonic()
        if remaining <= 0:
            raise TimeoutError('preview greeting deadline')
        peer.settimeout(remaining)
        chunk = peer.recv(len(expected) - len(data))
        if not chunk:
            raise RuntimeError('preview startup disconnected')
        data += chunk
    if data != expected:
        raise RuntimeError('preview greeting differs')


def wait_control(environment, connection, seconds=10):
    until = time.monotonic() + seconds
    while not (environment.session / 'ap1.control').exists():
        if connection.stopped():
            raise RuntimeError('native disconnected during startup')
        if time.monotonic() >= until:
            raise TimeoutError('native transport startup timeout')
        time.sleep(.02)


def serve(peer, create_environment, retire_environment, output, stopping):
    with peer:
        return serve_connected(peer, create_environment, retire_environment, output, stopping)


def serve_connected(peer, create_environment, retire_environment, output, stopping, *, supervision=None, component_case='exact-again', greeting_data=None):
    supervision = supervision or profile
    environment = None
    observed = None
    containment = True  # no Windows process until supervise is entered
    failure = None
    reporting_errors = []
    native_report = None

    def contained(observation):
        cleanup = observation.get('cleanup') if isinstance(observation, dict) else None
        return isinstance(cleanup, dict) and all(cleanup.get(key) is True
            for key in ('owned_descendants_zero', 'process_group_empty'))

    def report_error(stage, error):
        reporting_errors.append({'stage': stage, 'error': f'{type(error).__name__}: {error}'})
    # Linux records the connecting native process directly from the socket,
    # independently of DAW hosting settings or a client-supplied identifier.
    native_process_id = None
    if hasattr(socket, 'SO_PEERCRED'):
        native_process_id = struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[0]
    try:
        greeting(peer) if greeting_data is None else greeting(peer, greeting_data)
        environment = create_environment()
        session = secrets.token_hex(16)
        native_report = output / ('native-' + session + '.jsonl')
        reply = (session + '\n' + str(environment.session)).encode()
        if len(reply) > 1024:
            raise RuntimeError('preview startup reply bound')
        peer.settimeout(5)
        peer.sendall(struct.pack('<H', len(reply)) + reply)
        connection = Connection(peer, stopping)
        wait_control(environment, connection)
        def checkpoint(stage, available=None, error=None):
            nonlocal observed, containment
            if available is not None:
                observed = available
                containment = contained(available)
            # Useful private records, before any stage retirement.
            try:
                write_atomic(output / (environment.run_id + '.json'), canonical_json({
                    'stage': stage, 'observation': observed,
                    'error': str(error) if error else None}))
            except Exception as error:
                report_error(stage, error)  # never interrupt owned cleanup
        containment = False
        observed = runtime.supervise(environment, mode=supervision.MODE, profile=supervision, component_case=component_case,
            session_override=session, checkpoint=checkpoint,
            post_gate_seconds=None, stop_requested=connection.stopped)
        containment = contained(observed)
    except Exception as error:
        failure = str(error)
    finally:
        if environment is not None:
            report = environment.session / 'ap3-gui-report.jsonl'
            native = None
            try:
                if native_report is not None and native_report.exists():
                    report = native_report
                if report.is_file() and not report.is_symlink() and report.stat().st_size <= 65536:
                    native = report.read_text()
            except Exception as error:
                report_error('native_report', error)
            record = {'run_id': environment.run_id, 'native_process_id': native_process_id, 'observation': observed,
                      'native_report': native, 'error': failure, 'reporting_errors': reporting_errors,
                      'containment_confirmed': containment, 'retirement_error': None, 'retired': False}
            target = output / (environment.run_id + '.json')

            def persist(stage):
                try:
                    write_atomic(target, canonical_json(record))
                except Exception as error:
                    report_error(stage, error)

            persist('before_retirement')
            if containment:
                # Reporting is best effort; only the owned cleanup/disposition
                # result decides whether another session can be admitted.
                try:
                    retire_environment(environment)
                    record['retired'] = True
                except Exception as error:
                    record['retirement_error'] = f'{type(error).__name__}: {error}'
                persist('after_retirement')
                if not record['retired']:
                    raise ContainmentError('preview retirement incomplete; stage disposition unresolved', record)
                # AP6 recovery waits for positive owned-resource disposition;
                # EOF alone is also possible after containment failure.
                try:
                    peer.settimeout(5)
                    peer.sendall(b'R')
                except OSError:
                    pass  # a departed client cannot undo completed retirement
            else:
                raise ContainmentError('preview containment incomplete; stage retained', record)
            if failure is not None or reporting_errors:
                raise SessionError('preview instance failed', record)
    if failure is not None:
        raise RuntimeError(failure)


class SessionError(RuntimeError):
    """Keep the final outcome available even when no diagnostic can be written."""
    def __init__(self, message, record=None):
        self.record = record
        details = []
        if record is not None:
            details.append(f"containment_confirmed={record['containment_confirmed']}, retired={record['retired']}")
            if record['error'] is not None:
                details.append('primary: ' + record['error'])
            if record['retirement_error']:
                details.append('retirement: ' + record['retirement_error'])
            details.extend('reporting (' + item['stage'] + '): ' + item['error']
                           for item in record['reporting_errors'])
        super().__init__('; '.join([message] + details))


class ContainmentError(SessionError):
    pass


class Sessions:
    """One existing supervisor per connection; mutable session state is never shared."""
    capacity = 4

    def __init__(self, run):
        self.run = run
        self.threads = []  # admission/reaping belong exclusively to the listener
        self.blocked = threading.Event()

    def reap(self):
        for thread in self.threads:
            if not thread.is_alive():
                thread.join()
        self.threads = [thread for thread in self.threads if thread.is_alive()]

    def admit(self, peer):
        self.reap()
        if self.blocked.is_set() or len(self.threads) >= self.capacity:
            peer.close()
            return False
        def owned():
            try:
                self.run(peer)
            except ContainmentError as error:
                # Refuse NEW work when containment is uncertain. Existing healthy
                # siblings keep their own lifetime and supervisor.
                self.blocked.set()
                print(str(error), file=sys.stderr, flush=True)
            except Exception as error:
                print('preview instance failed: ' + str(error), file=sys.stderr, flush=True)
            finally:
                peer.close()
        thread = threading.Thread(target=owned, name='ap5-preview-instance')
        self.threads.append(thread)
        thread.start()
        return True

    def join(self):
        for thread in self.threads:
            thread.join()
        self.threads.clear()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host-root', type=pathlib.Path, required=True)
    parser.add_argument('--build-input', required=True)
    args = parser.parse_args()
    os.chdir(ROOT)
    os.umask(0o077)
    def verify_host(root, identity):
        return artifacts.verify_host_store(root, identity,
            expected_branch='codex/ap4-plugin-state-project-recall', expected_input_count=24)
    host = verify_host(args.host_root, args.build_input)
    fixture = artifacts.verify_fixture_store(dx0_deck_fixture_parent() / DX0_AGAIN_BUNDLE_MANIFEST_SHA256)
    runner = profile.verify_runtime()['launch_critical_manifest_sha256']
    source = host['build_receipt']['producer_source']
    def create():
        return runtime.create_dx0_environment(secrets.token_hex(16), host=host, fixture=fixture,
            execution_source=source, deck_execution_input_sha256=args.build_input,
            runner_identity_sha256=runner, verify_host=verify_host)
    def retire(environment):
        runtime.retire_environment(environment, runner_identity_sha256=runner)

    root = real_home() / 'AP4-State-Test/preview'
    private_directory(root)
    output = root / 'results'
    private_directory(output)
    address = root / 'owner.sock'
    stopping = threading.Event()
    def stop(signum, frame):
        stopping.set()
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    # Refuse an existing socket, including a stale one; never steal ownership.
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.bind(str(address))
        inode = address.stat().st_ino
        listener.listen(8)
        listener.setblocking(False)
        sessions = Sessions(lambda peer: serve(peer, create, retire, output, stopping.is_set))
        try:
            print('AP5 preview ready for normal desktop launch (capacity 4)', flush=True)
            while not stopping.is_set():
                sessions.reap()
                try:
                    peer, _ = listener.accept()
                except BlockingIOError:
                    time.sleep(.05)
                    continue
                sessions.admit(peer)
        finally:
            stopping.set()
            sessions.join()
            if address.is_socket() and address.stat().st_ino == inode:
                address.unlink()


if __name__ == '__main__':
    main()
