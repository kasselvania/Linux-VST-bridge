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
import selectors
import socket
import struct
import subprocess
import time
import uuid
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
parser.add_argument('--unfamiliar-fixtures', type=pathlib.Path,
                    help='Post-freeze first-party fixture artifact directory')
parser.add_argument('--revision', choices=('1.0.1', '1.0.2', '1.0.3', '1.0.4', '1.0.5'))
parser.add_argument('--frozen-candidate', type=pathlib.Path,
                    help='Retained package, freeze receipt and staged release manifest')
parser.add_argument('--selected-candidate', type=pathlib.Path,
                    help='Explicit later package selection; the original frozen engine must remain unchanged')
parser.add_argument('--publication-candidate', type=pathlib.Path,
                    help='Exact retained package supplying a restored predecessor engine; selected manager stays separately verified')
parser.add_argument('--recall-from', type=pathlib.Path,
                    help='Recall the exact saved states from a previously passing one-pair run')
parser.add_argument('--state-update', action='store_true',
                    help='Verify an explicitly frozen successor engine for state repair; not the original engine-generalization claim')
parser.add_argument('--restore-mode', choices=('recall','recall-disconnected','migrate','migrate-disconnected'), default='recall')
parser.add_argument('--expect-restore-refusal', action='store_true')
parser.add_argument('--invalid-state', choices=('wrong-class','corrupt','oversized'))
parser.add_argument('--abrupt-exit', action='store_true')
parser.add_argument('--sibling-module-sha256', help='Keep the other exact first-party class processing during a failure test')
parser.add_argument('--pairs', type=int, choices=(1, 4), default=4,
                    help='One initial probe or the complete predeclared four-pair workload')
args = parser.parse_args()
assert not args.state_update or (args.selected_candidate and args.pairs==1), 'exact bounded successor package required'
assert not args.publication_candidate or (args.state_update and args.recall_from and not args.expect_restore_refusal and not args.abrupt_exit), 'retained engine is only an explicit predecessor recall comparison'
assert args.restore_mode=='recall' or (args.state_update and args.recall_from), 'explicit state-update recall required'
assert not args.expect_restore_refusal or (args.recall_from and args.state_update)
assert not args.invalid_state or args.expect_restore_refusal
assert not args.abrupt_exit or (args.state_update and not args.recall_from)
assert not (args.expect_restore_refusal and args.restore_mode.startswith('migrate'))
assert not args.sibling_module_sha256 or (args.role and args.state_update and (args.expect_restore_refusal or args.abrupt_exit) and re.fullmatch('[0-9a-f]{64}',args.sibling_module_sha256))

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


def frozen_candidate(directory):
    frozen = read(directory/'FROZEN_CANDIDATE.json')
    assert frozen['schema'] == 1
    package_name, = [name for name in frozen['files']
                     if re.fullmatch(r'linux-vst-bridge-beta_[0-9A-Za-z.]+-1_amd64\.deb', name)]
    assert sha(directory/package_name) == frozen['files'][package_name], 'retained package changed'
    release_name, = [name for name in frozen['files']
                     if re.fullmatch(r'staged-[0-9A-Za-z]+/RELEASE_MANIFEST\.json', name)]
    release_path = directory/release_name
    assert sha(release_path) == frozen['files'][release_name]
    release = read(release_path)
    assert release['source_head'] == frozen['source_head'] and release['source_tree'] == frozen['source_tree']
    return frozen, frozen['files'][package_name], {item['destination']:item['sha256'] for item in release['files']}


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


