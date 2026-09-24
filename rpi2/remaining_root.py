#!/usr/bin/python3
"""Root-only, one-session kernel sampler and bounded JIT-code reader.

The coordinator installs this reviewed file, perf and its private libraries in
the root-owned CONTROL directory before invocation. No global setting changes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import select
import signal
import stat
import subprocess
import time


CONTROL=Path('/run/lvb-rpi2-remaining-profile-01')
WORK=CONTROL/'work'
TOOLROOT=Path('/var/tmp/lvb-rpi2-profile-tools-01')
PERF=TOOLROOT/'perf'
LIBS=TOOLROOT/'lib'
EXPECTED_PERF_SHA256='3e22b572c30d828ef595d007d8ed9d0b80c72aae8baaaf5a147104d70f8d71b5'
PAGE=4096
MAX_SEGMENTS=3
SEGMENT_BYTES=5*PAGE
MAX_RECORD_SECONDS=27
MAX_PERF_CPU_SECONDS=.5


def start_ticks(pid,tid=None):
    path=Path('/proc',str(pid),'task',str(tid),'stat') if tid is not None else Path('/proc',str(pid),'stat')
    raw=path.read_text()
    return int(raw[raw.rfind(')')+2:].split()[19])


def assert_identity(args):
    if start_ticks(args.pid)!=args.pid_start or start_ticks(args.pid,args.worker_tid)!=args.worker_start:
        raise RuntimeError('PID/TID start-time changed')
    if start_ticks(args.pid,args.caller_tid)!=args.caller_start:
        raise RuntimeError('caller start-time changed')
    group=Path('/proc',str(args.pid),'cgroup').read_text().splitlines()
    if not any(line.endswith(':'+args.cgroup) for line in group):
        raise RuntimeError('Pigments cgroup changed')
    for tid in (args.worker_tid,args.caller_tid):
        uid_line=next(line for line in Path('/proc',str(args.pid),'task',str(tid),'status').read_text().splitlines() if line.startswith('Uid:'))
        if int(uid_line.split()[1])!=args.operator_uid:
            raise RuntimeError('target thread owner changed')


def assert_staging():
    for path in (CONTROL,TOOLROOT,PERF,LIBS):
        info=path.lstat()
        if info.st_uid!=0 or info.st_mode&0o022 or stat.S_ISLNK(info.st_mode):
            raise RuntimeError('root staging ownership or mode invalid')
    if hashlib.sha256(PERF.read_bytes()).hexdigest()!=EXPECTED_PERF_SHA256:
        raise RuntimeError('staged perf binary hash mismatch')
    for path in LIBS.iterdir():
        info=path.lstat()
        if info.st_uid!=0 or info.st_mode&0o022 or stat.S_ISLNK(info.st_mode):
            raise RuntimeError('root library staging invalid')


def publish(state):
    temp=WORK/'status.tmp'
    with temp.open('w') as handle:
        json.dump(state,handle,indent=2)
        handle.write('\n')
    os.chown(temp,0,WORK.stat().st_gid)
    os.chmod(temp,0o640)
    os.replace(temp,WORK/'status.json')


def owned_file(path,data,uid):
    fd=os.open(path,os.O_CREAT|os.O_EXCL|os.O_WRONLY|os.O_NOFOLLOW,0o600)
    try:
        with os.fdopen(fd,'wb') as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.chown(path,uid,-1)
    except:
        path.unlink(missing_ok=True)
        raise


def read_segment(args,ip):
    assert_identity(args)
    match=None
    for line in Path('/proc',str(args.pid),'maps').read_text().splitlines():
        fields=line.split(maxsplit=5)
        begin,end=(int(x,16) for x in fields[0].split('-'))
        if begin<=ip<end:
            match=(begin,end,fields)
            break
    if match is None:
        raise RuntimeError('requested JIT IP is unmapped')
    begin,end,fields=match
    if fields[1]!='rwxp' or len(fields)!=5:
        raise RuntimeError('requested IP is not in anonymous executable JIT memory')
    page=ip&~(PAGE-1)
    offset=page-2*PAGE
    if offset<begin or offset+SEGMENT_BYTES>end:
        raise RuntimeError('bounded JIT read crosses mapping boundary')
    fd=os.open(f'/proc/{args.pid}/mem',os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW)
    try:data=os.pread(fd,SEGMENT_BYTES,offset)
    finally:os.close(fd)
    if len(data)!=SEGMENT_BYTES:
        raise RuntimeError('short JIT read')
    assert_identity(args)
    return offset,data


def perf_cpu(pid):
    try:
        raw=Path('/proc',str(pid),'stat').read_text()
    except FileNotFoundError:
        return None
    fields=raw[raw.rfind(')')+2:].split()
    return (int(fields[11])+int(fields[12]))/os.sysconf('SC_CLK_TCK')


def recv(fd,deadline):
    pending=b''
    while time.monotonic()<deadline:
        if not select.select([fd],[],[],.25)[0]:
            continue
        pending+=os.read(fd,256)
        if len(pending)>256:
            raise RuntimeError('command too long')
        if b'\n' in pending:
            line,rest=pending.split(b'\n',1)
            if rest:
                raise RuntimeError('multiple commands refused')
            return line.decode('ascii')
    raise RuntimeError('capture control timeout')


def run(args):
    if os.geteuid()!=0 or args.operator_uid<=0:
        raise RuntimeError('authenticated root and explicit operator UID required')
    assert_staging()
    assert_identity(args)
    operator_gid=pwd.getpwuid(args.operator_uid).pw_gid
    WORK.mkdir(mode=0o750)
    os.chown(WORK,0,operator_gid)
    os.chmod(WORK,0o750)
    fifo=WORK/'request'
    os.mkfifo(fifo,0o600)
    os.chown(fifo,args.operator_uid,-1)
    fd=os.open(fifo,os.O_RDWR|os.O_NONBLOCK|os.O_NOFOLLOW)
    sampler=None
    stage='ready'
    try:
        publish(dict(phase='ready',pid=args.pid,worker_tid=args.worker_tid,caller_tid=args.caller_tid))
        line=recv(fd,time.monotonic()+45)
        parts=line.split()
        if not 1<=len(parts)-1<=MAX_SEGMENTS or parts[0]!='pre':
            raise RuntimeError('expected bounded pre addresses')
        ips=[int(item,16) for item in parts[1:]]
        if len(set(ips))!=len(ips):
            raise RuntimeError('duplicate JIT address')
        segments=[]
        for index,ip in enumerate(ips):
            offset,data=read_segment(args,ip)
            owned_file(WORK/f'pre-{index}.bin',data,args.operator_uid)
            segments.append(dict(ip=ip,begin=offset,bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
        stage='pre_ready'
        publish(dict(phase=stage,segments=segments))
        if recv(fd,time.monotonic()+30)!='start':
            raise RuntimeError('expected start')
        assert_identity(args)
        stdout=(WORK/'perf.stdout').open('x')
        stderr=(WORK/'perf.stderr').open('x')
        env=dict(os.environ,LD_LIBRARY_PATH=str(LIBS),DEBUGINFOD_URLS='')
        sampler=subprocess.Popen([str(PERF),'record','--no-buildid','-e','cycles:k','-F','49',
            '--call-graph','fp','--clockid','mono','-t',str(args.worker_tid),'-o',str(WORK/'kernel.perf.data')],
            env=env,stdout=stdout,stderr=stderr)
        stdout.close();stderr.close()
        time.sleep(.2)
        if sampler.poll() is not None:
            raise RuntimeError('kernel perf exited before capture')
        stage='recording'
        publish(dict(phase=stage,perf_pid=sampler.pid,segments=segments))
        deadline=time.monotonic()+MAX_RECORD_SECONDS
        while True:
            assert_identity(args)
            cpu=perf_cpu(sampler.pid)
            if cpu is not None and cpu>MAX_PERF_CPU_SECONDS:
                raise RuntimeError('kernel profiler CPU stop')
            if sampler.poll() is not None:
                raise RuntimeError('kernel profiler exited during capture')
            if select.select([fd],[],[],.25)[0]:
                if recv(fd,time.monotonic()+1)!='finish':
                    raise RuntimeError('expected finish')
                break
            if time.monotonic()>deadline:
                raise RuntimeError('kernel profiler exceeded capture bound')
        cpu=perf_cpu(sampler.pid)
        sampler.send_signal(signal.SIGINT)
        code=sampler.wait(timeout=8)
        if code not in (0,-signal.SIGINT):
            raise RuntimeError('kernel perf failed to finalize')
        sampler=None
        perf_data=WORK/'kernel.perf.data'
        if not perf_data.exists() or perf_data.stat().st_size>32*1024*1024:
            raise RuntimeError('kernel perf data missing or oversized')
        os.chown(perf_data,args.operator_uid,-1)
        os.chmod(perf_data,0o600)
        for index,segment in enumerate(segments):
            offset,data=read_segment(args,segment['ip'])
            if offset!=segment['begin']:
                raise RuntimeError('JIT mapping moved')
            owned_file(WORK/f'post-{index}.bin',data,args.operator_uid)
            segment['post_sha256']=hashlib.sha256(data).hexdigest()
            segment['window_unchanged']=segment['post_sha256']==segment['sha256']
        stage='complete'
        publish(dict(phase=stage,segments=segments,perf_cpu_seconds_before_stop=cpu,
                     perf_bytes=perf_data.stat().st_size,perf_exit_code=code))
    except Exception as exc:
        publish(dict(phase='failed',stage=stage,error=str(exc)))
        raise
    finally:
        if sampler is not None and sampler.poll() is None:
            sampler.send_signal(signal.SIGINT)
            try:sampler.wait(timeout=8)
            except subprocess.TimeoutExpired:sampler.kill();sampler.wait()
        os.close(fd)
        fifo.unlink(missing_ok=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    for name in ('pid','pid-start','worker-tid','worker-start','caller-tid','caller-start','operator-uid'):
        parser.add_argument('--'+name,type=int,required=True)
    parser.add_argument('--cgroup',required=True)
    run(parser.parse_args())
