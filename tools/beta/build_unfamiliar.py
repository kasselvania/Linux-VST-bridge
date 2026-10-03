#!/usr/bin/env python3
"""Build independent Windows fixtures after a candidate package is frozen.

Only a disposable copy of our first-party fixture source is changed. The
existing MSVC/SDK build and state/DSP contract tests remain the build path.
No generated identity or module is added to the frozen package or its kit.
"""
import argparse
import hashlib
import io
import json
import os
import pathlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import uuid

ROOT = pathlib.Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sdk', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--evolve-generation', help='Existing first-party identity; build schema/parameter migration and refusal successors')
    parser.add_argument('--frozen-package-sha256', required=True)
    parser.add_argument('--frozen-engine-sha256', required=True)
    args = parser.parse_args()
    if os.name != 'nt':
        raise ValueError('Native Windows MSVC fixture build required')
    for value in (args.frozen_package_sha256, args.frozen_engine_sha256):
        if not re.fullmatch('[0-9a-f]{64}', value):
            raise ValueError('Exact frozen artifact digest required')
    if args.output.exists():
        raise ValueError('Retain earlier fixture artifacts; choose a new output')
    args.output.mkdir(parents=True)
    sdk = args.sdk.resolve(strict=True)
    git = lambda *values: subprocess.check_output(['git', '-C', str(ROOT), *values]).strip()
    if git('status', '--porcelain'):
        raise ValueError('Commit the fixture generator before running it')
    generation = args.evolve_generation or uuid.uuid4().hex[:8].upper()
    if not re.fullmatch('[0-9A-F]{8}',generation):
        raise ValueError('Exact first-party generation required')
    if generation == '45544131':
        raise ValueError('Generated identity collides with the original fixture')
    original = (ROOT / 'windows-fixtures/beta/stateful.cpp').read_text()
    if original.count('0x45544131') != 2 or original.count('"1.0.0"') != 2:
        raise ValueError('Reference fixture identity/version seam changed')
    manifest = dict(schema=1, classification='first_party_test_instrumentation',
        generator_source_head=git('rev-parse', 'HEAD').decode(),
        generator_source_sha256=sha(pathlib.Path(__file__)),
        reference_source_sha256=sha(ROOT / 'windows-fixtures/beta/stateful.cpp'),
        frozen_package_sha256=args.frozen_package_sha256,
        frozen_engine_sha256=args.frozen_engine_sha256,
        generation=generation, evolving_contract=bool(args.evolve_generation), revisions=[])
    with tempfile.TemporaryDirectory(prefix='lvb-unfamiliar-') as tmp:
        work = pathlib.Path(tmp)
        source, build = work / 'source', work / 'build'
        source.mkdir()
        source_archive = subprocess.check_output(['git', '-C', str(ROOT), 'archive', '--format=tar', 'HEAD'])
        with tarfile.open(fileobj=io.BytesIO(source_archive)) as archive:
            archive.extractall(source, filter='data')
        subprocess.run(['cmake', '-S', str(source), '-B', str(build),
            '-G', 'Visual Studio 17 2022', '-A', 'x64', '-T', 'v143',
            '-DCMAKE_SYSTEM_VERSION=10.0.19041.0', '-DWF0_BUILD_ONLY=ON',
            '-DWF0_VST3_SDK_ROOT:PATH=' + str(sdk)], check=True, timeout=120)
        for revision in ((3,4,5) if args.evolve_generation else (1,2)):
            version = f'1.0.{revision}'
            generated = original.replace('0x45544131', '0x' + generation)
            if args.evolve_generation:
                generated = f'#define LVB_BETA_STATE_VERSION 2\n#define LVB_BETA_RESTORE_POLICY {revision-3}\n' + generated
            generated = generated.replace('"1.0.0"', '"' + version + '"')
            generated = generated.replace('LVB Reference ', 'LVB Unfamiliar ')
            generated_path = source / 'windows-fixtures/beta/stateful.cpp'
            generated_path.write_text(generated, newline='\n')
            targets = [f'lvb-reference-{role}{suffix}'
                       for role in ('instrument', 'effect') for suffix in ('', '-tests')]
            subprocess.run(['cmake', '--build', str(build), '--config', 'Release',
                '--target', *targets, '--parallel', '2'], check=True, timeout=600)
            binaries = build / 'wf0/bin/Release'
            destination = args.output / version
            destination.mkdir()
            row = dict(version=version, state_schema=2 if args.evolve_generation else 1,
                       parameter_ids=[0,1,17] if args.evolve_generation else [0,1],
                       restore_policy=(revision-3) if args.evolve_generation else 0,
                       generated_source_sha256=sha(generated_path),
                       modules=[], contract_tests=[])
            for role in ('instrument', 'effect'):
                test = subprocess.check_output([str(binaries / f'lvb-reference-{role}-tests.exe')], timeout=30)
                row['contract_tests'].append(dict(role=role, result=test.decode().strip()))
                name = f'lvb-reference-{role}.vst3'
                shutil.copyfile(binaries / name, destination / name)
                role_id = '494E5354' if role == 'instrument' else '45464658'
                row['modules'].append(dict(role=role, file=version + '/' + name,
                    sha256=sha(destination / name),
                    processor_class='4C564242' + generation + role_id + '00000001',
                    controller_class='4C564242' + generation + role_id + '00000002'))
            env = dict(os.environ,
                LVB_BETA_INSTRUMENT=str((destination / 'lvb-reference-instrument.vst3').resolve()),
                LVB_BETA_EFFECT=str((destination / 'lvb-reference-effect.vst3').resolve()))
            installer = destination / ('LVB_Unfamiliar_' + version.replace('.', '_') + '.exe')
            subprocess.run(['rustc', str(ROOT / 'tools/beta/reference_installer.rs'),
                '--edition=2024', '-D', 'warnings', '-C', 'opt-level=2',
                '-C', 'target-feature=+crt-static', '--check-cfg', 'cfg(beta_hold)',
                '--check-cfg', 'cfg(beta_partial_hold)', '-o', str(installer)],
                env=env, check=True, timeout=120)
            subprocess.run([str(installer), '--self-test'], check=True, timeout=30)
            row['installer'] = dict(file=version + '/' + installer.name, sha256=sha(installer))
            manifest['revisions'].append(row)
    (args.output / 'UNFAMILIAR_FIXTURES.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest, sort_keys=True))


if __name__ == '__main__':
    main()
