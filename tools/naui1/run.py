"""One offline retained-environment observation. No application-launch authority."""
import json
import os
import pathlib
import re
import stat
import subprocess
import sys
import time
from common import Refusal, canonical, decode, digest, file_identity, publish, read, require, directory as open_directory
from identity import discover
from records import parse
from classify import classify
from package import verify

KEYS = {'schema', 'installed_source', 'installed_tree', 'basis_head', 'basis_tree', 'environment', 'operation',
        'installer_sha256', 'installer_size', 'transaction_sha256', 'result_sha256', 'managed_relation',
        'software_sha256', 'installed_artifacts', 'preinstallation_sha256'}


def validate_manifest(m):
    require(set(m) == KEYS and type(m['schema']) is int and m['schema'] == 1, 'input_schema')
    require(m['managed_relation'] == '.local/share/linux-vst-bridge/managed', 'managed_relation')
    for k in ('installed_source', 'installed_tree', 'basis_head', 'basis_tree'):
        require(isinstance(m[k], str) and re.fullmatch('[0-9a-f]{40}', m[k]), 'source_identity')
    for k in ('environment', 'operation'):
        require(isinstance(m[k], str) and re.fullmatch('[0-9a-f]{32}', m[k]), 'operation_identity')
    for k in ('installer_sha256', 'transaction_sha256', 'result_sha256', 'software_sha256', 'preinstallation_sha256'):
        require(isinstance(m[k], str) and re.fullmatch('[0-9a-f]{64}', m[k]), 'input_hash')
    require(type(m['installer_size']) is int and 0 < m['installer_size'] <= 2**31, 'installer_size')
    require(set(m['installed_artifacts']) == {'installer_launch', 'preparation_kit', 'operator_frontend', 'manager', 'supervisor', 'ownership', 'host', 'source_manifest', 'native_catalogue'}, 'artifact_set')
    require(all(re.fullmatch('[0-9a-f]{64}', v) for v in m['installed_artifacts'].values()), 'artifact_hash')


def idle(s):
    a = s['system']
    require(s['schema'] == 5 and a['service'] == 'active' and a['keepers'] == 2
            and a['cleanup_unconfirmed'] is False and all(a[k] == 0 for k in ('dsp', 'maintenance', 'pending_transactions', 'stale_transports'))
            and s['capture']['armed'] is False and s['capture']['active_retention'] == 0, 'not_idle')


def snapshot(manager):
    # Exact verified installed manager, only its readback command. No arbitrary CLI.
    r = subprocess.run([manager, 'operator', 'snapshot'], capture_output=True, timeout=150)
    require(r.returncode == 0 and len(r.stdout) <= 16*1024*1024, 'readback_refused')
    s = decode(r.stdout); idle(s)
    return s


def custody(m, home):
    root = home/m['managed_relation']; env = root/'environments'/m['environment']; d = root/'onboarding'/m['environment']
    software_raw = read(root/'software.json', expected=m['software_sha256']); sw = decode(software_raw)
    for k, expected in m['installed_artifacts'].items():
        a = sw[k]
        require(a['sha256'] == expected and file_identity(a['path'])['sha256'] == expected, 'installed_artifact_changed')
    before = snapshot(sw['manager']['path'])
    require(not (root/'operator/resume.json').exists(), 'pending_resume')
    eraw = read(env/'environment.json'); oraw = read(d/'record.json')
    e = decode(eraw); o = decode(oraw)
    require(e['id'] == m['environment'] and e['root'] == str(env) and o['environment'] == e
            and o['id'] == m['environment'] and o['installation_operation'] == m['operation']
            and o['installer'] == m['installer_sha256'] and o['published'] is False, 'environment_binding')
    unit = 'linux-vst-bridge-installer-'+m['operation']+'.service'
    status = subprocess.run(['systemctl', '--user', 'show', unit, '--property=ActiveState', '--value'], capture_output=True, timeout=15)
    require(status.stdout.strip() in (b'inactive', b'failed', b'') and status.returncode in (0, 1), 'installer_unit_not_retired')
    rr = read(d/(m['operation']+'-result.json'), expected=m['result_sha256']); result = decode(rr)
    tr = read(d/(m['operation']+'-transaction-private.json'), expected=m['transaction_sha256']); tx = decode(tr)
    require(result['schema'] == 2 and result['operation'] == tx['operation'] == m['operation']
            and result['state'] == 'completed' and result['owned_live'] == 0 and result['cleanup_confirmed'] is True, 'operation_not_retired')
    b = result['transaction']['launch_binding']
    require(b['operation'] == m['operation'] and b['status'] == 'bound' and b['artifact_sha256'] == m['installer_sha256']
            and b['size'] == m['installer_size'], 'imported_root_binding')
    return root, env, d, sw, before, result, tx, {'environment_record': digest(eraw), 'onboarding_record': digest(oraw), 'result': digest(rr), 'transaction': digest(tr)}


