#!/usr/bin/env python3
"""Assemble the one sealed UIR1 host package from its exact hosted archive.
Locations supply bytes, never profile policy. The native is reused unchanged.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[2]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def material(windows_zip):
    receipt = json.loads(Path(__file__).with_name('package.json').read_text())
    assert digest(windows_zip.read_bytes()) == receipt['zip_sha256'], 'archive mismatch'
    with zipfile.ZipFile(windows_zip) as z:
        build = z.read('SOURCE_COMMIT.txt').decode('utf-8-sig').strip()
        assert build == receipt['github_build_commit'], 'workflow identity mismatch'
        for name, sha in receipt['files'].items():
            assert digest(z.read(name)) == sha, 'helper mismatch'
        host = z.read('wf0-factory-probe.exe')
    assert git('rev-parse', build+'^{tree}') == receipt['common_tree']
    assert git('rev-parse', receipt['source_head']+'^{tree}') == receipt['common_tree']
    paths = ['CMakeLists.txt', 'cmake', 'windows-factory-probe', 'vst-state',
             'windows-fixtures/ap10', 'native-vst3-proxy/include',
             'native-vst3-proxy/source/output_results.h', '.github/workflows/ap8-windows.yml',
             'tools/wf0-factory-census', 'tools/ap10-tests', 'tools/ap11-tests',
             'tools/ap12-tests', 'tools/ap13-tests', 'tools/ap14-tests', 'tools/ap18-tests',
             'tools/uio1/windows.cpp', 'tools/uir1/windows.cpp',
             'tools/test_delivery_trace.py', 'tools/test_delivery_mailbox.py', 'tools/test_ap4_socket.py']
    records = []
    for line in git('ls-tree', '-r', build, '--', *paths).splitlines():
        mode, kind, blob, path = line.split(None, 3)
        assert kind == 'blob'
        records.append(dict(path=path, git_mode=mode, git_blob=blob))
    manifest = dict(schema='linux-vst-bridge-uir1-host-source/v1',
                    source_head=receipt['source_head'], commit=build, tree=receipt['common_tree'],
                    ci_run=receipt['workflow_run'], artifact_id=receipt['artifact_id'],
                    artifact_zip_sha256=receipt['zip_sha256'],
                    binaries={'wf0-factory-probe.exe': digest(host)},
                    sdk_commit='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96',
                    toolchain=dict(generator='Visual Studio 17 2022', architecture='x64',
                                   toolset='v143', windows_sdk='10.0.19041.0'), inputs=records)
    source = (json.dumps(manifest, sort_keys=True, separators=(',', ':'))+'\n').encode()
    return host, source


def assemble(windows_zip, native, output):
    p = json.loads((ROOT/'compatibility/uir1/arturia-pigments.json').read_text())
    assert p['revision'] == 12 and p['claim'] == 'review_candidate'
    host, source = material(windows_zip)
    files = {'host.exe': host, 'host-source-manifest.json': source,
             p['class']['class_id']+'.so': native.read_bytes()}
    req = p['requirements']
    assert [digest(x) for x in files.values()] == [req['host_sha256'], req['host_source_sha256'], req['native_sha256']]
    output.mkdir(mode=0o700)
    for name, data in files.items():
        (output/name).write_bytes(data); (output/name).chmod(0o400)
    return {name:digest(data) for name,data in files.items()}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--windows-zip', type=Path, required=True)
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    a = parser.parse_args()
    print(json.dumps(assemble(a.windows_zip, a.native, a.output)))
