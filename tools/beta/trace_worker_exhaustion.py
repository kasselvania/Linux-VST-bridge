#!/usr/bin/env python3
"""Installed OS thread-denial regression; fixture user, idle owned service only.

Lowers only this service's temporary task limit, then restores its exact previous
limit. It never raises the enclosing resource budget or edits product authority.
Development fault injection, not a frontend qualification journey.
"""
import argparse
import hashlib
import json
import pathlib
import socket
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', required=True, type=pathlib.Path)
args = parser.parse_args()
root = pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
unit = 'linux-vst-bridge.service'
args.output.mkdir(mode=0o700, parents=True, exist_ok=False)


def control(*values):
    return subprocess.check_output(['systemctl', '--user', *values],
                                   text=True, timeout=45).strip()


def emit(event, **fields):
    value = dict(event=event, **fields)
    with (args.output/'result.ndjson').open('a') as stream:
        stream.write(json.dumps(value)+'\n')
    print(json.dumps(value), flush=True)


software = json.loads((root/'software.json').read_text())
manager = pathlib.Path(software['manager']['path'])
assert hashlib.file_digest(manager.open('rb'), 'sha256').hexdigest() == software['manager']['sha256']
capacity = json.loads(subprocess.check_output([str(manager), 'capacity'],
                                             text=True, timeout=10))
assert capacity['ok'] and capacity['capacity']['dsp'] == capacity['capacity']['maintenance'] == 0
assert not capacity['capacity']['cleanup_unconfirmed']
assert not (root/'operator/resume.json').exists()
previous = control('show', unit, '--property=TasksMax', '--value')
emit('fixture', manager_sha256=software['manager']['sha256'],
     previous_service_tasks_max=previous, claim='installed_development_fault_injection')
try:
    control('stop', unit)
    control('set-property', '--runtime', unit, 'TasksMax=1')
    assert control('show', unit, '--property=TasksMax', '--value') == '1'
    control('start', unit)
    pid = control('show', unit, '--property=MainPID', '--value')
    assert pid != '0'
    deadline = time.monotonic()+5
    for attempt in range(3):
        with socket.socket(socket.AF_UNIX) as peer:
            peer.settimeout(2)
            while True:
                try:
                    peer.connect(str(root/'runtime/owner.sock'))
                    break
                except (FileNotFoundError, ConnectionRefusedError):
                    assert time.monotonic() < deadline, 'owned service endpoint unavailable'
                    time.sleep(.02)
            peer.sendall(b'LVC1\n')
            reply = peer.recv(1025)
        assert 0 < len(reply) <= 1024, 'bounded explicit refusal required'
        incident = json.loads((root/'runtime/admission-incidents/service.json').read_text())
        assert incident['source'] == 'worker_unavailable'
        assert control('show', unit, '--property=MainPID', '--value') == pid
        assert control('show', unit, '--property=ActiveState', '--value') == 'active'
        assert not list((root/'runtime/leases').glob('*.json')), 'no admission or owner granted'
        emit('thread_denial_contained', attempt=attempt+1, manager_pid_unchanged=True,
             refusal_bytes=len(reply), incident_source=incident['source'],
             live_leases=0)
    emit('fault_case_passed', maintainer_repair=False)
    control('set-property', '--runtime', unit, 'TasksMax='+previous)
    assert control('show', unit, '--property=MainPID', '--value') == pid
    recovered = json.loads(subprocess.check_output([str(manager), 'capacity'],
                                                  text=True, timeout=10))
    assert recovered['ok'] and recovered['capacity']['current_workers'] == 1
    assert recovered['capacity']['dsp'] == recovered['capacity']['maintenance'] == 0
    emit('classifier_recovered_without_restart', worker_count_including_this_request=1,
         failed_spawn_worker_counts_released=True)
finally:
    control('stop', unit)
    control('set-property', '--runtime', unit, 'TasksMax='+previous)
    assert control('show', unit, '--property=TasksMax', '--value') == previous
    control('start', unit)
    emit('exact_service_limit_restored', tasks_max=previous,
         enclosing_cpu_memory_swap_and_task_caps_unchanged=True)