def census(root):
    """Metadata-only before/after witness. Does not follow Wine's home aliases."""
    pending = [root]; out = {}; deadline = time.monotonic()+45
    while pending:
        d = pending.pop()
        require(d.resolve() == d, 'census_alias')
        fd = open_directory(d)
        try:
            with os.scandir(fd) as entries:
                for e in entries:
                    require(len(out) < 100000 and time.monotonic() < deadline, 'preservation_census_bound')
                    s = e.stat(follow_symlinks=False)
                    out[str((d/e.name).relative_to(root))] = [s.st_mode, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_dev, s.st_ino]
                    if stat.S_ISDIR(s.st_mode):
                        pending.append((d/e.name))
        finally:
            os.close(fd)
    return out


def inspect(drive, tx, result, logs):
    try:
        ident, private = discover(drive)
    except (Refusal, OSError, UnicodeError, KeyError, ValueError) as e:
        return {'application_identity': 'unresolved', 'renderer_family': 'unresolved',
                'reason': str(e) if isinstance(e, Refusal) else 'application_input_unavailable'}, None, {}, 'NAUI1_APPLICATION_IDENTITY_UNRESOLVED'
    try:
        evidence = parse(tx, result, logs, ident, private['windows_image'])
    except (Refusal, KeyError, TypeError, ValueError) as e:
        evidence = {'refusal': str(e) if isinstance(e, Refusal) else 'retained_record_unavailable'}
        disposition = ('NAUI1_ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED' if ident['renderer_family'] == 'electron_chromium'
                       else 'NAUI1_OTHER_RENDERER_CAUSE_UNRESOLVED')
    else:
        disposition = classify(ident, evidence)
    return ident, evidence, private, disposition


