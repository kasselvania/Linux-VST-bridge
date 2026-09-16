#!/usr/bin/env python3
"""Build and seal one fixed source-owned x86 package from committed source."""
import argparse,os,pathlib,shutil,subprocess
from identity import SOURCES,MANIFEST,atomic_new,read_json,digest,architecture,verify_package

def build(repo,output,context,zig):
    repo=pathlib.Path(repo).resolve();output=pathlib.Path(output);zig=pathlib.Path(zig).resolve()
    def git(*args):return subprocess.check_output(['git','-C',str(repo),*args]).decode().strip()
    source={'head':git('rev-parse','HEAD'),'tree':git('rev-parse','HEAD^{tree}')}
    if git('status','--porcelain','--untracked-files=no'):raise ValueError('commit_source_before_package')
    output.mkdir(mode=0o700)
    for name,relative in SOURCES.items():
        committed=subprocess.check_output(['git','-C',str(repo),'show',source['head']+':'+relative])
        if (repo/relative).read_bytes()!=committed:raise ValueError('source_drift')
        (output/name).write_bytes(committed)
    version=subprocess.check_output([str(zig),'version'],timeout=10).decode().strip()
    subprocess.run([str(zig),'c++','-target','x86-windows-gnu','-std=c++20','-O2','-municode','-DUNICODE','-D_UNICODE',str(output/'capability.cpp'),'-lbcrypt','-o',str(output/'payload.exe')],check=True,timeout=120)
    c=read_json(context)
    if set(c)!={'runner','installed','powershell_images','baseline_windows_environment'}:raise ValueError('context_schema')
    payload=output/'payload.exe'
    m={'schema':2,'source':source,'files':{p.name:digest(p) for p in output.iterdir()},'payload':{'sha256':digest(payload),'size':payload.stat().st_size,'architecture':architecture(payload)},**c,'build':{'recipe':'zig-x86-cpp20-bcrypt-v1','compiler_sha256':digest(zig),'compiler_version':version}}
    for p in output.iterdir():p.chmod(0o400)
    atomic_new(output/MANIFEST,m);seal=digest(output/MANIFEST)
    verify_package(output,seal,source)
    return {'manifest_sha256':seal,'source':source}

if __name__=='__main__':
    import json
    p=argparse.ArgumentParser();p.add_argument('--repo',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--context',type=pathlib.Path,required=True);p.add_argument('--zig',type=pathlib.Path,required=True);a=p.parse_args();os.umask(0o077)
    print(json.dumps(build(a.repo,a.output,a.context,a.zig)))
