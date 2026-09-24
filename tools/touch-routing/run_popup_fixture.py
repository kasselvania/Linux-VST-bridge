#!/usr/bin/env python3
"""Run the source-owned popup fixture with physical input, never injected input."""
import argparse
import json
import os
from pathlib import Path
import subprocess

from seal_candidate import RUNNER_ID, sha, tree_identity, write_json


def run(build_root, manifest_path, fixture, attempt):
    os.umask(0o077)
    manifest = json.loads(manifest_path.read_text())
    if manifest.get('kind') != 'x11_touch_routing_reference_runner' \
            or manifest['runner']['id'] != RUNNER_ID \
            or Path(manifest['root']) != build_root / ('candidate-' + RUNNER_ID) \
            or tree_identity(Path(manifest['root'])) != manifest['tree']:
        raise RuntimeError('fixture_runner_identity')
    if not fixture.is_file() or fixture != build_root / 'popup_fixture.exe':
        raise RuntimeError('fixture_executable_identity')
    if attempt not in (1, 2, 3, 4):
        raise RuntimeError('fixture_attempt_out_of_bounds')
    root = build_root / f'popup-fixture-{attempt:03d}.private'
    if root.exists() or root.is_symlink():
        raise RuntimeError('fixture_root_exists')
    for name in ('', 'compatdata', 'home', 'client', 'cache', 'config', 'data', 'tmp', 'runtime-var'):
        (root / name).mkdir(mode=0o700)
    user = os.environ['USER']
    env = {'HOME': str(root / 'home'), 'USER': user, 'LOGNAME': user,
           'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8',
           'XDG_RUNTIME_DIR': f'/run/user/{os.getuid()}',
           'XDG_CACHE_HOME': str(root / 'cache'), 'XDG_CONFIG_HOME': str(root / 'config'),
           'XDG_DATA_HOME': str(root / 'data'), 'TMPDIR': str(root / 'tmp'),
           'STEAM_COMPAT_APP_ID': '0', 'SteamAppId': '0', 'SteamGameId': '0',
           'STEAM_COMPAT_CLIENT_INSTALL_PATH': str(root / 'client'),
           'STEAM_COMPAT_DATA_PATH': str(root / 'compatdata'),
           'PRESSURE_VESSEL_VARIABLE_DIR': str(root / 'runtime-var'),
           'STEAM_ZENITY': '', 'PROTON_LOG': '0'}
    session = subprocess.run(['systemctl', '--user', 'show-environment'], text=True,
                             capture_output=True, check=True, timeout=10)
    for line in session.stdout.splitlines():
        key, _, value = line.partition('=')
        if key in ('DISPLAY', 'XAUTHORITY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS'):
            env[key] = value
    if not env.get('DISPLAY') or not env.get('XAUTHORITY'):
        raise RuntimeError('fixture_graphical_session_absent')
    command = [manifest['runner']['entry_point'], '--verb=run', '--',
               manifest['runner']['proton']]
    prefix = root / 'compatdata/pfx'
    fixture_status = None
    try:
        init = subprocess.run([*command, 'getcompatpath', '/'], cwd=root, env=env,
                              capture_output=True, timeout=180, start_new_session=True)
        if init.returncode:
            raise RuntimeError('fixture_prefix_initialization_failed')
        print('POPUP_FIXTURE_VISIBLE: use physical mouse and touchscreen, then close its window', flush=True)
        executed = subprocess.run([*command, 'runinprefix', str(fixture)], cwd=root, env=env,
                                  capture_output=True, timeout=240, start_new_session=True)
        fixture_status = executed.returncode
    finally:
        if prefix.is_dir():
            wineserver = str(Path(manifest['root']) / 'files/bin/wineserver')
            cleanup_env = dict(env, WINEPREFIX=str(prefix))
            try:
                subprocess.run([wineserver, '-w'], env=cleanup_env, capture_output=True,
                               timeout=2, check=True)
            except subprocess.TimeoutExpired:
                subprocess.run([wineserver, '-k'], env=cleanup_env, capture_output=True,
                               timeout=15, check=True)
                subprocess.run([wineserver, '-w'], env=cleanup_env, capture_output=True,
                               timeout=15, check=True)
    log = root / 'popup-fixture.private.tsv'
    if not log.is_file():
        raise RuntimeError('fixture_log_absent')
    receipt = {'schema': 1, 'runner_tree': manifest['tree']['sha256'],
               'fixture_executable_sha256': sha(fixture), 'fixture_log_sha256': sha(log),
               'fixture_exit': fixture_status, 'physical_input_only': True,
               'cleanup_confirmed': True}
    write_json(root / 'fixture-result.private.json', receipt)
    print(json.dumps(receipt, sort_keys=True))
    return 0 if fixture_status == 0 else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--build-root', type=Path, required=True)
    parser.add_argument('--attempt', type=int, default=1)
    args = parser.parse_args()
    if not args.build_root.is_absolute():
        raise RuntimeError('fixture_build_root')
    raise SystemExit(run(args.build_root,
                         args.build_root / 'touch-candidate-manifest.private.json',
                         args.build_root / 'popup_fixture.exe', args.attempt))
