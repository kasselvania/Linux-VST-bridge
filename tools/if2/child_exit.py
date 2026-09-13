#!/usr/bin/env python3
"""One exact candidate-16 Windows-child fault; private evidence, no desktop input.

PID/start and pidfd bind the signal. Candidate admission, exact mapped image,
random session arguments and ancestry to the installed session supervisor are
all required. No caller-supplied PID, command, image identity or signal.
"""
import argparse
import os
from pathlib import Path
import re
import signal
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]/'uio1'))
from launch import Context, private_json

CLASS = '41727475415649534B61743150726F63'
PROFILE = '3199611a7578f751a17fe9e0d8f20aa12a27ec1b296f4de2630f7cb7738ab9dc'


def bounded(path, limit):
    with open(path, 'rb') as f:
        data = f.read(limit+1)
    if len(data) > limit:
        raise RuntimeError('process observation capacity')
    return data


def mapped(pid, image):
    # /proc escapes whitespace in paths. Compare exact resolved image paths;
    # never match a basename or friendly product/window name.
    text = bounded(Path('/proc')/str(pid)/'maps', 4*1024*1024).decode()
    target = str(Path(image).resolve(strict=True))
    for line in text.splitlines():
        fields = line.split(None, 5)
        if len(fields) == 6:
            path = re.sub(r'\\([0-7]{3})', lambda m: chr(int(m[1], 8)), fields[5])
            if path == target:
                return True
    return False


def descendant(pid, ancestor, records):
    seen = set()
    while pid in records and pid not in seen and len(seen) < 64:
        if pid == ancestor:
            return True
        seen.add(pid)
        pid = records[pid]['ppid']
    return False


def select(records, supervisor, session, image, cmdlines, maps):
    owners = [r for r in records.values() if cmdlines.get(r['pid']) == supervisor]
    if len(owners) != 1:
        raise RuntimeError('one exact installed session supervisor required')
    owner = owners[0]
    matches = []
    for pid, args in cmdlines.items():
        pairs = list(zip(args, args[1:]))
        if ('--session', session) not in pairs or ('--scanner-sha256', image) not in pairs:
            continue
        if not maps.get(pid, False) or not descendant(pid, owner['pid'], records):
            continue
        matches.append(records[pid])
    if len(matches) != 1:
        raise RuntimeError('one mapped Windows image in the exact supervisor tree required')
    return owner, matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--admission-binary', type=Path, required=True)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    args.output.mkdir(mode=0o700)
    context = Context(args.admission_binary, CLASS, args.package, if1=True)
    try:
        if context.admission['profile_fingerprint'] != PROFILE:
            raise RuntimeError('exact candidate 16 required')
        before = context.fault.snapshot()
        if before.get('terminal_instance') or not before.get('editor', {}).get('open'):
            raise RuntimeError('healthy open editor required before the one fault')
        for artifact in (context.reg['native'], context.reg['host']):
            context.runtime.verify(artifact)
        records = {r['pid']: r for r in context.runtime.process_identities()}
        if len(records) > 8192:
            raise RuntimeError('process census capacity')
        cmdlines, images, natives = {}, {}, []
        for pid in records:
            try:
                root = Path('/proc')/str(pid)
                if root.stat().st_uid != os.getuid():
                    continue
                cmdlines[pid] = bounded(root/'cmdline', 65536).decode().rstrip('\0').split('\0')
                images[pid] = mapped(pid, context.reg['host']['path'])
                if mapped(pid, context.reg['native']['path']):
                    natives.append(records[pid])
            except (OSError, UnicodeError):
                continue
        if len(natives) != 1:
            raise RuntimeError('one native processor host required')
        native = natives[0]
        spec = context.prefix/'drive_c/bridge/sessions'/context.session['session']/'owner.json'
        supervisor = ['/usr/bin/python3', context.software['supervisor']['path'], str(spec)]
        owner, child = select(records, supervisor, context.session['session'],
                              context.reg['host']['sha256'], cmdlines, images)
        if child['pid'] == native['pid']:
            raise RuntimeError('Windows and native owners must be distinct')
        private_json(args.output/'before.json', dict(admission=context.admission,
            session=context.session['session'], fault=before, supervisor=owner,
            windows=child, native=native))
        fd = os.pidfd_open(child['pid'])
        try:
            fresh = {r['pid']: r for r in context.runtime.process_identities()}
            if fresh.get(child['pid']) != child or fresh.get(owner['pid']) != owner or fresh.get(native['pid']) != native:
                raise RuntimeError('process identity changed before signal')
            if not mapped(child['pid'], context.reg['host']['path']):
                raise RuntimeError('Windows image changed before signal')
            issued = time.monotonic_ns()
            signal.pidfd_send_signal(fd, signal.SIGKILL)
            private_json(args.output/'signal.json', dict(kind='SIGKILL', issued_monotonic_ns=issued,
                windows=child, pidfd_bound=True))
        finally:
            os.close(fd)
        deadline = time.monotonic()+30
        result = {}
        while time.monotonic() < deadline:
            snapshot = context.fault.snapshot()
            current = {r['pid']: r for r in context.runtime.process_identities()}
            same = current.get(native['pid']) == native
            result = dict(fault=snapshot, native_same_identity=same,
                          windows_absent=child['pid'] not in current,
                          sample_monotonic_ns=time.monotonic_ns())
            if not same or (result['windows_absent'] and snapshot.get('terminal_instance')):
                break
            time.sleep(.05)
        private_json(args.output/'after.json', result)
        if not result.get('native_same_identity') or not result.get('windows_absent') or not result.get('fault', {}).get('terminal_instance'):
            raise RuntimeError('terminal custody/native survival not established')
        print('Exact Windows-child fault committed; native owner survives. Editor presentation and removal remain separate checks.')
    finally:
        context.fault.close()


if __name__ == '__main__':
    main()
