#!/usr/bin/env python3
"""Build the prepared commercial descriptor with the retained native build lane."""
import argparse,hashlib,json,os,pathlib,subprocess,zipfile
from ap2_build_native import ROOT,SDK,SDK_RUNTIME,transfer_port
from pc0_proof_adapter import StrictSSHPort,SubprocessCommandPort

def main():
    p=argparse.ArgumentParser();p.add_argument('--descriptor',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--performance',action='store_true');p.add_argument('--registered',action='store_true');a=p.parse_args()
    if subprocess.check_output(['git','status','--porcelain'],cwd=ROOT):raise RuntimeError('commit build inputs first')
    source=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    compiler=subprocess.check_output(['rustup','which','--toolchain','stable','rustc'],text=True).strip()
    subprocess.run(['rustup','run','stable','cargo','build','--manifest-path','native-vst3-proxy/backend/Cargo.toml','--release','--locked','--offline','--target','x86_64-unknown-linux-gnu']+(['--features','registered'] if a.registered else []),cwd=ROOT,env={**os.environ,'RUSTC':compiler,'RUSTFLAGS':'-C relocation-model=pic'},check=True)
    files=subprocess.check_output(['git','ls-files','CMakeLists.txt','cmake/HP0Vst3SdkLock.cmake','cmake/HP0ModernGcc.cmake','native-vst3-proxy','vst-state'],cwd=ROOT,text=True).splitlines()
    a.output.mkdir(parents=True,exist_ok=False);archive=a.output/'native-input.zip'
    with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED) as z:
        for f in files:z.write(ROOT/f,f)
        z.write(a.descriptor,'ap8_descriptor.h');z.write(ROOT/'native-vst3-proxy/backend/target/x86_64-unknown-linux-gnu/release/libap2_backend.a','libap2_backend.a')
    key=hashlib.sha256(archive.read_bytes()).hexdigest();parent='/home/deck/.cache/linux-vst-bridge/'+('ap9-builds' if a.performance else 'ap8-builds');ssh=transfer_port();ssh.run('mkdir -p '+parent);ssh.copy(archive,parent+'/'+key+'.zip')
    program=f'PARENT={parent!r}\nKEY={key!r}\nSDK={SDK!r}\nSDK_RUNTIME={SDK_RUNTIME!r}\n'+r'''
import pathlib,hashlib,zipfile,subprocess,json
root=pathlib.Path(PARENT);archive=root/(KEY+'.zip');source=root/KEY
assert hashlib.sha256(archive.read_bytes()).hexdigest()==KEY
source.mkdir(mode=0o700)
with zipfile.ZipFile(archive) as z:
 for i in z.infolist():
  assert not i.is_dir() and not i.filename.startswith('/') and '..' not in pathlib.PurePosixPath(i.filename).parts and i.file_size<64*1024*1024
  p=source/i.filename;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(z.read(i))
sdk=pathlib.Path.home()/'.cache/linux-vst-bridge/dependencies/vst3sdk'/SDK
assert subprocess.check_output(['flatpak','info','--user','--show-commit','org.freedesktop.Sdk//25.08'],text=True).strip()==SDK_RUNTIME
base=['flatpak','run','--user','--unshare=network','--nofilesystem=host','--nofilesystem=home','--filesystem='+str(source),'--filesystem='+str(sdk)+':ro','--command=sh','org.freedesktop.Sdk//25.08','-c','exec "$@"','ap8-build']
commands=[['cmake','-S',str(source),'-B',str(source/'build'),'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DAP2_BUILD_ONLY=ON','-DVST3_SDK_ROOT='+str(sdk),'-DAP2_RUST_LIBRARY='+str(source/'libap2_backend.a'),'-DAP8_DESCRIPTOR='+str(source/'ap8_descriptor.h')],['cmake','--build',str(source/'build'),'--target','CommercialInstrumentBridge','AGainQueuedBridge','ap3-sustained-host','ap3-callback-audit','-j','4']]
for i,cmd in enumerate(commands):
 r=subprocess.run(base+cmd,capture_output=True,text=True,timeout=600);log=r.stdout+'\n'+r.stderr;(source/('build-'+str(i)+'.log')).write_text(log)
 if r.returncode:print(json.dumps({'returncode':r.returncode,'stage':i,'detail':log[-12000:]}));raise SystemExit(0)
bundle=source/'build/VST3/Release/CommercialInstrumentBridge.vst3'
print(json.dumps({'returncode':0,'bundle':str(bundle),'sha256':hashlib.sha256((bundle/'Contents/x86_64-linux/CommercialInstrumentBridge.so').read_bytes()).hexdigest()}))
'''
    raw=StrictSSHPort(SubprocessCommandPort()).run_python('/home/deck',program,[],timeout=700);(a.output/'build.json').write_bytes(raw);reply=json.loads(raw)
    record={**reply,'source_commit':source,'input_archive_sha256':key,'descriptor_sha256':hashlib.sha256(a.descriptor.read_bytes()).hexdigest(),'sdk':SDK,'sdk_runtime':SDK_RUNTIME};(a.output/'receipt.json').write_text(json.dumps(record,indent=2));print(json.dumps(record))
    if reply['returncode']:raise SystemExit(1)
if __name__=='__main__':main()
