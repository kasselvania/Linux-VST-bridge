"""One bounded user/kernel perf attachment for the existing RPI2 session.

The separate reviewed root service accepts only pre/start/finish. This file
never escalates privilege or changes machine settings.
"""
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import struct
import subprocess
import time

from critical_profile import Profile


CONTROL=Path('/run/lvb-rpi2-remaining-profile-01/work')
FAMILY=(0x7354e0,0x735530,0x7356e0)
VENDOR_SHA256='73e1bfc24a2f6d807d6bc162ff5dba89df50a787cc8a3c0354666fff38816f66'
SEGMENT_BYTES=5*4096


def ticks(pid,tid):
    data=Path('/proc',str(pid),'task',str(tid),'stat').read_text()
    return int(data[data.rfind(')')+2:].split()[19])


def cpu_seconds(pid):
    data=Path('/proc',str(pid),'stat').read_text()
    fields=data[data.rfind(')')+2:].split()
    return (int(fields[11])+int(fields[12]))/os.sysconf('SC_CLK_TCK')


def command(value):
    fd=os.open(CONTROL/'request',os.O_WRONLY|os.O_NONBLOCK|os.O_NOFOLLOW)
    try:os.write(fd,(value+'\n').encode('ascii'))
    finally:os.close(fd)


def wait_phase(phase,timeout):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        try:state=json.loads((CONTROL/'status.json').read_text())
        except FileNotFoundError:state={}
        if state.get('phase')=='failed':raise RuntimeError('root collector failed: '+state.get('error','unknown'))
        if state.get('phase')==phase:return state
        time.sleep(.1)
    raise RuntimeError('root collector '+phase+' timeout')


def module_base(pid):
    rows=[]
    for line in Path('/proc',str(pid),'maps').read_text().splitlines():
        fields=line.split(maxsplit=5)
        if len(fields)==6 and fields[2]=='00000000' and fields[5].lower().endswith('/pigmentsprocessor.dll'):
            rows.append((int(fields[0].split('-')[0],16),Path(fields[5])))
    if len(rows)!=1:raise RuntimeError('vendor module base ambiguous')
    base,path=rows[0]
    if hashlib.sha256(path.read_bytes()).hexdigest()!=VENDOR_SHA256:
        raise RuntimeError('vendor module changed')
    return base


def map_candidates(data):
    clusters=defaultdict(lambda: defaultdict(set))
    pattern=re.compile(rb'^0x([0-9a-f]+) [0-9a-f]+ .*pigmentsprocessor\.dll\+0x(7354e0|735530|7356e0) ',re.I)
    for line in data.splitlines():
        match=pattern.match(line)
        if match:
            addr=int(match[1],16)
            size=int(line.split()[1],16)
            clusters[addr>>16][int(match[2],16)].add((addr,size))
    ordered=sorted(clusters.items(),key=lambda item:sum(map(len,item[1].values())),reverse=True)
    selected=[]
    for _,labels in ordered[:3]:
        preferred=labels.get(0x735530) or labels.get(0x7354e0) or labels.get(0x7356e0)
        selected.append(min(preferred))
    if not selected:raise RuntimeError('arithmetic family missing from live FEX map')
    return selected


def family_blocks(data,segment_begin,module):
    """Validate FEX block header/tail; sparse RIP entries are not expanded."""
    found=[]
    for offset in range((16-segment_begin%16)%16,len(data)-44,16):
        tail_delta=struct.unpack_from('<I',data,offset)[0]
        if not 64<=tail_delta<SEGMENT_BYTES-40-offset or tail_delta%8:continue
        tail=offset+tail_delta
        size,rip,guest_size,entries,entries_delta,spin,single=struct.unpack_from('<QQQIIIB',data,tail)
        begin=segment_begin+offset
        if not (64<=size<=SEGMENT_BYTES-offset and size%16==0):continue
        if rip-module not in FAMILY or not 0<guest_size<=0x2000:continue
        if not (entries_delta==40 and entries<4096 and spin in (0,1) and single in (0,1)):continue
        if tail+40+entries>offset+size:continue
        found.append(dict(begin=begin,end=begin+size,rip=rip,guest_size=guest_size,
                          tail=segment_begin+tail,entries=entries))
    return found


