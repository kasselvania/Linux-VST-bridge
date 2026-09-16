#!/usr/bin/env python3
"""Build and seal one fixed source-owned x86 package from committed source."""
import argparse,os,pathlib,shutil,subprocess,tempfile
from identity import SOURCES,MANIFEST,atomic_new,read_json,digest,architecture,verify_package

def build(repo,output,context,zig):
    repo=pathlib.Path(repo).resolve();output=pathlib.Path(output);zig=pathlib.Path(zig).resolve()
    def git(*args):return subprocess.check_output(['git','-C',str(repo),*args]).decode().strip()
    source={'head':git('rev-parse','HEAD'),'tree':git('rev-parse','HEAD^{tree}')}
    if git('status','--porcelain','--untracked-files=no'):raise ValueError('commit_source_before_package')
    output.mkdir(mode=0o700)
    for name,relative in SOURCES.items():
        if relative is None:continue
        committed=subprocess.check_output(['git','-C',str(repo),'show',source['head']+':'+relative])
        if (repo/relative).read_bytes()!=committed:raise ValueError('source_drift')
        (output/name).write_bytes(committed)
    version=subprocess.check_output([str(zig),'version'],timeout=10).decode().strip()
    # Compiler sidecars (e.g. PDB) are build outputs, not executable inputs.
    # Keep them outside the closed staging directory; copy only the fixed PE.
    with tempfile.TemporaryDirectory(prefix='is3-build-',dir=output.parent) as temp:
        built=pathlib.Path(temp)/'payload.exe'
        subprocess.run([str(zig),'c++','-target','x86-windows-gnu','-std=c++20','-O2','-municode','-DUNICODE','-D_UNICODE',str(output/'capability.cpp'),'-lbcrypt','-o',str(built)],check=True,timeout=120)
        shutil.copyfile(built,output/'payload.exe')
    rustc=subprocess.check_output(['rustup','which','rustc']).decode().strip()
    rust_version=subprocess.check_output([rustc,'--version']).decode().strip()
    env=dict(os.environ,IS4_ZIG=str(zig),CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=str(repo/'tools/is4/linker.py'))
    subprocess.run(['cargo','build','--manifest-path',str(repo/'bridge-manager/Cargo.toml'),'--locked','--release','--target','x86_64-unknown-linux-gnu','--example','is4_policy'],env=env,check=True,timeout=240)
    shutil.copyfile(repo/'bridge-manager/target/x86_64-unknown-linux-gnu/release/examples/is4_policy',output/'policy-owner')
    c=read_json(context)
    if set(c)!={'runner','installed','powershell_images','baseline_windows_environment'}:raise ValueError('context_schema')
    payload=output/'payload.exe'
    m={'schema':2,'source':source,'files':{p.name:digest(p) for p in output.iterdir()},'payload':{'sha256':digest(payload),'size':payload.stat().st_size,'architecture':architecture(payload)},**c,'build':{'recipe':'is4-zig-x86-cpp20-rust-policy-v1','compiler_sha256':digest(zig),'compiler_version':version,'rustc_sha256':digest(rustc),'rustc_version':rust_version}}
    for p in output.iterdir():p.chmod(0o500 if p.name=='policy-owner' else 0o400)
    atomic_new(output/MANIFEST,m);seal=digest(output/MANIFEST)
    verify_package(output,seal,source)
    return {'manifest_sha256':seal,'source':source}

if __name__=='__main__':
    import json
    p=argparse.ArgumentParser();p.add_argument('--repo',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--context',type=pathlib.Path,required=True);p.add_argument('--zig',type=pathlib.Path,required=True);a=p.parse_args();os.umask(0o077)
    print(json.dumps(build(a.repo,a.output,a.context,a.zig)))
