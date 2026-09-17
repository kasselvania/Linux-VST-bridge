"""Bounded offline artifact/registry/process census. No Windows execution or TCP request."""
import hashlib,os,pathlib,re,stat,time
from common import read,require,file_identity,opened,stamp,digest
from identity import pe
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

def proc_census(prefix,proc=pathlib.Path('/proc')):
    """Names are leads only. Positive process authority requires mapped PE and prefix.

    Never signal/adopt a process; unreadable potential candidates remain ambiguity.
    The public record omits raw PID/environment/command lines.
    """
    candidates=[];unknown=0;visited=0;deadline=time.monotonic()+30
    for p in proc.iterdir():
        if not p.name.isdigit():continue
        visited+=1;require(visited<=32768 and time.monotonic()<deadline,'process_census_bound')
        try:
            if p.stat().st_uid!=os.getuid():continue
            cmd=(p/'cmdline').read_bytes()[:65537]
            require(len(cmd)<=65536,'command_extent')
            maps=(p/'maps').read_bytes()[:2*1024*1024+1]
            if b'ntkdaemon' not in cmd.lower() and b'ntkdaemon' not in maps.lower():continue
            require(len(maps)<=2*1024*1024,'maps_extent')
            before=(p/'stat').read_text();start=before[before.rfind(')')+2:].split()[19]
            env=(p/'environ').read_bytes()[:262145];require(len(env)<=262144,'environment_extent')
            values={x.split(b'=',1)[0]:x.split(b'=',1)[1] for x in env.split(b'\0') if b'=' in x}
            wp=values.get(b'WINEPREFIX');relation='unavailable'
            if wp:
                q=pathlib.Path(os.fsdecode(wp));relation='deleted' if not q.exists() else 'same' if q.resolve()==prefix.resolve() else 'foreign'
            matches=set()
            for line in maps.decode('utf8','replace').splitlines():
                f=line.split(None,5)
                if len(f)==6 and f[5].lower().endswith('/ntkdaemon.exe'):matches.add(f[5])
            imgs=[image(pathlib.Path(x)) for x in matches]
            after=(p/'stat').read_text();require(after[after.rfind(')')+2:].split()[19]==start,'process_reused')
            exact=len(imgs)==1 and any('ntk' in v.lower() for v in imgs[0]['pe']['version'].values()) and relation!='unavailable'
            if not exact:unknown+=1
            candidates.append({'ordinal':len(candidates)+1,'prefix_relation':relation,'exact':exact,'images':imgs,'generation_sha256':digest((p.name+':'+start).encode()),'record_sha256':digest(cmd+b'\n'+maps)})
        except (FileNotFoundError,ProcessLookupError):continue
        except (PermissionError,ValueError,OSError):unknown+=1
    return {'candidates':candidates,'unavailable':unknown,'visited':visited}

def listeners(proc=pathlib.Path('/proc')):
    rows=[]
    for name in ['tcp','tcp6']:
        raw=(proc/'net'/name).read_bytes();require(len(raw)<=8*1024*1024,'socket_extent')
        for line in raw.decode().splitlines()[1:]:
            fields=line.split();require(len(fields)>=10,'socket_record')
            address,port=fields[1].split(':');port=int(port,16)
            if port in (5146,5563) and fields[3]=='0A':
                rows.append({'port':port,'loopback':address in ('0100007F','00000000000000000000000001000000'),'inode_sha256':digest(fields[9].encode()),'authority':'passive_listener_only_not_readiness'})
    return rows