class RemainingProfile(Profile):
    def __init__(self,root,pid,session):
        super().__init__(root,pid,session)
        self.prepared=False
        self.active=False

    def prepare(self,warmup,run,cgroup):
        begin=warmup['samples'][0]['threads'];end=warmup['samples'][-1]['threads']
        candidates=[]
        for key,last in end.items():
            if not key.startswith(str(self.pid)+':') or key not in begin:continue
            elapsed=last['cpu_ns']-begin[key]['cpu_ns']
            candidates.append((last['name'],int(key.split(':')[1]),elapsed))
        callers=[(elapsed,tid) for name,tid,elapsed in candidates if name=='lvb-audio']
        workers=[(elapsed,tid) for name,tid,elapsed in candidates if name=='Processing Thre']
        if len(callers)!=1 or not workers:raise RuntimeError('caller/worker TID ownership ambiguous')
        self.caller_tid=callers[0][1]
        self.worker_tid=max(workers)[1]
        if max(workers)[0]<1_000_000_000:raise RuntimeError('processing worker did not run during warmup')
        self.module=module_base(self.pid)
        progress=self.progress()
        if not progress['stable']:raise RuntimeError('Windows PID unstable before capture')
        winpid=progress['lanes'][1]['pid']
        map_path=Path('/tmp')/f'perf-{winpid}.map'
        if map_path.stat().st_size>64*1024*1024:raise RuntimeError('FEX map too large')
        data=map_path.read_bytes();data=data[:data.rfind(b'\n')+1]
        (run/'pre.guest.map').write_bytes(data)
        addresses=map_candidates(data)
        group=next((line.split('::',1)[1] for line in Path('/proc',str(self.pid),'cgroup').read_text().splitlines() if line.startswith('0::')),None)
        if not group:raise RuntimeError('Pigments cgroup identity missing')
        identity=dict(pid=self.pid,pid_start=ticks(self.pid,self.pid),worker_tid=self.worker_tid,
                      worker_start=ticks(self.pid,self.worker_tid),caller_tid=self.caller_tid,
                      caller_start=ticks(self.pid,self.caller_tid),operator_uid=os.getuid(),cgroup=group)
        (run/'root-identity.json').write_text(json.dumps(identity,indent=2)+'\n')
        print('REMAINING_ROOT_READY '+json.dumps(identity),flush=True)
        ready=wait_phase('ready',90)
        if (ready.get('pid'),ready.get('worker_tid'),ready.get('caller_tid'))!=(self.pid,self.worker_tid,self.caller_tid):
            raise RuntimeError('root collector attached wrong process/thread')
        command('pre '+' '.join(format(ip,'x') for ip,_ in addresses))
        pre=wait_phase('pre_ready',15)
        validated=[]
        for index,segment in enumerate(pre['segments']):
            blob=(CONTROL/f'pre-{index}.bin').read_bytes()
            if len(blob)!=SEGMENT_BYTES or hashlib.sha256(blob).hexdigest()!=segment['sha256']:
                raise RuntimeError('JIT pre-read changed')
            blocks=family_blocks(blob,segment['begin'],self.module)
            matching=[block for block in blocks if block['begin']<=segment['ip']<block['end'] and block['end']-block['begin']==addresses[index][1]]
            if len(matching)!=1:raise RuntimeError('FEX block identity ambiguous at selected IP')
            validated.append(matching[0])
        self.validated=validated
        self.pre_segments=pre['segments']
        self.prepared=True
        return dict(identity=identity,addresses=addresses,blocks=validated,
                    limitation='Block extent and entry RIP only; sparse FEX debug entries do not map every guest arithmetic instruction.')

    def start(self,run,name):
        if name=='warmup':return
        if not self.prepared:raise RuntimeError('remaining profile not prepared')
        self.run,self.name=run,name
        command('start')
        wait_phase('recording',10)
        self.before=resource.getrusage(resource.RUSAGE_CHILDREN)
        self.record_begin_ns=time.monotonic_ns()
        self.process=subprocess.Popen([self.perf,'record','--no-buildid','-e','cycles:u','-F','49',
            '--clockid','mono','-t',str(self.caller_tid),'-o',str(run/(name+'.perf.data'))],
            env=self.env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        time.sleep(.2)
        if self.process.poll() is not None:raise RuntimeError('user perf exited before capture')
        self.active=True

    def progress(self):
        if self.active:
            if self.process.poll() is not None:raise RuntimeError('user perf exited during capture')
            if cpu_seconds(self.process.pid)>.5:raise RuntimeError('user profiler CPU stop')
            state=json.loads((CONTROL/'status.json').read_text())
            if state.get('phase')!='recording':raise RuntimeError('kernel profiler stopped during capture')
        return super().progress()

    def stop(self):
        if not self.active:return None
        self.active=False
        try:user=super().stop()
        finally:command('finish')
        final=wait_phase('complete',15)
        unchanged=[]
        for index,segment in enumerate(final['segments']):
            before=(CONTROL/f'pre-{index}.bin').read_bytes()
            after=(CONTROL/f'post-{index}.bin').read_bytes()
            block=self.validated[index]
            off=block['begin']-segment['begin'];end=block['end']-segment['begin']
            equal=before[off:end]==after[off:end]
            unchanged.append(equal)
        return dict(user=user,kernel=final,validated_blocks=self.validated,
                    block_bytes_unchanged=unchanged,
                    limitation='Pre/post unchanged bytes bracket sample window; no temporal JIT event log, and ARM cycle samples can skid.')
