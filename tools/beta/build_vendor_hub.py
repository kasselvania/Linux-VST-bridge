#!/usr/bin/env python3
"""Build the small first-party hub around two exact retained Windows installers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plugins', type=Path, required=True)
    parser.add_argument('--plugins-sha256', required=True)
    parser.add_argument('--companion', type=Path, required=True)
    parser.add_argument('--companion-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--target', default='x86_64-pc-windows-msvc')
    parser.add_argument('--sysroot', type=Path)
    parser.add_argument('--linker')
    args = parser.parse_args()
    for path, expected in [(args.plugins, args.plugins_sha256),
                           (args.companion, args.companion_sha256)]:
        if len(expected) != 64 or digest(path) != expected or path.read_bytes()[:2] != b'MZ':
            raise ValueError('Exact first-party installer input required')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).with_name('vendor_hub.rs')
    environment = dict(os.environ, LVB_HUB_PLUGINS=str(args.plugins.resolve()),
                       LVB_HUB_COMPANION=str(args.companion.resolve()))
    files = []
    for role, name in [('helper', 'helper.exe'), ('application', 'hub.exe'),
                       ('bootstrap', 'LVB_Source_Vendor_Hub_1_0_0.exe')]:
        path = output / name
        command = ['rustc', str(source), '--edition=2024', '--target', args.target,
                   '-D', 'warnings', '-C', 'opt-level=2', '-C', 'target-feature=+crt-static',
                   '--cfg', 'hub_' + role, '--check-cfg', 'cfg(hub_helper)',
                   '--check-cfg', 'cfg(hub_application)', '--check-cfg', 'cfg(hub_bootstrap)',
                   '-o', str(path)]
        if args.sysroot:
            command += ['--sysroot', str(args.sysroot)]
        if args.linker:
            command += ['-C', 'linker=' + args.linker]
        subprocess.run(command, env=environment, check=True, timeout=120)
        environment['LVB_HUB_' + role.upper()] = str(path)
        files.append(dict(file=name, sha256=digest(path), bytes=path.stat().st_size))
    (output / 'manifest.json').write_text(json.dumps(dict(schema=1,
        classification='first_party_test_instrumentation', source_sha256=digest(source),
        builder_sha256=digest(Path(__file__)), plugins_sha256=args.plugins_sha256,
        companion_sha256=args.companion_sha256, target=args.target, files=files), indent=2) + '\n')


if __name__ == '__main__':
    main()
