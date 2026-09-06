#!/usr/bin/env python3
"""Build exact AP2 native bytes with the retained SDK and the existing transfer lane."""
import argparse,ast,hashlib,json,os,pathlib,re,shlex,subprocess,sys,tempfile,zipfile
from ap2_native_artifact import PATHS,NAMES,AP3_NAMES,canonical,digest,verify_native
ROOT=pathlib.Path(__file__).resolve().parents[1]
SDK='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96'
SDK_RUNTIME='b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8'
def command(args,**kwargs):return subprocess.check_output(args,cwd=ROOT,**kwargs).decode().strip()
def transfer_port():
 sys.path.insert(0,str(ROOT/'tools/wf0-factory-census'));import common
 raw=subprocess.check_output(['git','cat-file','blob','beb57c62b94f3d99fb87f56db3b20b20553064c4'],cwd=ROOT)
 if hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!='beb57c62b94f3d99fb87f56db3b20b20553064c4':raise RuntimeError('retained transfer source differs')
 node=next(n for n in ast.parse(raw).body if isinstance(n,ast.ClassDef) and n.name=='SSHAdapter')
 namespace={**vars(common),'pathlib':pathlib,'subprocess':subprocess,'shlex':shlex,'re':re}
 exec(compile(ast.get_source_segment(raw.decode(),node),'<retained-transfer>','exec'),namespace)
 return namespace['SSHAdapter']()
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--profile',choices=('ap2','ap3','ap4'),default='ap2');parser.add_argument('--output',type=pathlib.Path,required=True);parser.add_argument('--binding-output',type=pathlib.Path);args=parser.parse_args()
 ap4=args.profile=='ap4';ap3=args.profile in {'ap3','ap4'};names=AP3_NAMES if ap3 else NAMES;manifest_name='AP4_NATIVE_BUILD.json' if ap4 else 'AP3_NATIVE_BUILD.json' if ap3 else 'AP2_NATIVE_BUILD.json'
 if command(['git','status','--porcelain=v1','--untracked-files=all']):raise RuntimeError('native build needs clean committed source')
 source=command(['git','rev-parse','HEAD']);records=[{'path':p,'git_blob':command(['git','rev-parse',source+':'+p])} for p in PATHS]
 compiler=command(['rustup','which','--toolchain','stable','rustc']);version=command([compiler,'--version'])
 env={**os.environ,'RUSTC':compiler,'RUSTFLAGS':'-C relocation-model=pic'}
 subprocess.run(['rustup','run','stable','cargo','build','--manifest-path','native-vst3-proxy/backend/Cargo.toml','--release','--locked','--offline','--target','x86_64-unknown-linux-gnu'],cwd=ROOT,env=env,check=True)
 args.output.mkdir(parents=True,exist_ok=True)
 archive=args.output/'native-input.zip'
 with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED) as z:
  for p in PATHS:z.write(ROOT/p,p)
  z.write(ROOT/'native-vst3-proxy/backend/target/x86_64-unknown-linux-gnu/release/libap2_backend.a','libap2_backend.a')
 key=digest(archive.read_bytes());ssh=transfer_port();parent='/home/deck/.cache/linux-vst-bridge/'+args.profile+'-builds'
 ssh.run('mkdir -p '+shlex.quote(parent));ssh.copy(archive,parent+'/'+key+'.zip')
 from pc0_proof_adapter import StrictSSHPort,SubprocessCommandPort
 program=r'''
import pathlib,hashlib,zipfile,subprocess,json,shutil
parent=pathlib.Path.home()/('.cache/linux-vst-bridge/'+PROFILE+'-builds');source=parent/KEY;archive=parent/(KEY+'.zip')
assert hashlib.sha256(archive.read_bytes()).hexdigest()==KEY
assert not source.exists();source.mkdir(mode=0o700)
with zipfile.ZipFile(archive) as z:
 for i in z.infolist():
  assert not i.is_dir() and not i.filename.startswith('/') and '..' not in pathlib.PurePosixPath(i.filename).parts and i.file_size<64*1024*1024
  p=source/i.filename;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(z.read(i))
sdk=pathlib.Path.home()/'.cache/linux-vst-bridge/dependencies/vst3sdk'/SDK
commit=subprocess.check_output(['flatpak','info','--user','--show-commit','org.freedesktop.Sdk//25.08'],text=True).strip();assert commit==SDK_RUNTIME
base=['flatpak','run','--user','--unshare=network','--nofilesystem=host','--nofilesystem=home','--filesystem='+str(source),'--filesystem='+str(sdk)+':ro','--command=sh','org.freedesktop.Sdk//25.08','-c','exec "$@"','ap2-build']
commands=[['cmake','-S',str(source),'-B',str(source/'build'),'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DAP2_BUILD_ONLY=ON','-DVST3_SDK_ROOT='+str(sdk),'-DAP2_RUST_LIBRARY='+str(source/'libap2_backend.a')],['cmake','--build',str(source/'build'),'--target','AGainOfflineBridge','ap2-offline-host',*(['AGainQueuedBridge','ap3-sustained-host','ap3-callback-audit'] if AP3 else []),'-j','4']]
for index,cmd in enumerate(commands):
 p=subprocess.run(base+cmd,capture_output=True,text=True,timeout=600);log=p.stdout+'\n'+p.stderr;(source/('build-'+str(index)+'.log')).write_text(log)
 if p.returncode:print(json.dumps({'returncode':p.returncode,'stage':index,'detail':log[-8000:].replace(str(pathlib.Path.home()),'<home>')}));raise SystemExit(0)
paths=[source/'build/native-vst3-proxy/ap2-offline-host',source/'build/VST3/Release/AGainOfflineBridge.vst3/Contents/x86_64-linux/AGainOfflineBridge.so']
if AP3:paths += [source/'build/native-vst3-proxy/ap3-sustained-host',source/'build/VST3/Release/AGainQueuedBridge.vst3/Contents/x86_64-linux/AGainQueuedBridge.so',source/'build/native-vst3-proxy/libap3-callback-audit.so']
output=source/'native-output.zip'
with zipfile.ZipFile(output,'w',compression=zipfile.ZIP_DEFLATED) as z:
 for path,name in zip(paths,NAMES):z.write(path,name)
 z.write(sdk/'LICENSE.txt','VST3_SDK_LICENSE.txt')
print(json.dumps({'returncode':0,'output_sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'binaries':[{'path':n,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p,n in zip(paths,NAMES)],'license_sha256':hashlib.sha256((sdk/'LICENSE.txt').read_bytes()).hexdigest()}))
'''
 program='AP3='+repr(ap3)+'\nPROFILE='+repr(args.profile)+'\nKEY='+repr(key)+'\nSDK='+repr(SDK)+'\nSDK_RUNTIME='+repr(SDK_RUNTIME)+'\nNAMES='+repr(names)+'\n'+program
 raw=StrictSSHPort(SubprocessCommandPort()).run_python('/home/deck',program,[],timeout=700)
 (args.output/'build.json').write_bytes(raw);reply=json.loads(raw)
 if reply['returncode']:raise RuntimeError('native build failed; bounded details retained in output/build.json')
 output=args.output/'native-output.zip';ssh.fetch(parent+'/'+key+'/native-output.zip',output)
 if digest(output.read_bytes())!=reply['output_sha256']:raise RuntimeError('native return transfer differs')
 record={'schema':args.profile+'-native-build/v1','source_commit':source,'input_sha256':digest(canonical(records)),'records':records,'rustc':version,'rust_target':'x86_64-unknown-linux-gnu','pic':True,'sdk':SDK,'sdk_runtime_commit':SDK_RUNTIME,'compiler':'GCC 15.2.0','binaries':reply['binaries'],'license_sha256':reply['license_sha256'],'build_archive_sha256':key}
 manifest=digest(canonical(record));binding={'source_commit':source,'input_sha256':record['input_sha256'],'manifest_sha256':manifest}
 from ap2_adapter import native_parent
 root=native_parent()/manifest;root.mkdir(parents=True,exist_ok=False)
 with zipfile.ZipFile(output) as z:
  if sorted(z.namelist())!=sorted([*names,'VST3_SDK_LICENSE.txt']):raise RuntimeError('native return roster differs')
  for name in z.namelist():p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(z.read(name));p.chmod(0o444)
 (root/manifest_name).write_bytes(canonical(record));verify_native(root,binding,ap3,ap4)
 ssh.publish_tree(root,'/home/deck/.local/share/linux-vst-bridge/native-artifacts/by-manifest',manifest)
 program=(ROOT/'tools/ap1_client_artifact.py').read_text()+'\n'+(ROOT/'tools/ap2_native_artifact.py').read_text().replace('from ap1_client_artifact import canonical,digest','')
 program+='\nap4='+repr(ap4)+'\nap3='+repr(ap3)+'\nbinding='+repr(binding)+"\nroot=pathlib.Path.home()/'.local/share/linux-vst-bridge/native-artifacts/by-manifest'/binding['manifest_sha256']\nverify_native(root,binding,ap3,ap4)\n[ (root/n).chmod(0o500) for n in ('ap2-offline-host', 'ap3-sustained-host') if (root/n).exists() ]\nprint('native artifact verified')\n"
 StrictSSHPort(SubprocessCommandPort()).run_python('/home/deck',program,[],timeout=30)
 (args.binding_output or ROOT/('docs/campaigns/'+args.profile.upper()+'_NATIVE.json')).write_bytes(canonical(binding));(args.output/'binding.json').write_bytes(canonical(binding))
 print(json.dumps({'native_manifest':manifest,'source':source,'workload_calls':0}))
if __name__=='__main__':main()
