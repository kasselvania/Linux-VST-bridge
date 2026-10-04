#!/usr/bin/env python3
"""Build first-party installers with the selected exact MSVC fixture payloads."""
import argparse, hashlib, json, os, pathlib, shutil, subprocess, time, uuid
parser = argparse.ArgumentParser()
parser.add_argument('--windows-artifacts', type=pathlib.Path, required=True)
parser.add_argument('--fixture-family', choices=('reference', 'completion'), default='reference')
args = parser.parse_args()
directory = args.windows_artifacts.resolve(strict=True)
source = pathlib.Path(__file__).with_name('reference_installer.rs')
family = args.fixture_family
env = dict(os.environ, LVB_BETA_INSTRUMENT=str(directory/f'lvb-{family}-instrument.vst3'),
           LVB_BETA_EFFECT=str(directory/f'lvb-{family}-effect.vst3'))
rows = []
installers = [('LVB_Completion_Plugins_1_0_0.exe', 'beta_completion')] if family == 'completion' else [
                           ('LVB_Reference_Plugins_1_0_0.exe', None),
                            ('LVB_Reference_Recovery_1_0_0.exe', 'beta_hold'),
                            ('LVB_Reference_Partial_1_0_0.exe', 'beta_partial_hold')]
for name, configuration in installers:
    command = ['rustc', str(source), '--edition=2024', '-D', 'warnings',
               '-C', 'opt-level=2', '-C', 'target-feature=+crt-static',
               '--check-cfg', 'cfg(beta_hold)', '--check-cfg', 'cfg(beta_partial_hold)',
               '--check-cfg', 'cfg(beta_completion)',
               '-o', str(directory/name)]
    if configuration: command += ['--cfg', configuration]
    subprocess.run(command, env=env, check=True)
    subprocess.run([str(directory/name), '--self-test'], check=True)
    rows.append({'file': name, 'sha256': hashlib.sha256((directory/name).read_bytes()).hexdigest(),
                 'configuration': configuration or 'ordinary'})
for role in ('instrument', 'effect'):
    name = f'lvb-{family}-{role}.vst3'
    rows.append({'file': name, 'sha256': hashlib.sha256((directory/name).read_bytes()).hexdigest(), 'role': role})

# Census the actual modules through the production Windows inspection host.
# These records supply the builder's exact prebuilt descriptors; names, class
# IDs and buses are never guessed from the reference module's source.
host = directory/'wf0-factory-probe.exe'
source_manifest = directory/'host-source-manifest.json'
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
for role in ('instrument', 'effect'):
    sid = uuid.uuid4().hex
    session = pathlib.Path('C:/bridge/sessions')/sid
    session.mkdir(parents=True)
    module = session/'fixture.vst3'
    shutil.copyfile(directory/f'lvb-{family}-{role}.vst3', module)
    ready, gate = session/(sid+'.ready'), session/(sid+'.gate')
    fields = [('schema', 'linux-vst-bridge-wf0-handshake/v1'), ('session', sid),
        ('scanner_sha256', digest(host)), ('module_sha256', digest(module)),
        ('bundle_manifest_sha256', '02'*32),
        ('implementation_source_manifest_sha256', digest(source_manifest)),
        ('mode', 'ap8-module-inspection'), ('component_case', 'first-audio'), ('run_ordinal', '1')]
    binding = ''.join(f'{key}={value}\n' for key, value in fields).encode()
    command = [str(host), '--session', sid, '--scanner-sha256', digest(host),
        '--implementation-source-manifest-sha256', digest(source_manifest),
        '--module', str(module), '--module-sha256', digest(module),
        '--bundle-manifest-sha256', '02'*32, '--ready', str(ready), '--gate', str(gate),
        '--max-classes', '256', '--stdout-cap', '1048576',
        '--mode', 'ap8-module-inspection', '--component-case', 'first-audio']
    environment = dict(os.environ)
    for name in ('LVB_AP8_CONTROLLER_QUERY_CASE', 'LVB_AUDIO_LAYOUT_POLICY'):
        environment.pop(name, None)
    child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=environment)
    try:
        deadline = time.monotonic()+20
        while True:
            if child.poll() is not None: raise RuntimeError(f'{family} host exited before readiness')
            if time.monotonic() >= deadline: raise RuntimeError(f'{family} host readiness timeout')
            try: actual = ready.read_bytes()
            except (FileNotFoundError, PermissionError):
                time.sleep(.01); continue
            if actual != binding: raise RuntimeError(f'{family} host binding mismatch')
            break
        temporary = gate.with_suffix('.tmp')
        with temporary.open('xb') as stream:
            stream.write(binding); stream.flush(); os.fsync(stream.fileno())
        os.replace(temporary, gate)
        output, error = child.communicate(timeout=30)
        if child.returncode != 0: raise RuntimeError(f'{family} inspection failed: {child.returncode} {error[-1024:]!r} {output[-2048:]!r}')
        if len(output) > 1048576: raise RuntimeError(f'{family} inspection exceeded bound')
        records = [json.loads(line) for line in output.splitlines()]
        classes = [r for r in records if r.get('state') == 'ap12_class' and
                   ('Instrument' if role == 'instrument' else 'Fx') in r.get('subcategories', '').split('|')]
        if len(classes) != 1: raise RuntimeError(f'{family} processor class census differs')
        import sys
        sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
        from ap8_descriptor import prebuilt_descriptor
        prebuilt_descriptor(records, classes[0]['class_id'], digest(module))
        report = {'schema': 1, 'gated': True, 'cleanup_confirmed': True, 'transport_retired': True,
            'error': None, 'class_id': classes[0]['class_id'], 'module_sha256': digest(module),
            'records': records, 'provenance': 'first-party Windows CI production-host census; no installed DAW claim'}
        (directory/f'lvb-{family}-{role}-inspection.json').write_text(json.dumps(report, sort_keys=True)+'\n')
    finally:
        if child.poll() is None: child.kill(); child.wait(10)
        child.stdout.close(); child.stderr.close(); shutil.rmtree(session)
(directory/f'LVB_{family.upper()}_MANIFEST.json').write_text(json.dumps({'schema': 1,
    'version': '1.0.0', 'classification': 'first_party_test_instrumentation', 'files': rows}, indent=2)+'\n')
