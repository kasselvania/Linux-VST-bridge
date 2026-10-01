#!/usr/bin/env python3
"""Exercise real installed first-party proxies; never edits product authority.

Run as the fixture user with the independent SDK host and callback audit library.
This is development regression coverage, not frontend or DAW project acceptance.
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import socket
import struct
import subprocess
import time
from datetime import datetime, timezone

FIXTURES = {
    'effect': ('4C564242455441314546465800000001',
               'b3498eb5c8d6d440259b04a9c9b7699ab72f4f64eb31f45167f0341d3d1e719f'),
    'instrument': ('4C56424245544131494E535400000001',
                   '2a117a128c0e84f53c92c4b1d977325f8eb198c1221497677dd3fec395817462'),
}
# Native class identities are distinct from Windows class identities. These
# exact generated identities are independently checked at the SDK factory.
NATIVE_IDS = {
    'effect': ('1A954DE265CF529E87C53ED21FCB2598', '824887B5E350572388EB7E70A604D331'),
    'instrument': ('AE1F229A50065EBDBB450D0F0E26D983', 'C7FBEFACA44154768C5E18284DDEFF77'),
}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--host', type=pathlib.Path, required=True)
parser.add_argument('--audit', type=pathlib.Path, required=True)
parser.add_argument('--role', choices=FIXTURES)
parser.add_argument('--pairs', type=int, choices=(1, 4), default=4,
                    help='One initial probe or the complete predeclared four-pair workload')
args = parser.parse_args()
root = pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
os.umask(0o077)
args.output.mkdir(mode=0o700, parents=True, exist_ok=False)


class CapacityUnavailable(Exception):
    pass


def read(path):
    assert path.stat().st_size <= 8*1024*1024, 'record extent'
    return json.loads(path.read_text())


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def receive(peer, size):
    value = b''
    while len(value) < size:
        more = peer.recv(size-len(value))
        assert more, 'incomplete service readback'
        value += more
    return value


def lease_observations():
    """Bounded ownership observations, without exporting paths or session data."""
    leases = list((root/'runtime/leases').glob('*.json'))
    environments = list((root/'environments').iterdir())
    assert len(leases) <= 64 and len(environments) <= 128, 'ownership extent'
    rows = []
    for lease in leases:
        sid = lease.stem
        assert re.fullmatch(r'[a-f0-9]{32}', sid), 'lease identity'
        row = {'session':sid}
        try:
            report = pathlib.Path(read(lease))
            assert report.parent == root/'runtime/results', 'report identity'
            owners = [env/'compatdata/pfx/drive_c/bridge/sessions'/sid/'owner.json'
                      for env in environments]
            row['temporary_owner_count'] = sum(p.exists() for p in owners)
            row['report_exists'] = report.exists()
            if row['report_exists']:
                result = read(report)
                row.update({key:result.get(key) for key in
                            ('cleanup_confirmed', 'raw_exit', 'transport_retired')})
            row['lease_still_exists'] = lease.exists()
        except FileNotFoundError:
            row['lease_still_exists'] = lease.exists()
        rows.append(row)
    return rows


def capacity():
    with socket.socket(socket.AF_UNIX) as peer:
        peer.settimeout(5)
        peer.connect(str(root/'runtime/owner.sock'))
        peer.sendall(b'LVC1\n')
        size, = struct.unpack('<I', receive(peer, 4))
        assert size <= 65536, 'capacity extent'
        value = json.loads(receive(peer, size))
        if value.get('ok') is not True:
            refusal = value.get('refusal')
            code = refusal.get('code', 'unclassified') if isinstance(refusal, dict) else str(refusal)
            if not code or len(code) > 96 or not all(c.isalnum() or c in '_-' for c in code):
                code = 'unclassified'
            detail = refusal.get('detail') if isinstance(refusal, dict) else None
            if detail not in ('active_device_lease', 'active_lease_unresolved'):
                detail = 'not_exported'
            owners = []
            inode = (root/'registry.lock').stat().st_ino
            for line in pathlib.Path('/proc/locks').read_text().splitlines():
                fields = line.split()
                if len(fields) >= 8 and fields[1] != '->' and fields[5].endswith(':'+str(inode)):
                    owners.append(int(fields[4]))
            emit('capacity_unavailable', refusal_code=code, refusal_detail=detail,
                 sampled_registry_owner_pids=owners, lease_observations=lease_observations())
            raise CapacityUnavailable(code)
        return {k:value['capacity'][k] for k in
                ('dsp', 'maintenance', 'keepers', 'cleanup_unconfirmed')}


def idle():
    value = capacity()
    assert value['dsp'] == value['maintenance'] == 0, 'existing work must finish'
    assert not value['cleanup_unconfirmed'], 'unconfirmed cleanup'
    assert not (root/'operator/resume.json').exists(), 'existing recovery must finish'
    return value


def session_reports():
    return {p for p in (root/'runtime/results').glob('windows-*.json')
            if re.fullmatch(r'windows-[a-f0-9]{32}\.json', p.name)}


def emit(event, **fields):
    value = dict(event=event, utc=datetime.now(timezone.utc).isoformat(),
                 monotonic_ns=time.monotonic_ns(), **fields)
    print(json.dumps(value), flush=True)
    with (args.output/'result.ndjson').open('a') as stream:
        stream.write(json.dumps(value)+'\n')


def sanitized_phases(stderr):
    phases = []
    for line in stderr.splitlines():
        try:
            value = json.loads(line)
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        if value.get('event') != 'ap7_audio_phase':
            continue
        if value.get('phase') not in ('processing_ready', 'processing_stopped',
                                      'deactivated', 'closing', 'failed',
                                      'retired', 'retirement_unconfirmed'):
            continue
        row = {k:value[k] for k in ('event', 'schema', 'phase', 'epoch',
                                    'monotonic_ns', 'omitted_phase_markers') if k in value}
        for phase in ('startup', 'processing'):
            row[phase] = {k:v for k,v in value.get(phase, {}).items()
                          if k in ('admitted_frames', 'missing_frames', 'gaps',
                                   'expired_frames', 'delivered_frames', 'priming_frames')
                          and isinstance(v, int)}
        phases.append(row)
    return phases


software = read(root/'software.json')
registry = read(root/'registry.json')
for key in ('manager', 'host'):
    artifact = software[key]
    assert sha(pathlib.Path(artifact['path'])) == artifact['sha256'], 'selected artifact changed'
emit('candidate', manager_sha256=software['manager']['sha256'],
     windows_host_sha256=software['host']['sha256'],
     sdk_consumer_sha256=sha(args.host), audit_sha256=sha(args.audit),
     claim='installed_development_regression', real_daw_project_claim=False)
environment = os.environ.copy()
for line in subprocess.check_output(['systemctl', '--user', 'show-environment'],
                                    text=True, timeout=5).splitlines():
    key, _, value = line.partition('=')
    if key in ('DISPLAY', 'XAUTHORITY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS'):
        environment[key] = value
environment['LD_PRELOAD'] = str(args.audit.resolve(strict=True))
roles = [args.role] if args.role else list(FIXTURES)
for role in roles:
    key, module_sha = FIXTURES[role]
    entry = registry['classes'][key]
    registration = entry['registration']
    assert entry['publication'] == 'Published', 'fixture must be published normally'
    assert registration['module']['sha256'] == module_sha, 'exact fixture module required'
    assert registration['host']['sha256'] == software['host']['sha256'], 'replace the predecessor through product controls'
    for name in ('module', 'host', 'native'):
        artifact = registration[name]
        assert sha(pathlib.Path(artifact['path'])) == artifact['sha256'], 'fixture artifact changed'
    performance = read(root/'performance'/(key+'.json'))
    assert performance == {'schema':1, 'added_frames':1024}, 'predeclared 1024-frame workload required'
    bundle = pathlib.Path.home()/'.vst3'/('LVB_'+key+'.vst3')
    assert bundle.is_dir() and registration['native']['sha256'] == sha(
        bundle/'Contents/x86_64-linux'/('LVB_'+key+'.so')), 'published native identity differs'
    emit('publication', role=role, class_id=key, module_sha256=module_sha,
         native_sha256=registration['native']['sha256'], added_frames=1024)
    for pair, frames in enumerate((1024, 1024, 1024, 1008)[:args.pairs], 1):
        prefix = args.output/(role+'-'+str(pair))
        for mode in ('record', 'recall'):
            before = idle()
            known = session_reports()
            started = time.monotonic()
            child = subprocess.Popen([str(args.host), str(bundle), role, mode,
                                      str(prefix), str(frames), *NATIVE_IDS[role]], env=environment,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                stdout, stderr = child.communicate(timeout=95)
            except subprocess.TimeoutExpired:
                # Retire this test-owned consumer only. Product DSP retirement
                # must still be confirmed; this can never count as a pass.
                child.terminate()
                stdout, stderr = child.communicate(timeout=10)
                emit('test_timeout', role=role, pair=pair, mode=mode,
                     intervention='terminated_test_owned_consumer')
            events = []
            for line in stdout.splitlines():
                if line.strip():
                    events.append(json.loads(line))
            attempted = any(e['event'] in ('initialized_state', 'activated', 'host_retired')
                            for e in events)
            reports = []
            retirement_readback_failed = False
            retirement_observations = []
            until = time.monotonic()+20
            while time.monotonic() < until:
                reports = []
                for path in session_reports()-known:
                    report = read(path)
                    reports.append({k:report.get(k) for k in
                                    ('session', 'cleanup_confirmed', 'transport_retired', 'gated', 'raw_exit')})
                remaining = [r['session'] for r in reports
                             if (root/'runtime/leases'/(r['session']+'.json')).exists()]
                if reports and not remaining and all(r['cleanup_confirmed'] is True
                        and r['transport_retired'] is True for r in reports):
                    break
                if not attempted and child.returncode != 0:
                    break
                if reports and not retirement_readback_failed:
                    try:
                        observed = capacity()
                        row = {'leases_remaining':remaining, 'capacity':observed}
                        if not retirement_observations or row != retirement_observations[-1]:
                            assert len(retirement_observations) < 64, 'retirement observation extent'
                            retirement_observations.append(row)
                    except (CapacityUnavailable, OSError, AssertionError):
                        retirement_readback_failed = True
                time.sleep(.1)
            phases = []
            for report in reports:
                diagnostic = root/'runtime/results'/('native-'+report['session']+'.jsonl')
                if diagnostic.exists():
                    assert diagnostic.stat().st_size <= 131072, 'native diagnostic extent'
                    for phase in sanitized_phases(diagnostic.read_bytes()):
                        phases.append(dict(session=report['session'], **phase))
            try:
                after = capacity()
            except (CapacityUnavailable, OSError, AssertionError):
                # Keep the actual consumer and retirement results even when
                # global readback fails. The required readback still fails the run.
                after = {'unavailable':True}
            emit('run', role=role, pair=pair, mode=mode, frames=frames,
                 duration_ms=round((time.monotonic()-started)*1000, 3),
                 exit_code=child.returncode, host_events=events, audio_phases=phases,
                 dsp_retirement=reports, capacity_before=before, capacity_after=after,
                 retirement_readback_failed=retirement_readback_failed,
                 retirement_observations=retirement_observations,
                 omitted_stderr_bytes=len(stderr), stderr_sha256=hashlib.sha256(stderr).hexdigest())
            assert child.returncode == 0 and events[-1]['event'] == 'passed', 'installed lifecycle failed'
            assert reports and not remaining and all(r['cleanup_confirmed'] is True
                and r['transport_retired'] is True for r in reports), 'positive retirement missing'
            assert not retirement_readback_failed, 'retirement capacity readback refused'
            assert after.get('dsp') == 0 and after.get('cleanup_unconfirmed') is False, 'fresh capacity readback missing'
            assert phases, 'phase attribution missing'
            for report in reports:
                session_phases = [p for p in phases if p['session'] == report['session']]
                assert session_phases and session_phases[-1]['phase'] == 'retired', 'native retirement missing'
            assert all(p.get('processing', {}).get('missing_frames', 0) == 0 for p in phases), 'processing audio gap'
        emit('state_roundtrip', role=role, pair=pair, frames=frames,
             component_sha256=sha(prefix.with_suffix('.component')),
             controller_sha256=sha(prefix.with_suffix('.controller')))
emit('suite_passed' if args.role is None and args.pairs == 4 else 'subset_passed',
     roles=roles, pairs=args.pairs, claim='installed_development_regression',
     real_daw_project_claim=False, maintainer_repairs=0)
