#!/usr/bin/env python3
"""Closed source-owned invocation of the production installer owner, never installed."""
import hashlib,json,pathlib,sys
import session

def main():
    spec_path=pathlib.Path(sys.argv[1]);mode=sys.argv[2]
    if mode not in ('baseline','unix_override','restored'):raise ValueError('fixture mode')
    spec=json.loads(spec_path.read_text())
    manifest=json.loads(pathlib.Path(__file__).with_name('fixture.json').read_text())
    for name,digest in manifest['files'].items():
        if pathlib.Path(name).name!=name:raise ValueError('fixture path')
        p=pathlib.Path(__file__).with_name(name)
        if hashlib.sha256(p.read_bytes()).hexdigest()!=digest:raise ValueError('fixture source drift')
    if spec['installer']['sha256']!=manifest['files']['payload.exe']:raise ValueError('not exact source-owned payload')
    session.managed_install(spec,source_owned_is3={'schema':1,'mode':mode,'operation':spec['operation'],'artifact':spec['installer']})
if __name__=='__main__':main()