def idle(expected_dsp=0):
    value = capacity()
    assert value['dsp'] == expected_dsp and value['maintenance'] == 0, 'existing work must finish'
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
unfamiliar = None
if args.unfamiliar_fixtures is not None:
    assert args.revision and args.frozen_candidate, 'exact unfamiliar revision and frozen candidate required'
    unfamiliar = read(args.unfamiliar_fixtures/'UNFAMILIAR_FIXTURES.json')
    frozen, package_sha, roster = frozen_candidate(args.frozen_candidate)
    assert unfamiliar['schema'] == 1
    assert unfamiliar['classification'] == 'first_party_test_instrumentation'
    assert package_sha == unfamiliar['frozen_package_sha256']
    assert unfamiliar['frozen_engine_sha256'] == frozen['engine_sha256']
    selected = frozen
    if args.selected_candidate is not None:
        selected, package_sha, roster = frozen_candidate(args.selected_candidate)
        if not args.state_update:
            assert selected['engine_sha256'] == frozen['engine_sha256'], 'package update changed the frozen engine'
    assert roster['usr/lib/linux-vst-bridge/proxy/ReusableEngine.so'] == selected['engine_sha256']
    publication = selected
    publication_package_sha = package_sha
    if args.publication_candidate is not None:
        publication, publication_package_sha, publication_roster = frozen_candidate(args.publication_candidate)
        assert publication_roster['usr/lib/linux-vst-bridge/proxy/ReusableEngine.so'] == publication['engine_sha256']
    for key, destination in (('manager', 'usr/bin/linux-vst-bridge'),
                             ('host', 'usr/lib/linux-vst-bridge/host/bridge-host.exe')):
        assert software[key]['sha256'] == roster[destination], 'selected software differs from declared package'
    revision, = [row for row in unfamiliar['revisions'] if row['version'] == args.revision]
    assert len(revision['modules']) == 2 and {row['role'] for row in revision['modules']} == set(FIXTURES)
    namespace = uuid.UUID('9389480f-b4b0-5e02-a1d7-687a57b54b3f')
    for module in revision['modules']:
        role, key = module['role'], module['processor_class']
        assert re.fullmatch('[0-9A-F]{32}', key) and re.fullmatch('[0-9a-f]{64}', module['sha256'])
        assert pathlib.PurePosixPath(module['file']).parts == (args.revision, f'lvb-reference-{role}.vst3')
        assert sha(args.unfamiliar_fixtures/module['file']) == module['sha256'], 'generated fixture bytes changed'
        FIXTURES[role] = (key, module['sha256'])
        NATIVE_IDS[role] = tuple(uuid.uuid5(namespace, key+suffix).hex.upper()
                                 for suffix in (':processor', ':controller'))
    emit('unfamiliar_fixture', revision=args.revision,
         generator_source_head=unfamiliar['generator_source_head'],
         frozen_package_sha256=unfamiliar['frozen_package_sha256'],
         frozen_engine_sha256=unfamiliar['frozen_engine_sha256'],
         selected_package_sha256=package_sha, selected_source_head=selected['source_head'],
         selected_engine_sha256=selected['engine_sha256'], state_update=args.state_update,
         publication_package_sha256=publication_package_sha,
         publication_source_head=publication['source_head'], publication_engine_sha256=publication['engine_sha256'])
else:
    assert args.revision is None and args.frozen_candidate is None and args.selected_candidate is None, 'unfamiliar fixture directory required'
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
class HealthySibling:
    def __init__(self,role):
        self.role=role;self.child=None;self.events=[];self.sessions=set()
    def __enter__(self):
        if not args.sibling_module_sha256:return self
        idle()
        key,_=FIXTURES[self.role]
        entry=registry['classes'][key];registration=entry['registration']
        assert entry['publication']=='Published' and registration['module']['sha256']==args.sibling_module_sha256
        assert registration['native']['sha256']==publication['engine_sha256'], 'sibling must use the declared frozen engine'
        for name in ('module','native','host'):
            artifact=registration[name]
            assert sha(pathlib.Path(artifact['path']))==artifact['sha256']
        bundle=pathlib.Path.home()/'.vst3'/('LVB_'+key+'.vst3')
        before={p.stem for p in (root/'runtime/leases').glob('*.json')}
        command=[str(args.host),str(bundle),self.role,'sibling',str(args.output/'sibling-state'),'1024',*NATIVE_IDS[self.role]]
        self.child=subprocess.Popen(command,env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0)
        try:
            with selectors.DefaultSelector() as poll:
                poll.register(self.child.stdout,selectors.EVENT_READ)
                until=time.monotonic()+60
                while time.monotonic()<until:
                    if not poll.select(.2):continue
                    line=self.child.stdout.readline()
                    assert line, 'sibling exited before processing'
                    row=json.loads(line);self.events.append(row)
                    if row['event']=='processing_state':break
                else:raise TimeoutError('sibling processing readiness')
            self.sessions={p.stem for p in (root/'runtime/leases').glob('*.json')}-before
            assert len(self.sessions)==1 and self.child.poll() is None, 'one live exact sibling required'
            idle(1)
            emit('sibling_processing',role=self.role,class_id=key,module_sha256=args.sibling_module_sha256,
                 native_sha256=registration['native']['sha256'],sessions=sorted(self.sessions))
            return self
        except BaseException:
            self.stop();raise
    def stop(self):
        self.child.terminate()
        try:return self.child.communicate(timeout=15)
        except subprocess.TimeoutExpired:
            self.child.kill()
            return self.child.communicate(timeout=5)
    def __exit__(self,kind,value,trace):
        if self.child is None:return False
        try:
            try:stdout,stderr=self.child.communicate(timeout=80)
            except subprocess.TimeoutExpired:
                stdout,stderr=self.stop()
                raise AssertionError('sibling timed out')
            self.events += [json.loads(line) for line in stdout.splitlines() if line.strip()]
            emit('sibling_result',role=self.role,exit_code=self.child.returncode,host_events=self.events,
                 omitted_stderr_bytes=len(stderr),stderr_sha256=hashlib.sha256(stderr).hexdigest())
            assert self.child.returncode==0 and self.events[-1]['event']=='passed', 'healthy sibling audio/state failed'
            until=time.monotonic()+20
            while time.monotonic()<until:
                if not any((root/'runtime/leases'/(sid+'.json')).exists() for sid in self.sessions):break
                time.sleep(.1)
            for sid in self.sessions:
                report=read(root/'runtime/results'/('windows-'+sid+'.json'))
                assert report['cleanup_confirmed'] and report['transport_retired']
                assert not (root/'runtime/leases'/(sid+'.json')).exists()
            idle()
        except BaseException as error:
            emit('sibling_failure',reason=str(error))
            if kind is None:raise
        return False

