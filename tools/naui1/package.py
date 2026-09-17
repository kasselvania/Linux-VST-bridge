"""Seal committed diagnostic bytes; package is outside all managed environments."""
import os
import pathlib
import re
import subprocess
import sys
from common import canonical, decode, digest, read, require, publish

FILES = {'common.py', 'identity.py', 'records.py', 'classify.py', 'run.py', 'package.py', 'input.json'}


def verify(root):
    root = pathlib.Path(root)
    require({p.name for p in root.iterdir()} == FILES | {'seal.json'}, 'package_file_set')
    raw = read(root/'seal.json', 32768)
    seal = decode(raw)
    require(raw == canonical(seal)+b'\n' and set(seal) == {'schema', 'source_head', 'source_tree', 'files'}, 'package_schema')
    require(seal['schema'] == 1 and all(re.fullmatch('[0-9a-f]{40}', seal[k]) for k in ('source_head', 'source_tree'))
            and set(seal['files']) == FILES, 'package_identity')
    for name, expected in seal['files'].items():
        require(not (root/name).stat().st_mode & 0o222, 'package_writable')
        require(digest(read(root/name, 256*1024)) == expected, 'package_file_digest')
    return seal, digest(raw)


def build(repo, output):
    repo = pathlib.Path(repo); output = pathlib.Path(output)
    def git(*args):
        return subprocess.check_output(['git', '-C', str(repo), *args])
    head = git('rev-parse', 'HEAD').decode().strip(); tree = git('rev-parse', 'HEAD^{tree}').decode().strip()
    require(not git('status', '--porcelain', '--', 'tools/naui1'), 'uncommitted_diagnostic_source')
    output.mkdir(mode=0o700)
    hashes = {}
    for name in sorted(FILES):
        raw = git('show', head+':tools/naui1/'+name)
        hashes[name] = digest(raw)
        p = output/name
        with p.open('xb') as f:
            f.write(raw); f.flush(); os.fsync(f.fileno()); os.fchmod(f.fileno(), 0o400)
    publish(output/'seal.json', {'schema': 1, 'source_head': head, 'source_tree': tree, 'files': hashes})
    return verify(output)


if __name__ == '__main__':
    require(len(sys.argv) == 3, 'package_usage_repo_and_new_output')
    print(canonical(build(sys.argv[1], sys.argv[2])[0]).decode())
