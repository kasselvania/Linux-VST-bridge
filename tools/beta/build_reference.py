#!/usr/bin/env python3
"""Build the first-party installer with exact MSVC reference module payloads."""
import argparse, hashlib, json, os, pathlib, subprocess
parser = argparse.ArgumentParser()
parser.add_argument('--windows-artifacts', type=pathlib.Path, required=True)
args = parser.parse_args()
directory = args.windows_artifacts.resolve(strict=True)
source = pathlib.Path(__file__).with_name('reference_installer.rs')
env = dict(os.environ, LVB_BETA_INSTRUMENT=str(directory/'lvb-reference-instrument.vst3'),
           LVB_BETA_EFFECT=str(directory/'lvb-reference-effect.vst3'))
rows = []
for name, configuration in [('LVB_Reference_Plugins_1_0_0.exe', None),
                            ('LVB_Reference_Recovery_1_0_0.exe', 'beta_hold'),
                            ('LVB_Reference_Partial_1_0_0.exe', 'beta_partial_hold')]:
    command = ['rustc', str(source), '--edition=2024', '-D', 'warnings',
               '-C', 'opt-level=2', '-C', 'target-feature=+crt-static',
               '--check-cfg', 'cfg(beta_hold)', '--check-cfg', 'cfg(beta_partial_hold)',
               '-o', str(directory/name)]
    if configuration: command += ['--cfg', configuration]
    subprocess.run(command, env=env, check=True)
    subprocess.run([str(directory/name), '--self-test'], check=True)
    rows.append({'file': name, 'sha256': hashlib.sha256((directory/name).read_bytes()).hexdigest(),
                 'configuration': configuration or 'ordinary'})
for role in ('instrument', 'effect'):
    name = f'lvb-reference-{role}.vst3'
    rows.append({'file': name, 'sha256': hashlib.sha256((directory/name).read_bytes()).hexdigest(), 'role': role})
(directory/'LVB_REFERENCE_MANIFEST.json').write_text(json.dumps({'schema': 1,
    'version': '1.0.0', 'classification': 'first_party_test_instrumentation', 'files': rows}, indent=2)+'\n')
