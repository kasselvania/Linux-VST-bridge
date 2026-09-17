"""Bounded offline artifact/registry/process census. No Windows execution or TCP request."""
import hashlib,os,pathlib,re,stat,time
from common import read,require,file_identity,opened,stamp,digest
from pe_metadata import metadata as pe
SERVICE='NTKDaemonService'
DAEMON='Program Files/Common Files/Native Instruments/NTK/NTKDaemon.exe'
BUNDLE='Program Files/Native Instruments/Native Access/resources/daemon/win'
def image(path):
    before=file_identity(path,256*1024*1024)
    with opened(path) as f:metadata=pe(f)
    require(file_identity(path,256*1024*1024)==before,'image_changed')
    return dict(before,pe=metadata)
def bundle(drive):
    directory=drive/BUNDLE
    if not directory.exists():return []
    require(directory.resolve()==directory,'bundle_alias')
    entries=list(directory.iterdir());require(len(entries)<=64,'bundle_extent')
    matches=[p for p in entries if re.fullmatch(r'NTKDaemon[ A-Za-z0-9._-]*Setup PC\.exe',p.name,re.I)]
    require(len(matches)<=1,'ambiguous_installer')
    return [dict(image(p),relative=BUNDLE+'/'+p.name) for p in matches]
def registry(raw):
    require(len(raw)<=32*1024*1024,'registry_extent')
    text=raw.decode('utf8',errors='strict');require(text.startswith('WINE REGISTRY Version 2'),'registry_format')
    blocks=re.split(r'(?m)^\[',text);out=[]
    for block in blocks[1:]:
        header,_,body=block.partition('\n');key=header.split(']',1)[0].replace('\\\\','\\').casefold()
        if key=='system\\currentcontrolset\\services\\'+SERVICE.casefold():
            fields={}
            for line in body.splitlines():
                if line.startswith('"') and '"=' in line:
                    k,value=line[1:].split('"=',1);require(k not in fields,'duplicate_service_field');fields[k]=value
            out.append({'sha256':digest(block.encode()),'fields':fields})
    require(len(out)<=1,'duplicate_service_registration')
    return out

def metadata(root):
    out={};pending=[root];deadline=time.monotonic()+60
    while pending:
        d=pending.pop()
        with os.scandir(d) as entries:
            for e in entries:
                require(len(out)<100000 and time.monotonic()<deadline,'metadata_bound')
                s=e.stat(follow_symlinks=False);p=d/e.name
                out[str(p.relative_to(root))]=[s.st_mode,s.st_size,s.st_mtime_ns,s.st_ctime_ns,s.st_dev,s.st_ino]
                if stat.S_ISDIR(s.st_mode):pending.append(p)
    return out

def proc_census(prefix,proc=pathlib.Path('/proc'),admitted=None):
    from dependency_process import census
    return census(prefix,admitted,proc)

def listeners(proc=pathlib.Path('/proc')):
    rows=[]
    for name in ['tcp','tcp6']:
        with (proc/'net'/name).open('rb') as stream:raw=stream.read(8*1024*1024+1)
        require(len(raw)<=8*1024*1024 and raw.count(b'\n')<=65536,'socket_extent')
        for line in raw.decode().splitlines()[1:]:
            fields=line.split();require(len(fields)>=10,'socket_record')
            address,port=fields[1].split(':');port=int(port,16)
            if port in (5146,5563) and fields[3]=='0A':
                rows.append({'port':port,'loopback':address in ('0100007F','00000000000000000000000001000000'),'inode_sha256':digest(fields[9].encode()),'authority':'passive_listener_only_not_readiness'})
    return rows
