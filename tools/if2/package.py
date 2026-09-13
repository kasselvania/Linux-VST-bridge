#!/usr/bin/env python3
"""Assemble exact IF2 candidate 16, reusing the unchanged IF1 Windows artifact."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('if1_package', ROOT/'tools/if1/package.py')
prior = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prior)

def assemble(windows_zip, native, output):
    profile = json.loads((ROOT/'compatibility/if2/arturia-pigments.json').read_text())
    assert profile['revision'] == 16 and profile['claim'] == 'review_candidate'
    host, source = prior.material(windows_zip)
    files = {'host.exe': host, 'host-source-manifest.json': source,
             profile['class']['class_id']+'.so': native.read_bytes()}
    req = profile['requirements']
    assert [hashlib.sha256(b).hexdigest() for b in files.values()] == [
        req['host_sha256'], req['host_source_sha256'], req['native_sha256']]
    output.mkdir(mode=0o700)
    for name, data in files.items():
        (output/name).write_bytes(data)
        (output/name).chmod(0o400)
    return {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--windows-zip', type=Path, required=True)
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(assemble(args.windows_zip, args.native, args.output)))
