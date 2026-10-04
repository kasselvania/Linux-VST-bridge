#!/usr/bin/env python3
"""Build one reusable native engine; plug-in discovery data is prepared on installation."""
import argparse, hashlib, json, os, pathlib, subprocess, tempfile, zipfile, sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]))
from native_builder import SDK,SDK_RUNTIME
ROOT=pathlib.Path(__file__).resolve().parents[2]
SOURCES=('CMakeLists.txt','cmake/HP0Vst3SdkLock.cmake','cmake/HP0ModernGcc.cmake',
         'native-vst3-proxy','vst-state','native-audio-client','plugin-descriptor')
def sha(data):return hashlib.sha256(data).hexdigest()
def git(*args):return subprocess.check_output(['git','-C',str(ROOT),*args],text=True).strip()
def source_roster():
    return {name:sha((ROOT/name).read_bytes()) for name in git('ls-files',*SOURCES).splitlines()}
def compile_proxy(source,sdk,backend,target):
    subprocess.run(['cmake','-S',str(source),'-B',str(target),'-G','Ninja',
        '-DCMAKE_BUILD_TYPE=Release','-DAP2_BUILD_ONLY=ON',
        '-DVST3_SDK_ROOT='+str(sdk),'-DAP2_RUST_LIBRARY='+str(backend),
        '-DLVB_REUSABLE_ENGINE=ON'],check=True,timeout=120)
    subprocess.run(['cmake','--build',str(target),'--target','CommercialInstrumentBridge','-j','2'],
        check=True,timeout=600)
    return (target/'VST3/Release/CommercialInstrumentBridge.vst3/Contents/x86_64-linux/CommercialInstrumentBridge.so').read_bytes()
def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--sdk',type=pathlib.Path,required=True)
    parser.add_argument('--windows-package',type=pathlib.Path,required=True)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    a=parser.parse_args()
    if git('status','--porcelain'):raise SystemExit('Commit the exact build inputs first')
    subprocess.run(['cargo','+1.95.0','build','--manifest-path',str(ROOT/'native-vst3-proxy/backend/Cargo.toml'),
        '--release','--locked','--target','x86_64-unknown-linux-gnu','--features','registered'],
        env={**os.environ,'RUSTFLAGS':'-C relocation-model=pic'},check=True,timeout=600)
    backend=ROOT/'native-vst3-proxy/backend/target/x86_64-unknown-linux-gnu/release/libap2_backend.a'
    files={'libap2_backend.a':backend.read_bytes(),
        'tools/mf3/native_builder.py':(ROOT/'tools/mf3/native_builder.py').read_bytes(),
        'tools/ap8_descriptor.py':(ROOT/'tools/ap8_descriptor.py').read_bytes()}
    for component in ('', 'base', 'pluginterfaces', 'public.sdk'):
        name=(component or 'vst3sdk')+'.txt'
        files['licenses/'+name]=(a.sdk/component/'LICENSE.txt').read_bytes()
    for name in ('host.exe','host-source-manifest.json'):
        files['runtime/'+name]=(a.windows_package/('wf0-factory-probe.exe' if name=='host.exe' else name)).read_bytes()
    manifest=json.loads(files['runtime/host-source-manifest.json'])
    assert manifest['host_sha256']==sha(files['runtime/host.exe'])
    for name,expected in manifest['files'].items():
        data=(ROOT/name).read_bytes()
        assert expected in (sha(data),sha(data.replace(b'\n',b'\r\n'))),'Windows host source differs: '+name
    with tempfile.TemporaryDirectory(prefix='lvb-engine-') as temp:
        native=compile_proxy(ROOT,a.sdk,backend,pathlib.Path(temp)/'build')
    files['prebuilt/engine.so']=native
    index=dict(schema=3,engine='prebuilt/engine.so',engine_sha256=sha(native),
        descriptor_schema=1,maximum_bridge_frames=1024,audio_completion_contract=1,native_sources=source_roster())
    files['prebuilt/index.json']=json.dumps(index,sort_keys=True,separators=(',',':')).encode()
    recipe=dict(schema=4,source_commit=git('rev-parse','HEAD'),sdk=SDK,sdk_runtime=SDK_RUNTIME,
        files={name:sha(data) for name,data in files.items()})
    a.output.parent.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(a.output,'x',compression=zipfile.ZIP_DEFLATED) as z:
        for name,data in files.items():z.writestr(name,data)
        z.writestr('recipe.json',json.dumps(recipe,sort_keys=True))
    print(json.dumps(dict(kit_sha256=sha(a.output.read_bytes()),engine_sha256=sha(native),source=recipe['source_commit'])))
if __name__=='__main__':main()
