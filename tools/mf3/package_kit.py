#!/usr/bin/env python3
"""Maintainer packaging: fixed native recipe, owned sources and registered archive.
No SDK, vendor module, installer, account or preset is included.
"""
import argparse, hashlib, json, os, pathlib, subprocess, zipfile
ROOT=pathlib.Path(__file__).resolve().parents[2]
from native_builder import SDK, SDK_RUNTIME

def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--windows-package',type=pathlib.Path,required=True);a=p.parse_args()
    def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
    if git('status','--porcelain'):raise SystemExit('Commit exact build inputs first')
    compiler=subprocess.check_output(['rustup','which','--toolchain','stable','rustc'],text=True).strip()
    subprocess.run(['rustup','run','stable','cargo','build','--manifest-path','native-vst3-proxy/backend/Cargo.toml','--release','--locked','--offline','--target','x86_64-unknown-linux-gnu','--features','registered'],cwd=ROOT,env={**os.environ,'RUSTC':compiler,'RUSTFLAGS':'-C relocation-model=pic'},check=True)
    names=git('ls-files','CMakeLists.txt','cmake/HP0Vst3SdkLock.cmake','cmake/HP0ModernGcc.cmake','native-vst3-proxy','vst-state').splitlines()
    files={name:(ROOT/name).read_bytes() for name in names}
    files['libap2_backend.a']=(ROOT/'native-vst3-proxy/backend/target/x86_64-unknown-linux-gnu/release/libap2_backend.a').read_bytes()
    for name in ['host.exe','host-source-manifest.json']:
        path=a.windows_package/('wf0-factory-probe.exe' if name=='host.exe' else name)
        files['runtime/'+name]=path.read_bytes()
    manifest=json.loads(files['runtime/host-source-manifest.json'])
    if manifest['host_sha256']!=hashlib.sha256(files['runtime/host.exe']).hexdigest():raise SystemExit('Windows package identity differs')
    for name,sha in manifest['files'].items():
        data=(ROOT/name).read_bytes()
        # actions/checkout uses CRLF on Windows. Match the exact retained checkout
        # digest; permit only Git's declared text line-ending conversion.
        if sha not in {hashlib.sha256(data).hexdigest(),hashlib.sha256(data.replace(b'\n',b'\r\n')).hexdigest()}:raise SystemExit('Windows package source differs: '+name)
    recipe=dict(schema=1,source_commit=git('rev-parse','HEAD'),sdk=SDK,sdk_runtime=SDK_RUNTIME,files={n:hashlib.sha256(v).hexdigest() for n,v in files.items()})
    a.output.parent.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(a.output,'x',compression=zipfile.ZIP_DEFLATED) as z:
        for name,data in files.items():z.writestr(name,data)
        z.writestr('recipe.json',json.dumps(recipe,sort_keys=True))
    print(json.dumps(dict(sha256=hashlib.sha256(a.output.read_bytes()).hexdigest(),size=a.output.stat().st_size,source=recipe['source_commit'])))
if __name__=='__main__':main()
