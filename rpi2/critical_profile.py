"""Small perf attachment helper for the existing supervised Pigments experiment."""
from pathlib import Path
import json
import os
import re
import resource
import signal
import struct
import subprocess
import time

CONTROL=Path('/run/lvb-rpi2-profile-guard-01')


def finish_guard():
    fifo=CONTROL/'request'
    if fifo.exists():
        fd=os.open(fifo,os.O_WRONLY|os.O_NONBLOCK)
        try:os.write(fd,b'finish\n')
        finally:os.close(fd)
    until=time.monotonic()+8
    while time.monotonic()<until:
        state=json.loads((CONTROL/'status.json').read_text())
        if state['phase']=='restored':return state
        if state['phase']=='restore_failed':raise RuntimeError('root guard restoration failed')
        time.sleep(.1)
    raise RuntimeError('root guard did not acknowledge restoration')


class Profile:
    def __init__(self,root,pid,session):
        self.stage=root/'staging/profile-01'
        self.pid=int(pid)
        self.status=root/'pigments-arm/compatdata/pfx/drive_c/bridge/sessions'/session/'ap12.status'
        self.session=bytes.fromhex(session)
        self.perf=str(self.stage/'perf-root/usr/bin/perf')
        self.env=dict(os.environ,LD_LIBRARY_PATH=str(self.stage/'perf-root/usr/lib/aarch64-linux-gnu'),DEBUGINFOD_URLS='')
        self.process=None

    def progress(self):
        began=time.monotonic_ns()
        for _ in range(3):
            a=self.status.read_bytes();b=self.status.read_bytes()
            if len(a)!=1024 or a[:4]!=b'LVFS' or struct.unpack_from('<I',a,4)[0]!=2 or a[16:32]!=self.session:
                raise ValueError('AP12 status identity/schema differs')
            rows=[]
            for lane in range(3):
                base=64+lane*320;c=struct.unpack_from('<Q',a,base)[0]
                if c!=struct.unpack_from('<Q',b,base)[0]:break
                offset=base+64+(c&1)*128
                if a[offset:offset+128]!=b[offset:offset+128]:break
                row=struct.unpack_from('<16Q',a,offset)
                rows.append(dict(counter=c,generation=row[0],epoch=row[1],sequence=row[2],position=row[3],stage=row[4],detail=row[5],timestamp=row[6],frequency=row[7],tid=row[8],pid=row[9],presentation=list(row[10:16]) if lane==0 else None))
            if len(rows)==3:return dict(begin_ns=began,end_ns=time.monotonic_ns(),stable=True,lanes=rows)
        return dict(begin_ns=began,end_ns=time.monotonic_ns(),stable=False)

    def start(self,run,name):
        self.run,self.name=run,name
        self.before=resource.getrusage(resource.RUSAGE_CHILDREN)
        self.record_begin_ns=time.monotonic_ns()
        self.process=subprocess.Popen([self.perf,'record','--no-buildid','-e','cycles:u','-F','99','--clockid','mono','-p',str(self.pid),'-o',str(run/(name+'.perf.data'))],env=self.env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)

    def stop(self):
        if self.process is None:return None
        sampler=self.process;self.process=None
        perf_cpu=None
        try:
            text=Path('/proc',str(sampler.pid),'stat').read_text();fields=text[text.rfind(')')+2:].split()
            perf_cpu=(int(fields[11])+int(fields[12]))/os.sysconf('SC_CLK_TCK')
        except FileNotFoundError:pass
        if sampler.poll() is None:sampler.send_signal(signal.SIGINT)
        try:stdout,stderr=sampler.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            sampler.kill();stdout,stderr=sampler.communicate()
            raise RuntimeError('perf did not flush within10s; data retained incomplete')
        end=time.monotonic_ns();after=resource.getrusage(resource.RUSAGE_CHILDREN)
        maps=Path('/proc',str(self.pid),'maps').read_text()
        (self.run/(self.name+'.process.maps')).write_text(maps)
        progress=self.progress()
        if not progress['stable']:raise RuntimeError('Windows PID readback unstable')
        winpid=progress['lanes'][1]['pid']
        # Proven live ARM64EC integration emits Linux /tmp using Windows PID.
        file=Path('/tmp')/f'perf-{winpid}.map'
        if file.stat().st_size>64*1024*1024:raise RuntimeError('JIT map exceeds64MiB bound')
        data=file.read_bytes();complete=data[:data.rfind(b'\n')+1]
        (self.run/(self.name+'.guest.map')).write_bytes(complete)
        result=dict(record_begin_ns=self.record_begin_ns,record_end_ns=end,exit_code=sampler.returncode,
            stderr=stderr,stdout=stdout,linux_pid=self.pid,windows_pid=winpid,map_bytes=len(data),
            perf_cpu_seconds_before_stop=perf_cpu,
            complete_map_bytes=len(complete),observer_children_cpu_seconds=(after.ru_utime+after.ru_stime)-(self.before.ru_utime+self.before.ru_stime),
            cpu_measurement_limitation='Includes the existing stimulus child and other helper children as well as perf; not isolated profiler overhead.',
            valid_stop=sampler.returncode in (0,-signal.SIGINT))
        if not result['valid_stop']:raise RuntimeError('perf recording failed: '+stderr[:300])
        return result
