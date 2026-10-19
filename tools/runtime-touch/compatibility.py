#!/usr/bin/env python3
"""Compare the loader-visible contract of a replacement Wine Unix driver.

Run against the retained GE preimage and the exact built artifact. No execution
of either shared object is required; readelf/strings are the only ELF consumers.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def inspect(path):
    def read(*args):
        return subprocess.check_output(['readelf', *args, str(path)], text=True,
                                       env={**os.environ, 'LC_ALL': 'C'})
    header = dict(line.strip().split(':', 1) for line in read('-h').splitlines() if ':' in line)
    header = {key: header[key].strip() for key in
              ('Class', 'Data', 'OS/ABI', 'ABI Version', 'Type', 'Machine', 'Flags')}
    if header['Class'] != 'ELF64' or header['Machine'] != 'Advanced Micro Devices X86-64' or not header['Type'].startswith('DYN '):
        raise ValueError('Expected x86-64 ELF shared object')
    dynamic = {}
    for line in read('-d', '--wide').splitlines():
        match = re.search(r'\((NEEDED|SONAME|RPATH|RUNPATH|FLAGS|FLAGS_1)\)\s+(.*)', line)
        if match:
            dynamic.setdefault(match[1], []).append(match[2].strip())
    if len(dynamic.get('SONAME', [])) != 1:
        raise ValueError('Expected one driver SONAME')
    exports, imports = [], []
    for line in read('--dyn-syms', '--wide').splitlines():
        fields = line.split()
        if len(fields) < 8 or not fields[0].rstrip(':').isdigit():
            continue
        _, _, size, kind, binding, visibility, section, name, *_ = fields
        # Numeric version-table indexes and section indexes are file-local.
        # Version names remain part of the symbol name. Data size is ABI.
        symbol = [name, kind, binding, visibility, int(size) if kind in ('OBJECT', 'TLS') else None]
        (imports if section == 'UND' else exports).append(symbol)
    strings = subprocess.check_output(['strings', str(path)], text=True).splitlines()
    libraries = sorted({s for s in strings if re.fullmatch(r'lib[^ /]+\.so(?:\.[0-9]+)*', s)})
    compiler = [line.split(']', 1)[1].strip() for line in read('-p', '.comment').splitlines() if ']' in line]
    return dict(header=header, dynamic={k: sorted(v) for k, v in dynamic.items()},
                exports=sorted(exports), imports=sorted(imports),
                library_names=libraries, compiler=compiler)


def compare(original, replacement):
    before, after = inspect(original), inspect(replacement)
    for key in before:
        if before[key] != after[key]:
            raise ValueError('GE driver compatibility mismatch: ' + key)
    if not before['exports'] or not before['imports']:
        raise ValueError('Empty driver symbol surface')
    return before


def build_facts(built):
    config = (built/'obj64/include/config.h').read_text()
    makefile = (built/'obj64/Makefile').read_text()
    source = built/'wine/dlls/winex11.drv'
    inputs = '\n'.join(p.read_text() for pattern in ('*.c', '*.h') for p in source.glob(pattern))
    macros = sorted(set(re.findall(r'\b(?:HAVE|SONAME)_[A-Z0-9_]+', inputs)))
    facts = {}
    for macro in macros:
        match = re.search(r'^#define ' + macro + r' (.*)$', config, re.M)
        if not match or (macro.startswith("HAVE_") and match.group(1).strip() != "1"):
            raise ValueError('Missing X11 build feature: ' + macro)
        facts[macro] = match.group(1)
    if re.search(r'^\s*#.*(?:HAVE_\w*WAYLAND|HAVE_FFMPEG)', inputs, re.M):
        raise ValueError('X11 source now depends on Wayland/FFmpeg configuration')
    flags = {}
    for name in ('CC', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS', 'X_CFLAGS', 'X_LIBS'):
        match = re.search(r'^' + name + r' = (.*)$', makefile, re.M)
        if not match:
            raise ValueError('Missing build variable: ' + name)
        flags[name] = match.group(1)
    versions = {}
    for name in ('gdi', 'vulkan', 'opengl'):
        header = (built/'wine/include/wine'/f'{name}_driver.h').read_text()
        macro = 'WINE_' + name.upper() + '_DRIVER_VERSION'
        versions[macro] = int(re.search(r'^#define ' + macro + r' (\d+)$', header, re.M).group(1))
    if versions != {'WINE_GDI_DRIVER_VERSION': 111, 'WINE_VULKAN_DRIVER_VERSION': 49,
                    'WINE_OPENGL_DRIVER_VERSION': 38}:
        raise ValueError('GE driver interface header version changed')
    return dict(x11_features=facts, build_variables=flags, driver_versions=versions,
                config_sha256=digest(built/'obj64/include/config.h'),
                makefile_sha256=digest(built/'obj64/Makefile'))


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify(original, built):
    record = json.loads((built/'build-manifest.json').read_text())
    replacement = built/'obj64/dlls/winex11.drv/winex11.so'
    if digest(original) != '5acbf619a5d13b4d38cde3adba9724e5de5427a117caf0e2336864e20281f080':
        raise ValueError('GE driver preimage changed')
    if digest(replacement) != record['artifact']['sha256']:
        raise ValueError('Built driver changed')
    return dict(schema=1, original_sha256=digest(original), replacement_sha256=digest(replacement),
                interface=compare(original, replacement), configuration=build_facts(built))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--original', type=Path, required=True)
    parser.add_argument('--build', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(verify(args.original, args.build), indent=2, sort_keys=True))
