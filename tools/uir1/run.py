"""One generated headless differential; run in its own bounded systemd unit.
Inputs are private exact runner and build receipts, never a vendor/prefix copy.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

from isolation import compositor_environment
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "uio1"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "bridge-manager/runtime"))
import ownership
from launch import Helper, sealed_bytes


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
    if Path(runner['entry_point']).stat().st_dev != a.root.stat().st_dev:
        raise RuntimeError('scratch and pinned runtime must share a filesystem; refuse implicit full runtime copy')
    for item in runner['files']:
        with open(item['path'], 'rb') as f:
            if hashlib.file_digest(f, 'sha256').hexdigest() != item['sha256']:
                raise RuntimeError('pinned runner differs')
    files = json.loads(Path(__file__).with_name('package.json').read_text())['files']
    if json.loads((a.package / 'manifest.json').read_text()) != files:
        raise RuntimeError('package differs from source-owned build receipt')
    if set(files) != {'uir1-input-fixture.exe', 'uio1-observer.exe', 'uio1-hook.dll'}:
        raise RuntimeError('generated helper file set')
    for name, sha in files.items():
        sealed_bytes(a.package / name, sha)
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
    result = Helper(ownership, command, env, a.root, a.root / "compositor.log", 190).finish()
    print(json.dumps({"compositor_exit": result["exit"], "overflow": result["overflow"], "cleanup": result["cleanup"]}))
    receipt = a.root / "result-private.json"
    completed = receipt.exists() and json.loads(receipt.read_text()).get("completed") is True
    clean = result["cleanup"].get("owned_descendants_zero") is True and result["cleanup"].get("process_group_empty") is True
    return 0 if completed and clean and result["exit"] == 0 and result["overflow"] == 0 else 1

if __name__ == '__main__':
    sys.exit(main())
