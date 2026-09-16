#!/usr/bin/env python3
"""Private developer fixture launcher; no commercial module or managed environment.

Stage contains exact copied runtime helpers, a source-owned Windows fixture and
fixture.json with the verified pinned runner. Called only by the retained LC1
native test inside an owned, bounded systemd unit.
"""
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time


def main():
    stage = pathlib.Path(__file__).resolve().parent
    config = json.loads((stage / 'fixture.json').read_text())
    sys.path.insert(0, str(stage))
    import session
    for name in ('session.py', 'ownership.py', 'ap18-lc1-session.exe'):
        assert hashlib.sha256((stage / name).read_bytes()).hexdigest() == config['artifacts'][name]
    runner = config['runner']
    for artifact in runner['files']:
        session.verify(artifact)
    root = stage / 'fixture-environment'
    for name in ('compatdata', 'client', 'runtime-var', 'host-cache', 'host-config', 'host-data', 'host-tmp', 'home'):
        (root / name).mkdir(parents=True, exist_ok=True, mode=0o700)
    env = session.environment({'environment': {'root': str(root)}, 'compatibility': {'disable_windows_accessibility': False}})
    env['HOME'] = str(root / 'home')
    env['WINEDEBUG'] = '-all'
    base = [runner['entry_point'], '--verb=run', '--', runner['proton']]
    # Pinned public initialization route; no Steam client/account startup.
    if not (root / 'compatdata/pfx/system.reg').is_file():
        result = subprocess.run(base + ['getcompatpath', '/'], env=env,
                                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL, timeout=120)
        assert result.returncode == 0 and (root / 'compatdata/pfx/system.reg').is_file()
    if sys.argv[1:] == ['--initialize']:
        return
    directory = pathlib.Path(sys.argv[1]).resolve(strict=True)
    sid = sys.argv[2]
    assert directory.parent == stage and directory.name in ('normal', 'activation_only', 'wrong_sequence')
    assert sid == '1c' * 16
    deadline = time.monotonic() + 10
    while not (directory / 'ap1.control').is_file():
        assert time.monotonic() < deadline
        time.sleep(.01)
    prefix = root / 'compatdata/pfx'
    args = base + ['runinprefix', session.windows(stage / 'ap18-lc1-session.exe', prefix),
                   session.windows(directory, prefix), sid]
    os.execve(args[0], args, env)


if __name__ == '__main__':
    main()