def main():
    require(len(sys.argv) == 1, 'no_runtime_arguments')
    source = pathlib.Path(__file__).resolve().parent
    seal, seal_hash = verify(source)
    raw = read(source/'input.json'); m = decode(raw); validate_manifest(m)
    home = pathlib.Path.home(); out = home/'.cache/linux-vst-bridge'/('naui1-'+seal['source_head'][:12])
    out.mkdir(mode=0o700, exist_ok=True)
    # One-use marker prevents a second physical pass, including after refusal.
    publish(out/'started.json', {'seal_sha256': seal_hash, 'source_head': seal['source_head']})
    root, env, directory, software, before, result, tx, hashes = custody(m, home)
    installation = home/'.cache/linux-vst-bridge/is4-install-8fb2f20/before-install.json'
    prior = decode(read(installation, expected=m['preinstallation_sha256']))
    preserved = {group: {p: file_identity(p, 2*1024*1024*1024)['sha256'] for p in prior[group]} for group in ('retained', 'projects', 'predecessor_files')}
    require(all(preserved[k] == prior[k] for k in preserved), 'prior_preservation_mismatch')
    before_tree = census(env)
    # Only existing operation-owned bounded sinks. Never open fresh vendor logs.
    logpaths = [directory/(m['operation']+'-private.log')]
    delta = tx.get('durable_installation', {}).get('delta', {}).get('logs', {})
    for rel in (delta.get('added', [])+delta.get('changed', []))[:32]:
        logpaths.append(directory/(m['operation']+'-log-'+digest(rel.encode())))
    logs = {}; loghashes = {}; unavailable = 0
    for p in logpaths:
        if not p.exists():
            unavailable += 1; continue
        b = read(p, 131072); key = digest(p.name.encode()); logs[key] = b; loghashes[key] = digest(b)
    identity, evidence, private, disposition = inspect(env/'compatdata/pfx/drive_c', tx, result, logs)
    after_tree = census(env)
    require(before_tree == after_tree, 'environment_mutation')
    require(all(file_identity(p, 2*1024*1024*1024)['sha256'] == h for g in preserved.values() for p, h in g.items()), 'retained_mutation')
    require(read(root/'software.json', expected=m['software_sha256']), 'software_mutation')
    require(read(directory/(m['operation']+'-result.json'), expected=m['result_sha256'])
            and read(directory/(m['operation']+'-transaction-private.json'), expected=m['transaction_sha256']), 'historical_mutation')
    require(all(digest(read(p, 131072)) == loghashes[digest(p.name.encode())] for p in logpaths if p.exists()), 'log_mutation')
    for key, filename in (('environment_record', env/'environment.json'), ('onboarding_record', directory/'record.json')):
        require(digest(read(filename)) == hashes[key], 'record_mutation')
    require(all(file_identity(a['path'])['sha256'] == a['sha256'] for a in software.values() if isinstance(a, dict) and 'sha256' in a), 'installed_artifact_mutation')
    after = snapshot(software['manager']['path'])
    require(before['products'] == after['products'] and before['onboarding'] == after['onboarding']
            and before.get('operation') == after.get('operation'), 'canonical_state_changed')
    links = {str(p): os.readlink(p) for p in (home/'.vst3').glob('LVB_*.vst3') if p.is_symlink()}
    require(links == prior['links'], 'publication_changed')
    require(not (root/'operator/resume.json').exists(), 'pending_resume')
    require(verify(source)[1] == seal_hash, 'diagnostic_source_changed')
    public = {'schema': 1, 'source_head': seal['source_head'], 'source_tree': seal['source_tree'],
              'source_seal_sha256': seal_hash, 'input_manifest_sha256': digest(raw), 'input': m,
              'disposition': disposition, 'identity': identity, 'evidence': evidence,
              'private_inputs': hashes, 'log_sha256': loghashes, 'unavailable_logs': unavailable,
              'retained_diagnostics': result['transaction']['diagnostics'],
              'presentation': 'operator_observed_blank_white_client_area',
              'preservation': {'system': after['system'], 'capture': after['capture'],
                  'operator_schema': after['schema'], 'software_unchanged': True, 'products_unchanged': True,
                  'publications_unchanged': True, 'onboarding_unchanged': True, 'operation_unchanged': True,
                  'environment_metadata_entries': len(before_tree), 'environment_metadata_unchanged': True,
                  'protected_counts': {k: len(v) for k, v in preserved.items()},
                  'prior_result_transaction_unchanged': True, 'commercial_launches': 0,
                  'registry_mutations': 0, 'new_environments': 0, 'new_installer_operations': 0,
                  'source_owned_process': 'this_read_only_python_process; direct_child_wait_by_remote_custodian'}}
    publish(out/'private.json', {'application': private, 'environment_metadata': before_tree,
                                'before': before, 'after': after, 'log_locations': [str(p) for p in logpaths]})
    publish(out/'result.json', public)
    print(canonical(public).decode())


if __name__ == '__main__':
    try:
        main()
    except (Refusal, OSError, ValueError, KeyError, TypeError) as e:
        print(canonical({'refused': str(e) if isinstance(e, Refusal) else 'unavailable_input', 'exception_type': type(e).__name__}).decode())
        sys.exit(1)