roles = [args.role] if args.role else list(FIXTURES)
saved = None
if args.recall_from is not None:
    assert args.pairs == 1 and unfamiliar is not None, 'bounded unfamiliar update recall only'
    log = args.recall_from/'result.ndjson'
    assert log.stat().st_size <= 8*1024*1024, 'prior result extent'
    saved = [json.loads(line) for line in log.read_text().splitlines()]
    assert saved[-1]['event'] == 'subset_passed' and saved[-1]['pairs'] == 1
    assert all(role in saved[-1]['roles'] for role in roles), 'prior role did not pass'
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
    if unfamiliar is not None:
        assert registration['native']['sha256'] == publication['engine_sha256'], 'publication differs from the declared retained engine'
        artifact = registration['descriptor']
        assert sha(pathlib.Path(artifact['path'])) == artifact['sha256'], 'prepared data changed'
        descriptor = read(pathlib.Path(artifact['path']))
        assert descriptor['class_id'] == key and descriptor['module_sha256'] == module_sha
        assert descriptor['version'] == args.revision and descriptor['engine_sha256'] == publication['engine_sha256']
    performance = read(root/'performance'/(key+'.json'))
    assert performance == {'schema':1, 'added_frames':1024}, 'predeclared 1024-frame workload required'
    bundle = pathlib.Path.home()/'.vst3'/('LVB_'+key+'.vst3')
    assert bundle.is_dir() and registration['native']['sha256'] == sha(
        bundle/'Contents/x86_64-linux'/('LVB_'+key+'.so')), 'published native identity differs'
    emit('publication', role=role, class_id=key, module_sha256=module_sha,
         native_sha256=registration['native']['sha256'], added_frames=1024,
         retained_revision=entry.get('managed_revision'))
    for pair, frames in enumerate((1024, 1024, 1024, 1008)[:args.pairs], 1):
        prefix = args.output/(role+'-'+str(pair))
        if saved is not None:
            prior, = [row for row in saved if row['event'] == 'publication' and row['role'] == role]
            assert prior['class_id'] == key
            if not args.state_update:
                assert prior['native_sha256'] == registration['native']['sha256']
            state, = [row for row in saved if row['event'] == 'state_roundtrip' and row['role'] == role and row['pair'] == pair]
            prefix = args.recall_from/(role+'-'+str(pair))
            for part in ('component', 'controller'):
                assert sha(prefix.with_suffix('.'+part)) == state[part+'_sha256'], 'prior saved state changed'
            emit('prior_state_recall', role=role, from_module_sha256=prior['module_sha256'],
                 to_module_sha256=module_sha, component_sha256=state['component_sha256'],
                 controller_sha256=state['controller_sha256'])
        original_prefix=prefix
        original_hashes={part:sha(prefix.with_suffix('.'+part)) for part in ('component','controller')} if saved is not None else None
        if args.invalid_state:
            damaged=args.output/(role+'-invalid')
            component=bytearray(prefix.with_suffix('.component').read_bytes())
            if args.invalid_state=='wrong-class':component[16]^=1
            elif args.invalid_state=='corrupt':component[-1]^=1
            else:component[64:68]=(1024*1024+1).to_bytes(4,'little')
            damaged.with_suffix('.component').write_bytes(component)
            damaged.with_suffix('.controller').write_bytes(prefix.with_suffix('.controller').read_bytes())
            prefix=damaged
        with HealthySibling('instrument' if role=='effect' else 'effect') as sibling:
            modes=('abrupt',) if args.abrupt_exit else ((args.restore_mode,'recall') if args.restore_mode.startswith('migrate') else (args.restore_mode,)) if saved is not None else ('record','recall')
            for mode in modes:
                before = idle(1 if sibling.child else 0)
                known = session_reports()
                started = time.monotonic()
                command=[str(args.host),str(bundle),role,mode,str(prefix),str(frames),*NATIVE_IDS[role]]
                migrated=args.output/(role+'-'+str(pair))
                if mode.startswith('migrate'):command.append(str(migrated))
                child = subprocess.Popen(command,env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
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
                        if path.stem.removeprefix('windows-') in sibling.sessions:continue
                        report = read(path)
                        reports.append({k:report.get(k) for k in
                                        ('session', 'cleanup_confirmed', 'transport_retired', 'gated', 'raw_exit','native_retirement_basis')})
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
                if args.abrupt_exit:
                    assert child.returncode==23 and events[-1]['event']=='abrupt_exit', 'declared abrupt loss missing'
                    assert not any(e['event']=='host_retired' for e in events), 'consumer unexpectedly performed SDK teardown'
                    assert any(r.get('native_retirement_basis')=='authenticated_process_generation_ended' for r in reports), 'independent native-death proof missing'
                elif args.expect_restore_refusal:
                    assert child.returncode==1 and events[-1]['event']=='failed' and events[-1]['stage']=='restore', 'explicit restore refusal missing'
                    assert any(e['event']=='host_retired' and e['module_unloaded'] for e in events), 'orderly SDK teardown missing'
                    assert not any(e['event'] in ('activated','audio') for e in events), 'failed restore must not substitute playable default state'
                else:
                    assert child.returncode == 0 and events[-1]['event'] == 'passed', 'installed lifecycle failed'
                assert reports and not remaining and all(r['cleanup_confirmed'] is True
                    and r['transport_retired'] is True for r in reports), 'positive retirement missing'
                assert not retirement_readback_failed, 'retirement capacity readback refused'
                assert after.get('dsp') == (1 if sibling.child else 0) and after.get('cleanup_unconfirmed') is False, 'fresh capacity readback missing'
                if sibling.child:assert sibling.child.poll() is None, 'sibling must remain processing through failure retirement'
                assert phases or args.abrupt_exit, 'phase attribution missing'
                for report in reports:
                    session_phases = [p for p in phases if p['session'] == report['session']]
                    if not args.abrupt_exit:
                        assert session_phases and session_phases[-1]['phase'] == 'retired', 'native retirement missing'
                assert all(p.get('processing', {}).get('missing_frames', 0) == 0 for p in phases), 'processing audio gap'
                if original_hashes:
                    assert all(sha(original_prefix.with_suffix('.'+part))==value for part,value in original_hashes.items()), 'original saved object changed'
                if mode.startswith('migrate'):
                    old=original_prefix.with_suffix('.component').read_bytes()
                    new=migrated.with_suffix('.component').read_bytes()
                    assert old[16:32]==new[16:32]==bytes.fromhex(key), 'logical class changed'
                    assert old[32:64]==bytes.fromhex(prior['module_sha256']), 'historical producing identity changed'
                    assert new[32:64]==bytes.fromhex(module_sha), 'new producing identity is false'
                    assert hashlib.sha256(new[104:]).digest()==new[72:104], 'new payload integrity'
                    emit('state_migration',role=role,old_module_sha256=old[32:64].hex(),new_module_sha256=new[32:64].hex(),
                         old_parameter_count=int.from_bytes(old[112:116],'little'),new_parameter_count=int.from_bytes(new[112:116],'little'),
                         old_component_bytes=int.from_bytes(old[104:108],'little'),new_component_bytes=int.from_bytes(new[104:108],'little'),
                         original_sha256=original_hashes['component'],migrated_sha256=sha(migrated.with_suffix('.component')))
                    prefix=migrated
        if args.abrupt_exit or args.expect_restore_refusal:continue
        emit('state_roundtrip', role=role, pair=pair, frames=frames,
             component_sha256=sha(prefix.with_suffix('.component')),
             controller_sha256=sha(prefix.with_suffix('.controller')))
emit('suite_passed' if args.role is None and args.pairs == 4 else 'subset_passed',
     roles=roles, pairs=args.pairs, claim='installed_development_regression',
     real_daw_project_claim=False, maintainer_repairs=0, state_update=args.state_update,
     expected_refusal=args.expect_restore_refusal, invalid_state=args.invalid_state, abrupt_exit=args.abrupt_exit)
