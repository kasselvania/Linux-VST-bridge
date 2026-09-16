import copy,hashlib,pathlib,struct
from identity import SOURCES,OWNERS,canonical,hashed,envelope,atomic_new,digest,MANIFEST

def fixture_manifest(payload=b'owned'):
    artifact={'sha256':'a'*64,'size':12,'location_sha256':'b'*64}
    runner={'id':'pinned','record_sha256':'c'*64,'entry_point':artifact,'proton':artifact,'files':[artifact]}
    return {'schema':2,'source':{'head':'a'*40,'tree':'b'*40},'files':{**{n:'d'*64 for n in SOURCES},'payload.exe':hashlib.sha256(payload).hexdigest()},'payload':{'sha256':hashlib.sha256(payload).hexdigest(),'size':len(payload),'architecture':'x86'},'runner':runner,'powershell_images':{a:{'sha256':'e'*64,'size':100} for a in ('system32','syswow64')},'installed':{'record_sha256':'f'*64,'artifacts':{k:artifact for k in OWNERS}},'baseline_windows_environment':{'present':True,'utf16_code_units':7,'sha256_utf16le':hashlib.sha256('prior=n'.encode('utf-16le')).hexdigest(),'duplicate_count':0},'build':{'recipe':'zig-x86-cpp20-bcrypt-v1','compiler_sha256':'a'*64,'compiler_version':'fixture'}}

def fake_package(path):
    path=pathlib.Path(path);path.mkdir()
    payload=bytearray(128);payload[:2]=b'MZ';struct.pack_into('<I',payload,60,64);payload[64:68]=b'PE\0\0';struct.pack_into('<H',payload,68,0x14c)
    m=fixture_manifest(bytes(payload))
    for n in SOURCES:(path/n).write_bytes(n.encode())
    (path/'payload.exe').write_bytes(payload)
    m['files']={p.name:digest(p) for p in path.iterdir()}
    for p in path.iterdir():p.chmod(0o400)
    atomic_new(path/MANIFEST,m)
    return m,digest(path/MANIFEST)
