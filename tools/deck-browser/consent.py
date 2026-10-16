#!/usr/bin/env python3
"""Inspect and operate an ordinary KDE sharing dialog over existing SSH access."""
import argparse
from pathlib import Path
import shlex
import subprocess
import tempfile

REMOTE = r'''
import fcntl,json,os,struct,subprocess,sys,tempfile,time
from pathlib import Path
settings=json.load(sys.stdin)
env=dict(os.environ)
env.update(line.split('=',1) for line in subprocess.check_output(
    ['systemctl','--user','show-environment'],text=True).splitlines()
    if line.startswith(('DISPLAY=','WAYLAND_DISPLAY=','XDG_RUNTIME_DIR=','DBUS_SESSION_BUS_ADDRESS=')))
if settings['action']=='snapshot':
    with tempfile.TemporaryDirectory(prefix='lpb-deck-consent-',dir=env['XDG_RUNTIME_DIR']) as folder:
        env['XDG_CONFIG_HOME']=folder
        env['XDG_DATA_HOME']=folder
        env['XDG_STATE_HOME']=folder
        image=Path(folder)/'desktop.png'
        result=subprocess.run(['spectacle','--background','--nonotify','--fullscreen','--output',str(image)],
            env=env,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,timeout=20)
        if result.returncode or not image.exists():
            raise RuntimeError('Existing Spectacle could not capture the desktop')
        sys.stdout.buffer.write(image.read_bytes())
else:
    # Existing uinput ACL is required. Never modify device permissions or run as root.
    with open('/dev/uinput','wb',buffering=0) as device:
        for event in (0,1): fcntl.ioctl(device,0x40045564,event)
        if settings['action']=='key':
            fcntl.ioctl(device,0x40045565,settings['key'])
        else:
            fcntl.ioctl(device,0x40045565,0x110)
            fcntl.ioctl(device,0x40045564,3)
            for axis,maximum in ((0,settings['width']-1),(1,settings['height']-1)):
                fcntl.ioctl(device,0x40045567,axis)
                fcntl.ioctl(device,0x401c5504,struct.pack('H2x6i',axis,0,0,maximum,0,0,0))
        fcntl.ioctl(device,0x405c5503,struct.pack('HHHH80sI',3,0x1d6b,0x0104,1,b'LPB temporary UI input',0))
        fcntl.ioctl(device,0x5501)
        try:
            time.sleep(.4)
            def event(kind,code,value):device.write(struct.pack('llHHi',0,0,kind,code,value))
            def sync():event(0,0,0)
            if settings['action']=='click':
                event(3,0,settings['x']);event(3,1,settings['y']);sync()
                time.sleep(.1)
            key=settings['key'] if settings['action']=='key' else 0x110
            event(1,key,1);sync();time.sleep(.05);event(1,key,0);sync();time.sleep(.1)
        finally:
            fcntl.ioctl(device,0x5502)
'''

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('ssh_target')
actions = parser.add_subparsers(dest='action', required=True)
actions.add_parser('snapshot')
key = actions.add_parser('key')
key.add_argument('key', type=int, help='Linux input key code, for example 28 for Enter')
click = actions.add_parser('click')
for name in ('x', 'y', 'width', 'height'):
    click.add_argument(name, type=int)
options = parser.parse_args()
if options.ssh_target.startswith('-') or any(c.isspace() for c in options.ssh_target):
    parser.error('Use one SSH destination without command-line options')
settings = vars(options)
if options.action == 'key' and not 1 <= options.key <= 255:
    parser.error('Use a key code from 1 through 255')
if options.action == 'click' and not (0 <= options.x < options.width <= 16384 and 0 <= options.y < options.height <= 16384):
    parser.error('Click must be inside the captured desktop dimensions')
import json
result = subprocess.run(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', options.ssh_target,
    'python3 -c ' + shlex.quote(REMOTE)], input=json.dumps(settings).encode(), capture_output=True, timeout=35)
if result.returncode:
    raise SystemExit('Deck UI operation failed: ' + result.stderr.decode(errors='replace').strip())
if options.action == 'snapshot':
    with tempfile.NamedTemporaryFile(prefix='lpb-deck-consent-',suffix='.png',delete=False) as out:
        out.write(result.stdout)
    print(out.name)
