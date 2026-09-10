#!/usr/bin/env python3
"""Assemble the finite AP15 qualification package; paths supply bytes, not policy."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
REVIEWED = '0d82c75250f62b585a21081d3fc052d04c1f514e'
BUILD = '46c7b85fce2370644cccec6197a6f1a06c188be1'
TREE = '9d9ea7de6111a32e8359187c88802deff8f005fb'
ZIP = '66724d36be9dd19bdb15210409c6e9deb5edd63485a0755e9fe0f1caecc5c07d'
BINARIES = {
    'wf0-factory-probe.exe': '3ee36fd34384b2c282ec9cb6ae6c48d17f83940eaaf6d8f9a1fa130b65274939',
    'ap11-editor-tests.exe': '0fc35cc60af93080bca0b7aecc6a4e25c4072acef487d4031a1fd8dc8521e11d',
    'ap13-state-delivery-tests.exe': 'f566c916bd24fe140f0684aa2e8b63cfcac2e39723d310a9c7656dddb2f3e59e',
}
PROFILE_FILES = ['arturia-pure-lofi.json', 'arturia-efx-fragments.json']

def sha(data):
    return hashlib.sha256(data).hexdigest()

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()

def manifest():
    assert git('rev-parse', REVIEWED+'^{tree}') == git('rev-parse', BUILD+'^{tree}') == TREE
    paths = ['CMakeLists.txt', 'cmake', 'windows-factory-probe', 'vst-state',
             'windows-fixtures/ap10', 'native-vst3-proxy/include',
             'native-vst3-proxy/source/output_results.h', '.github/workflows/ap8-windows.yml',
             'tools/wf0-factory-census', 'tools/ap10-tests', 'tools/ap11-tests',
             'tools/ap12-tests', 'tools/ap13-tests', 'tools/ap14-tests',
             'tools/test_delivery_trace.py', 'tools/test_delivery_mailbox.py', 'tools/test_ap4_socket.py']
    records = []
    for line in git('ls-tree', '-r', BUILD, '--', *paths).splitlines():
        mode, kind, blob, path = line.split(None, 3)
        assert kind == 'blob'
        records.append(dict(path=path, git_mode=mode, git_blob=blob))
    value = dict(schema='linux-vst-bridge-ap15-host-source/v1', reviewed_head=REVIEWED,
                 commit=BUILD, tree=TREE, ci_run=34418620336, artifact_id=10130485567,
                 artifact_zip_sha256=ZIP, binaries=BINARIES,
                 sdk_commit='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96',
                 sdk_lock='cmake/HP0Vst3SdkLock.cmake',
                 toolchain=dict(generator='Visual Studio 17 2022', architecture='x64',
                                toolset='v143', windows_sdk='10.0.19041.0',
                                observed_msvc='19.44.35228.0'), inputs=records)
    return (json.dumps(value, sort_keys=True, separators=(',', ':'))+'\n').encode()

def assemble(windows_zip, natives, output):
    assert sha(windows_zip.read_bytes()) == ZIP, 'Windows archive differs'
    with zipfile.ZipFile(windows_zip) as z:
        assert z.read('SOURCE_COMMIT.txt').decode('utf-8-sig').strip() == BUILD
        for name, digest in BINARIES.items():
            assert sha(z.read(name)) == digest, name
        host = z.read('wf0-factory-probe.exe')
    source = manifest()
    files = {'host.exe': host, 'host-source-manifest.json': source}
    for name in PROFILE_FILES:
        p = json.loads((ROOT/'compatibility/ap15'/name).read_text())
        r = p['requirements']
        assert p['revision'] == 4 and p['claim'] == 'review_candidate'
        assert r['host_sha256'] == sha(host) and r['host_source_sha256'] == sha(source)
        filename = p['class']['class_id']+'.so'
        files[filename] = (natives/filename).read_bytes()
        assert sha(files[filename]) == r['native_sha256'], filename
    # Validate all bytes before creating a package; never overwrite an old one.
    output.mkdir(mode=0o700)
    for name, data in files.items():
        (output/name).write_bytes(data)
        (output/name).chmod(0o400)
    receipt = dict(schema=1, reviewed_head=REVIEWED, build_commit=BUILD, tree=TREE,
                   files={name: sha(data) for name, data in files.items()})
    (output/'package-receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    return receipt

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--windows-zip', type=pathlib.Path, required=True)
    parser.add_argument('--natives', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    print(json.dumps(assemble(args.windows_zip, args.natives, args.output)))
