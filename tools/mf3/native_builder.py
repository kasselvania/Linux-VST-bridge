"""Fixed offline build recipe. Input is emitted only by the canonical manager owner.
The SDK is an already installed, pinned dependency; no network or downloads.
"""
import hashlib, json, os, pathlib, stat, subprocess, sys, threading, zipfile
SDK='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96'
SDK_RUNTIME='b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8'
def digest(data): return hashlib.sha256(data).hexdigest()
def run(argv, timeout=600, log=None):
    process=subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    tail=bytearray(); dropped=0
    def drain():
        nonlocal dropped
        while block:=process.stdout.read(8192):
            tail.extend(block)
            if len(tail)>32768:
                dropped+=len(tail)-32768; del tail[:-32768]
    thread=threading.Thread(target=drain,daemon=True);thread.start()
    try: code=process.wait(timeout)
    except subprocess.TimeoutExpired:
        process.terminate()
        try: process.wait(10)
        except subprocess.TimeoutExpired: process.kill();process.wait()
        raise ValueError('native_build_deadline')
    finally: thread.join(10)
    if thread.is_alive():raise ValueError('native_build_pipe_retirement_unconfirmed')
    process.stdout.close()
    if log is not None:
        previous=log.read_bytes() if log.exists() else b''
        log.write_bytes((previous+bytes(tail))[-32768:])
    if code: raise ValueError('native_build_failed_'+str(code))
    return bytes(tail),dropped

def build(request, generator):
    source=pathlib.Path(request['directory'])
    archive=pathlib.Path(request['kit'])
    # The manager has verified the exact immutable archive before invoking us.
    assert archive.is_file() and digest(archive.read_bytes())==request['kit_sha256']
    source.mkdir(mode=0o700)
    with zipfile.ZipFile(archive) as z:
        names=z.namelist(); assert len(names)==len(set(names)) and len(names)<=512
        assert z.getinfo('recipe.json').file_size<=65536
        recipe=json.loads(z.read('recipe.json'))
        assert recipe['schema']==1 and recipe['sdk']==SDK and recipe['sdk_runtime']==SDK_RUNTIME
        assert set(names)==set(recipe['files'])|{'recipe.json'}
        total=0
        for name,sha in recipe['files'].items():
            p=pathlib.PurePosixPath(name); info=z.getinfo(name)
            assert not p.is_absolute() and '..' not in p.parts and not info.is_dir()
            assert stat.S_IFMT(info.external_attr>>16)!=stat.S_IFLNK
            total+=info.file_size; assert info.file_size<=128*1024*1024 and total<=256*1024*1024
            data=z.read(name);assert digest(data)==sha
            target=source/p;target.parent.mkdir(mode=0o700,parents=True,exist_ok=True);target.write_bytes(data)
    assert 'libap2_backend.a' in recipe['files']
    sdk=pathlib.Path.home()/'.cache/linux-vst-bridge/dependencies/vst3sdk'/SDK
    if not sdk.is_dir():raise ValueError('build_prerequisite_pinned_vst3_sdk_missing')
    runtime,_=run(['flatpak','info','--user','--show-commit','org.freedesktop.Sdk//25.08'],30,source/'build.log')
    if runtime.decode().strip()!=SDK_RUNTIME:raise ValueError('build_prerequisite_pinned_sdk_runtime_missing')
    records=json.loads(pathlib.Path(request['inspection']).read_text())['records']
    descriptor=generator(records,request['class_id'],request['module_sha256'])
    (source/'ap8_descriptor.h').write_text(descriptor)
    base=['flatpak','run','--user','--unshare=network','--nofilesystem=host','--nofilesystem=home',
          '--filesystem='+str(source),'--filesystem='+str(sdk)+':ro','--command=cmake','org.freedesktop.Sdk//25.08']
    _,drop_a=run(base+['-S',str(source),'-B',str(source/'build'),'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DAP2_BUILD_ONLY=ON','-DVST3_SDK_ROOT='+str(sdk),'-DAP2_RUST_LIBRARY='+str(source/'libap2_backend.a'),'-DAP8_DESCRIPTOR='+str(source/'ap8_descriptor.h')],log=source/'build.log')
    _,drop_b=run(base+['--build',str(source/'build'),'--target','CommercialInstrumentBridge','-j','4'],log=source/'build.log')
    artifact=source/'build/VST3/Release/CommercialInstrumentBridge.vst3/Contents/x86_64-linux/CommercialInstrumentBridge.so'
    assert artifact.is_file() and artifact.resolve()==artifact
    data=artifact.read_bytes();assert data[:4]==b'\x7fELF' and len(data)<128*1024*1024
    (source/'native.so').write_bytes(data)
    return dict(schema=1,source_commit=recipe['source_commit'],native_sha256=digest(data),descriptor_sha256=digest(descriptor.encode()),dropped_bytes=drop_a+drop_b,sdk=SDK,sdk_runtime=SDK_RUNTIME)
# entrypoint appended by the manager; generator is embedded from its exact source.
