#!/usr/bin/env python3
"""Replay an offered production refresh and retain bounded, sanitized timings.

Development instrumentation, not frontend qualification. Never alters registry,
leases, vendor state, deadlines, or installed binaries. Run as the fixture user.
"""
import argparse
import fcntl
import hashlib
import json
import os
import pathlib
import socket
import struct
import subprocess
import threading
import time
from datetime import datetime, timezone

parser = argparse.ArgumentParser()
parser.add_argument('--environment', required=True)
parser.add_argument('--capacity-traffic', action='store_true')
parser.add_argument('--restoration-only', action='store_true',
                    help='Restart the idle production service and replay its LVE1 crossing; no operator/front-end claim')
parser.add_argument('--registry-contention-ms', type=int, default=0,
                    help='Bounded development fault injection before LVE1; owns the lock and writes no data')
args = parser.parse_args()
assert 0 <= args.registry_contention_ms <= 5000
assert not args.registry_contention_ms or args.restoration_only
root = pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
started = time.monotonic()
output_lock = threading.Lock()
done = threading.Event()

def emit(event, **fields):
    with output_lock:
        print(json.dumps(dict(event=event, elapsed_ms=round((time.monotonic()-started)*1000, 3),
                              utc=datetime.now(timezone.utc).isoformat(), **fields)), flush=True)

def read(path):
    try:
        if path.stat().st_size > 8*1024*1024:
            raise ValueError('record exceeds bound')
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None

def receive(peer, size):
    value = b''
    while len(value) < size:
        more = peer.recv(size-len(value))
        if not more:
            raise ValueError('incomplete readback')
        value += more
    return value

def capacity():
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(5)
        peer.connect(str(root/'runtime/owner.sock'))
        peer.sendall(b'LVC1\n')
        size, = struct.unpack('<I', receive(peer, 4))
        if size > 65536:
            raise ValueError('capacity readback exceeds bound')
        value = json.loads(receive(peer, size))
        if value.get('ok') is not True:
            raise ValueError('capacity refused')
        value = value['capacity']
        return {key:value[key] for key in ['dsp','keepers','maintenance','cleanup_unconfirmed']}

def readers():
    while not done.is_set():
        before = time.monotonic()
        try:
            value = capacity()
            emit('capacity', duration_ms=round((time.monotonic()-before)*1000, 3), **value)
        except (OSError, ValueError, KeyError):
            emit('capacity_unavailable', duration_ms=round((time.monotonic()-before)*1000, 3))
        done.wait(1)

generations = set((root/'runtime/lease-generations').glob('*.json'))
reports = {p:p.stat().st_mtime_ns for p in (root/'runtime/results').glob('environment-*.json')}
last_resume = None
last_owners = None

def observe():
    global last_resume, last_owners
    resume = read(root/'operator/resume.json')
    owner = None if resume is None else resume.get('owner_operation')
    if owner != last_resume:
        emit('resume_owner', operation=owner, prior_service_active=None if resume is None else resume.get('resume'))
        last_resume = owner
    for path in (root/'runtime/lease-generations').glob('*.json'):
        if path not in generations:
            generations.add(path)
            emit('generation_created', session=path.stem, file_mtime_ns=path.stat().st_mtime_ns)
    for generation in generations:
        path = root/'runtime/results'/('environment-'+generation.stem+'.json')
        if not path.exists():
            continue
        stamp = path.stat().st_mtime_ns
        if reports.get(path) == stamp:
            continue
        reports[path] = stamp
        value = read(path)
        if value is not None:
            emit('keeper_report', session=generation.stem, file_mtime_ns=stamp,
                 ready=value.get('ready'), cleanup_confirmed=value.get('cleanup_confirmed'),
                 error_code=code(value.get('error')))
    inodes = {}
    for name in ['registry.lock','operator-environment.lock','operator-resume.lock']:
        path = root/name
        if path.exists():
            inodes[path.stat().st_ino] = name
    owners = []
    for line in pathlib.Path('/proc/locks').read_text().splitlines():
        fields = line.split()
        if len(fields) < 8 or fields[1] == '->':
            continue
        try:
            inode = int(fields[5].rsplit(':', 1)[-1])
            if inode in inodes:
                owners.append({'lock':inodes[inode], 'pid':int(fields[4])})
        except ValueError:
            continue
    owners.sort(key=lambda value:value['lock'])
    if owners != last_owners:
        emit('lock_owners', owners=owners, sampling_interval_ms=100)
        last_owners = owners

def code(value):
    if value is None:
        return None
    # Error classification only; never retain arbitrary private diagnostics.
    text = str(value)
    prefix = text.split(':', 1)[0].split(' ', 1)[0]
    return prefix if all(c.isalnum() or c in '_-' for c in prefix) else 'unclassified_error'

