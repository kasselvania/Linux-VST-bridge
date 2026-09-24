"""Bounded read-only observer for the pinned FEX ARM64EC shared-stat v2 layout.

Source basis: FEX-Emu/FEX 82510eb452b258959ef982be58a9c3c1bafc82a4,
FEXCore/include/FEXCore/Utils/SHMStats.h and Source/Common/SHMStats.{h,cpp}.
Original decoder, not copied allocator code. File PID is Linux; slot TID is Windows.
"""
import os
from pathlib import Path
import stat
import struct
import time

HEADER=struct.Struct('<BBH48sIII')
SLOT=struct.Struct('<II13Q')
MAX_BYTES=4*1024*1024
COUNTERS=('jit_path_ticks','signal_ticks','sigbus_count','smc_count','float_fallback_count',
          'cache_miss_count','cache_read_lock_ticks','cache_write_lock_ticks','jit_count',
          'disk_cache_hit_count','disk_cache_miss_count','disk_cache_lookup_ticks')


def decode(blob):
    if not HEADER.size<=len(blob)<=MAX_BYTES:raise ValueError('FEX buffer extent')
    version,app,slot_size,build,head,size,_=HEADER.unpack_from(blob)
    if (version,app,slot_size)!=(2,2,112):raise ValueError('Unsupported FEX stats schema/application')
    if not HEADER.size<=size<=len(blob):raise ValueError('FEX advertised size exceeds captured bytes')
    rows=[];seen=set();offset=head;links=[]
    while offset:
        if offset in seen or offset<64 or (offset-64)%112 or offset+112>size:
            raise ValueError('Invalid FEX slot linkage')
        seen.add(offset)
        following,tid,*values=SLOT.unpack_from(blob,offset)
        links.append((offset,following,tid))
        if tid:rows.append({'slot_offset':offset,'windows_tid':tid,'counters':dict(zip(COUNTERS,values[:12]))})
        offset=following
    if len({r['windows_tid'] for r in rows})!=len(rows):raise ValueError('Duplicate live Windows TID')
    return {'version':version,'app_type':app,'slot_size':slot_size,'size':size,'head':head,
            'fex_build':build.split(b'\0',1)[0].decode('ascii'), 'rows':rows,'links':links}


def structural_identity(parsed):
    return (parsed['version'],parsed['app_type'],parsed['slot_size'],parsed['size'],parsed['head'],parsed['fex_build'],parsed['links'])


def interval(before,after):
    """Matched live slots only. Missing, recycled or reset counters are never zero-filled."""
    old={(r['slot_offset'],r['windows_tid']):r['counters'] for r in before['rows']}
    new={(r['slot_offset'],r['windows_tid']):r['counters'] for r in after['rows']}
    common=old.keys()&new.keys();totals={k:0 for k in COUNTERS};resets=[]
    for identity in sorted(common):
        changes={k:new[identity][k]-old[identity][k] for k in COUNTERS}
        if any(v<0 for v in changes.values()):resets.append(identity);continue
        for k,v in changes.items():totals[k]+=v
    return {'matched_slot_deltas':totals,'matched_slots':len(common)-len(resets),
            'added_slots':sorted(new.keys()-old.keys()),'removed_slots':sorted(old.keys()-new.keys()),
            'reset_slots':resets,'observed_identity_stable':old.keys()==new.keys() and not resets,
            'limitation':'No generation counter: rapid exit/reuse of same Windows TID and slot between samples may escape detection.'}


def process_start(pid):
    text=Path('/proc',str(pid),'stat').read_text()
    return int(text[text.rfind(')')+2:].split()[19])


class Reader:
    def __init__(self,pid):
        self.pid=pid;self.start=process_start(pid)
        proc=Path('/proc',str(pid));status=proc.joinpath('status').read_text().splitlines()
        ids=next((list(map(int,s.split()[1:])) for s in status if s.startswith('NSpid:')),[pid])
        # Only names belonging to this process in its known Linux PID namespaces.
        self.namespace_pids=list(dict.fromkeys([ids[-1],pid]));self.path=None;self.attempts=[]
        self.fd=None
        uid=proc.stat().st_uid
        for namespace in [Path('/dev/shm'),proc/'root/dev/shm']:
            for identifier in self.namespace_pids:
                path=namespace/f'fex-{identifier}-stats'
                try:
                    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
                    metadata=os.fstat(fd)
                    if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid!=uid or not 64<=metadata.st_size<=MAX_BYTES:
                        os.close(fd);raise ValueError('Owned FEX file identity/extent invalid')
                    self.fd=fd;self.path=str(path);self.inode=(metadata.st_dev,metadata.st_ino)
                    break
                except (FileNotFoundError,PermissionError) as exc:
                    self.attempts.append({'path':str(path),'error':type(exc).__name__})
            if self.fd is not None:break
        if self.fd is None:raise FileNotFoundError('No readable FEX stats for owned Linux PID in host or owned mount namespace')

    @classmethod
    def for_cohort(cls,cgroup,module_path):
        expected=module_path.stat();matches=[]
        for file in cgroup.rglob('cgroup.procs'):
            for pid in file.read_text().split():
                try:lines=Path('/proc',pid,'maps').read_text().splitlines()
                except (FileNotFoundError,PermissionError,ProcessLookupError):continue
                for line in lines:
                    fields=line.split(None,5)
                    if len(fields)==6 and fields[5].endswith('/Pigments.vst3'):
                        major,minor=(int(x,16) for x in fields[3].split(':'))
                        if int(fields[4])==expected.st_ino and os.makedev(major,minor)==expected.st_dev:
                            matches.append(int(pid));break
        matches=sorted(set(matches))
        if len(matches)!=1:raise ValueError('Owned exact Pigments module mapping ambiguous or absent')
        return cls(matches[0])

    def snapshot(self):
        begin=time.monotonic_ns();cpu=time.process_time_ns()
        try:
            if process_start(self.pid)!=self.start:raise ValueError('Owned process lifetime changed')
            metadata=os.fstat(self.fd)
            if (metadata.st_dev,metadata.st_ino)!=self.inode or not 64<=metadata.st_size<=MAX_BYTES:
                raise ValueError('FEX file identity/extent changed')
            first=decode(os.pread(self.fd,metadata.st_size,0))
            second=decode(os.pread(self.fd,metadata.st_size,0))
            if structural_identity(first)!=structural_identity(second):
                raise ValueError('FEX slot topology changed during read')
            checks=interval(first,second)
            if checks['reset_slots']:raise ValueError('FEX counters reset during read')
            return {'begin_ns':begin,'end_ns':time.monotonic_ns(),'observer_cpu_ns':time.process_time_ns()-cpu,
                    'linux_pid':self.pid,'linux_start_ticks':self.start,'namespace_pids':self.namespace_pids,
                    'path':self.path,'stats':second,'not_atomic_across_counters':True}
        except Exception:
            raise

    def close(self):
        if self.fd is not None:os.close(self.fd);self.fd=None
