#!/usr/bin/python3
"""Bounded root settings guard; experiment and profiler remain unprivileged.

Bootstrap installs this exact helper under root ownership and starts one systemd
service. RuntimeMaxSec and ExecStopPost restore settings independently of the
helper's signal/finally handling, including helper SIGKILL. Kernel/power failure
cannot be covered. The operator FIFO accepts only 'finish', never a command.
"""
import json
import os
from pathlib import Path
import select
import shutil
import signal
import stat
import subprocess
import sys
import time

CONTROL=Path('/run/lvb-rpi2-profile-guard-01')
GOVERNOR=Path('/sys/devices/system/cpu/cpufreq/policy0/scaling_governor')
SCHEDSTATS=Path('/proc/sys/kernel/sched_schedstats')
UNIT='lvb-rpi2-profile-guard-01.service'
LIMIT=300


def write_setting(path,value):
    fd=os.open(path,os.O_WRONLY|os.O_NOFOLLOW|os.O_CLOEXEC)
    try:
        data=(value+'\n').encode('ascii')
        if os.write(fd,data)!=len(data):raise RuntimeError('short setting write')
    finally:os.close(fd)
    if path.read_text().strip()!=value:raise RuntimeError('setting readback mismatch')


def state_file(name):
    p=CONTROL/name
    m=p.lstat()
    if not stat.S_ISREG(m.st_mode) or m.st_uid!=0 or m.st_mode&0o022:
        raise RuntimeError('guard record ownership')
    return p


def publish(state):
    p=CONTROL/('status.'+str(os.getpid())+'.tmp')
    fd=os.open(p,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o644)
    with os.fdopen(fd,'w') as f:json.dump(state,f,indent=2);f.write('\n')
    os.replace(p,CONTROL/'status.json')


def restore():
    original=json.loads(state_file('original.json').read_text())
    errors=[]
    for key,path in [('schedstats',SCHEDSTATS),('governor',GOVERNOR)]:
        try:write_setting(path,original[key])
        except Exception as exc:errors.append(path.name+': '+str(exc))
    state={'phase':'restore_failed' if errors else 'restored','restoration_errors':errors,
           'governor':GOVERNOR.read_text().strip(),'schedstats':SCHEDSTATS.read_text().strip()}
    publish(state)
    return bool(errors)


def bootstrap():
    uid=int(os.environ.get('SUDO_UID','-1'))
    if uid<=0:raise RuntimeError('non-root sudo operator required')
    original={'uid':uid,'governor':GOVERNOR.read_text().strip(),
              'schedstats':SCHEDSTATS.read_text().strip()}
    if original['governor']!='ondemand' or original['schedstats'] not in ('0','1'):
        raise RuntimeError('unexpected original settings')
    CONTROL.mkdir(mode=0o755) # Existing directory or symlink is a refusal.
    with (CONTROL/'original.json').open('x') as f:json.dump(original,f)
    (CONTROL/'original.json').chmod(0o600)
    installed=CONTROL/'guard.py'
    shutil.copyfile(Path(__file__),installed);installed.chmod(0o500)
    # This service manager, independent of the helper, owns expiry/restoration.
    subprocess.run(['systemd-run','--unit='+UNIT,'--collect',
        '--property=Type=exec','--property=RuntimeMaxSec='+str(LIMIT),
        '--property=TimeoutStopSec=10',
        '--property=ExecStopPost=/usr/bin/python3 '+str(installed)+' --restore',
        '/usr/bin/python3',str(installed),'--run'],check=True)


def run():
    original=json.loads(state_file('original.json').read_text())
    fifo=CONTROL/'request';fd=None;stopped=False
    def stop(_signum,_frame):
        nonlocal stopped
        stopped=True
    for sig in (signal.SIGTERM,signal.SIGINT,signal.SIGHUP,signal.SIGALRM):
        signal.signal(sig,stop)
    signal.alarm(LIMIT-15)
    try:
        os.mkfifo(fifo,0o600);os.chown(fifo,original['uid'],-1)
        fd=os.open(fifo,os.O_RDWR|os.O_NONBLOCK|os.O_NOFOLLOW)
        write_setting(GOVERNOR,'performance');write_setting(SCHEDSTATS,'1')
        publish({'phase':'ready','governor':GOVERNOR.read_text().strip(),
                 'schedstats':SCHEDSTATS.read_text().strip(),'limit_seconds':LIMIT,
                 'ready_monotonic_ns':time.monotonic_ns()})
        pending=b''
        while not stopped:
            if not select.select([fd],[],[],.25)[0]:continue
            pending+=os.read(fd,32)
            if len(pending)>7:raise RuntimeError('invalid finish request')
            if b'\n' not in pending:continue
            if pending!=b'finish\n':raise RuntimeError('invalid finish request')
            break
    finally:
        signal.alarm(0)
        if fd is not None:os.close(fd)
        fifo.unlink(missing_ok=True)
        if restore():raise RuntimeError('settings restoration failed')


if __name__=='__main__':
    if os.geteuid()!=0:raise SystemExit('root settings guard requires sudo authentication')
    os.umask(0o022)
    if sys.argv[1:]==['--bootstrap']:bootstrap()
    elif sys.argv[1:]==['--run']:run()
    elif sys.argv[1:]==['--restore']:raise SystemExit(int(restore()))
    else:raise SystemExit('expected --bootstrap, --run or --restore')