def monitor():
    while not done.is_set():
        observe()
        done.wait(.1)

software = read(root/'software.json')
manager = software['manager']['path']
for name in ['manager','host']:
    artifact = software[name]
    assert hashlib.sha256(pathlib.Path(artifact['path']).read_bytes()).hexdigest() == artifact['sha256']
emit('fixture', manager_sha256=software['manager']['sha256'], host_sha256=software['host']['sha256'],
     environment=args.environment, capacity_traffic=args.capacity_traffic,
     claim='installed_development_regression_not_frontend_qualification')
initial = capacity()
assert initial['dsp']==initial['maintenance']==0 and not initial['cleanup_unconfirmed']
assert not (root/'operator/resume.json').exists(), 'Existing recovery must finish first'
worker = threading.Thread(target=monitor)
worker.start()
reader = None
holder = None
try:
    if args.restoration_only:
        emit('idle_service_restart_begin', claim='development_test_arrangement')
        subprocess.run(['systemctl','--user','restart','linux-vst-bridge.service'],check=True,timeout=75)
        emit('idle_service_restart_returned')
        if args.capacity_traffic:
            reader = threading.Thread(target=readers)
            reader.start()
        deadline = time.monotonic()+5
        while True:
            peer = socket.socket(socket.AF_UNIX)
            try:
                peer.connect(str(root/'runtime/owner.sock'))
                break
            except OSError:
                peer.close()
                if time.monotonic()>=deadline:
                    raise
                time.sleep(.02)
        with peer:
            peer.settimeout(70)
            if args.registry_contention_ms:
                registry_lock = open(root/'registry.lock', 'r+')
                fcntl.flock(registry_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                emit('registry_contention_acquired', duration_ms=args.registry_contention_ms,
                     claim='controlled_negative_arrangement_not_original_failure_attribution')
                def release_lock():
                    time.sleep(args.registry_contention_ms/1000)
                    fcntl.flock(registry_lock, fcntl.LOCK_UN)
                    registry_lock.close()
                    emit('registry_contention_released')
                holder = threading.Thread(target=release_lock)
                holder.start()
            emit('restoration_request_begin')
            peer.sendall(b'LVE1\n')
            try:
                response = receive(peer,11)
                emit('restoration_acknowledgment', exact_ready=response==b'LVE1 ready\n')
                assert response==b'LVE1 ready\n', 'Exact readiness acknowledgment required'
            except (OSError,ValueError):
                emit('restoration_acknowledgment_failed')
                raise SystemExit(1)
        observe()
        emit('final', state='completed', resume_pending=(root/'operator/resume.json').exists(), capacity=capacity(),
             claim='production_restoration_crossing_not_operator_or_frontend_pass')
        raise SystemExit(0)
    emit('snapshot_begin')
    snapshot = json.loads(subprocess.check_output([manager,'operator','snapshot'], timeout=90))
    emit('snapshot_received')
    def offers(value):
        if isinstance(value, dict):
            if 'action' in value and 'disabled_reason' in value:
                yield value
            for child in value.values():
                yield from offers(child)
        elif isinstance(value, list):
            for child in value:
                yield from offers(child)
    target = {'kind':'environment_rescan','environment':args.environment}
    assert any(item['action']==target and item['disabled_reason'] is None for item in offers(snapshot)), 'No fresh enabled refresh offer'
    if args.capacity_traffic:
        reader = threading.Thread(target=readers)
        reader.start()
    request = {'schema':snapshot['schema'],'state_token':snapshot['state_token'],'action':target}
    emit('request_begin')
    receipt = json.loads(subprocess.check_output([manager,'operator','request'], input=json.dumps(request).encode(), timeout=90))
    emit('request_acknowledged', accepted=receipt['accepted'], operation=receipt['operation'], refusal_code=code(receipt.get('refusal')))
    assert receipt['accepted'], 'Production request refused'
    operation = receipt['operation']
    deadline = time.monotonic()+240
    last = None
    while time.monotonic() < deadline:
        result = read(root/'operator'/operation/'result.json')
        if result and result.get('state') != last:
            last = result.get('state')
            emit('operation_state', operation=operation, state=last, reason_code=code(result.get('reason')))
        if result and last in ['completed','refused','failed','interrupted']:
            observe()
            emit('final', operation=operation, state=last, resume_pending=(root/'operator/resume.json').exists(), capacity=capacity())
            if last != 'completed':
                raise SystemExit(1)
            break
        time.sleep(.1)
    else:
        emit('observation_deadline', operation=operation, claim='no_product_deadline_changed')
        raise SystemExit(2)
finally:
    done.set()
    worker.join()
    if reader:
        reader.join()
    if holder:
        holder.join()
