"""Small exact-binary verifier for the AP1 native caller; no execution API."""
import hashlib,json,pathlib,struct
PATHS=('native-audio-client/Cargo.toml','native-audio-client/Cargo.lock',
       'native-audio-client/src/lib.rs','native-audio-client/src/main.rs')
def canonical(v):return (json.dumps(v,sort_keys=True,separators=(',',':'))+'\n').encode()
def digest(b):return hashlib.sha256(b).hexdigest()
def elf(raw):
    if (len(raw)<64 or raw[:6]!=b'\x7fELF\x02\x01' or struct.unpack_from('<H',raw,18)[0]!=62):
        raise RuntimeError('AP1 caller is not x86-64 ELF')
    offset=struct.unpack_from('<Q',raw,32)[0];size,count=struct.unpack_from('<HH',raw,54)
    if size!=56 or not 1<=count<=64 or offset+size*count>len(raw):raise RuntimeError('AP1 ELF program headers differ')
    for i in range(count):
        p=offset+i*size;kind=struct.unpack_from('<I',raw,p)[0]
        if kind==3:raise RuntimeError('AP1 caller must be static, no interpreter')
        if kind==2:
            start,n=struct.unpack_from('<Q',raw,p+8)[0],struct.unpack_from('<Q',raw,p+32)[0]
            if start+n>len(raw) or n%16:raise RuntimeError('AP1 ELF dynamic bounds')
            if any(struct.unpack_from('<Q',raw,j)[0]==1 for j in range(start,start+n,16)):
                raise RuntimeError('AP1 caller has dynamic library dependency')
def verify_client(root,binding):
    root=pathlib.Path(root)
    for p in (root,root/'AP1_CLIENT_BUILD.json',root/'ap1-native-client'):
        if p.is_symlink():raise RuntimeError('AP1 caller symlink forbidden')
    raw=(root/'AP1_CLIENT_BUILD.json').read_bytes();record=json.loads(raw)
    if canonical(record)!=raw or digest(raw)!=binding['manifest_sha256']:raise RuntimeError('AP1 native manifest differs')
    if (record['source_commit']!=binding['source_commit'] or record['input_sha256']!=binding['input_sha256']
            or record['binary_sha256']!=binding['binary_sha256']):raise RuntimeError('AP1 native binding differs')
    binary=(root/'ap1-native-client').read_bytes()
    if len(binary)>4*1024*1024 or digest(binary)!=binding['binary_sha256']:raise RuntimeError('AP1 caller bytes differ')
    elf(binary);return record
