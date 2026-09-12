"""One generated headless differential; run in its own bounded systemd unit.
Inputs are private exact runner and build receipts, never a vendor/prefix copy.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from isolation import compositor_environment


def main():
    p = argparse.ArgumentParser()
    p.add_argument('root', type=Path)
    p.add_argument('admission', type=Path)
    p.add_argument('package', type=Path)
    a = p.parse_args()
    os.umask(0o077)
    a.root.mkdir(mode=0o700)
    if a.root.resolve() != a.root:
        raise RuntimeError('root alias')
    runner = json.loads(a.admission.read_text())['registration']['environment']['runner']
    for item in runner['files']:
        with open(item['path'], 'rb') as f:
            if hashlib.file_digest(f, 'sha256').hexdigest() != item['sha256']:
                raise RuntimeError('pinned runner differs')
    files = json.loads((a.package / 'manifest.json').read_text())
    if set(files) != {'uir1-input-fixture.exe', 'uio1-observer.exe', 'uio1-hook.dll'}:
        raise RuntimeError('generated helper file set')
    for name, sha in files.items():
        with open(a.package / name, 'rb') as f:
            if hashlib.file_digest(f, 'sha256').hexdigest() != sha:
                raise RuntimeError('helper digest differs')
    (a.root / 'runner.json').write_text(json.dumps(runner))
    (a.root / 'package.json').write_text(json.dumps(files))
    env = compositor_environment(a.root)
    command = ['dbus-run-session', '--', 'kwin_wayland', '--virtual', '--xwayland',
               '--socket', 'uir1-isolated', '--width', '800', '--height', '600',
               '--no-lockscreen', '--no-global-shortcuts', '--no-kactivities',
               '--exit-with-session', str(Path(__file__).with_name('session.sh'))]
    env['UIR1_ROOT'] = str(a.root)
    env['UIR1_PACKAGE'] = str(a.package)
    # Parent systemd cgroup retains KWin, Xwayland, runner and all descendants.
    # No --replace, physical backend, user session bus or operator display.
    return subprocess.call(command, env=env)

if __name__ == '__main__':
    sys.exit(main())
