#!/usr/bin/env python3
"""Seal the exact Wine X11 window-touch successor without mutating candidate C."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess

RUNNER_ID = 'proton-11.0-2c-x11-touch-routing-v2'
RUNNER_VERSION = '1788504981 proton-11.0-2c-x86_64+x11-touch-routing-v2; SLR 4.0.20260805.254769'
BASE_RUNNER = 'proton-11.0-2c-x11-touch-release-v1'
PROTON_SOURCE = '5b89db940e0ebe3a137a6009a3589232fe084c09'
WINE_SOURCE = 'dc26e61847081a1b5cb0733dc30feba6ee575482'
WINE_TREE = 'da4b1eb3b7f209eb4a971d4b5929fcda08ff6b4e'
PATCH_SHA = '44c3c11806fe48ed0358fd3469d60c5572c0f78773057ef4c335198c1c497203'
MOUSE_SHA = '8e79383dd969aeb7c48533dcfab945a42c27ac6072ff96a79d9091f7f5a7a2aa'
HEADER_SHA = 'fb91d7b0c1bb51b183aadb71e3505687c42827b1381dbc700a4b8fddefa6c57d'
SDK_SHA = '97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9'
BASE_DRIVER_SHA = 'c939ed62a6226a2e88827428281f2e934ebb5546fad101bd354a8a64164d00dc'
CHANGED = ('version', 'files/lib/wine/x86_64-unix/winex11.so')


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def artifact(path):
    return {'path': str(path), 'sha256': sha(path)}


def write_json(path, value):
    data = (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o400)
    with os.fdopen(fd, 'wb') as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())


def tree_identity(root):
    rows, total, pending = [], 0, [root]
    while pending:
        for path in pending.pop().iterdir():
            metadata = path.lstat()
            relative = path.relative_to(root).as_posix()
            mode = metadata.st_mode & 0o7777
            if stat.S_ISDIR(metadata.st_mode):
                pending.append(path)
                row = ['d', relative, mode]
            elif stat.S_ISREG(metadata.st_mode):
                total += metadata.st_size
                row = ['f', relative, mode, metadata.st_size, sha(path)]
            elif stat.S_ISLNK(metadata.st_mode):
                if not path.resolve(strict=True).is_relative_to(root):
                    raise RuntimeError('outside_link:' + relative)
                row = ['l', relative, mode, os.readlink(path)]
            else:
                raise RuntimeError('unsupported_type:' + relative)
            rows.append((relative, row))
    rows.sort(key=lambda item: item[0])
    h = hashlib.sha256()
    for _, row in rows:
        h.update(json.dumps(row, separators=(',', ':'), ensure_ascii=False).encode())
        h.update(b'\n')
    return {'schema': 1, 'entries': len(rows), 'regular_bytes': total, 'sha256': h.hexdigest()}


def source_check(args):
    if sha(args.patch) != PATCH_SHA or sha(args.source / 'dlls/winex11.drv/mouse.c') != MOUSE_SHA \
            or sha(args.source / 'dlls/winex11.drv/touch_message_flags.h') != HEADER_SHA:
        raise RuntimeError('patched_source_changed')
    for ref, expected in (('HEAD', WINE_SOURCE), ('HEAD^{tree}', WINE_TREE)):
        actual = subprocess.check_output(['git', '-C', str(args.source), 'rev-parse', ref], text=True).strip()
        if actual != expected:
            raise RuntimeError('wine_source_changed')


def compose(args, environment):
    source_check(args)
    if environment['id'] != '4db060b14388e41103834fc4dfdd023a' or environment['revision'] != 2:
        raise RuntimeError('candidate_c_environment_required')
    before = environment['runner']
    if before['id'] != BASE_RUNNER or before.get('policy') != 'x11_touch_release_v1' or len(before['files']) != 32:
        raise RuntimeError('candidate_c_runner_required')
    old_root = Path(before['proton']).parent
    old_driver = old_root / CHANGED[1]
    if not any(row['path'] == str(old_driver) and row['sha256'] == BASE_DRIVER_SHA for row in before['files']):
        raise RuntimeError('candidate_c_driver_changed')
    if args.root.exists() or args.root.is_symlink():
        raise RuntimeError('candidate_root_exists')
    shutil.copytree(old_root, args.root, symlinks=True)
    args.root.chmod(0o700)
    version = args.root / 'version'
    version.write_text('1788504981 proton-11.0-2c-x86_64+x11-touch-routing-v2\n')
    driver = args.root / CHANGED[1]
    built = args.stage / 'lib/wine/x86_64-unix/winex11.so'
    if not built.is_file() or not driver.is_file() or (args.stage / 'lib/wine/i386-unix/winex11.so').exists():
        raise RuntimeError('exact_wow64_driver_required')
    driver.chmod(driver.stat().st_mode | stat.S_IWUSR)
    shutil.copy2(built, driver)
    changed = {relative: artifact(args.root / relative) for relative in CHANGED}
    files = []
    for prior in before['files']:
        prior_path = Path(prior['path'])
        new_path = args.root / prior_path.relative_to(old_root) if prior_path.is_relative_to(old_root) else prior_path
        current = artifact(new_path)
        relative = new_path.relative_to(args.root).as_posix() if new_path.is_relative_to(args.root) else None
        if relative not in CHANGED and current['sha256'] != prior['sha256']:
            raise RuntimeError('unchanged_runner_artifact_differs:' + new_path.name)
        files.append(current)
    if len(files) != 32 or len({row['path'] for row in files}) != 32:
        raise RuntimeError('runner_roster')
    runner = {'id': RUNNER_ID, 'version': RUNNER_VERSION, 'proton': str(args.root / 'proton'),
              'entry_point': before['entry_point'], 'files': files}
    plan = {'schema': 1, 'tree': tree_identity(args.root), 'runner': runner,
            'changed_artifacts': changed,
            'base_runner_sha256': hashlib.sha256(json.dumps(before, sort_keys=True, separators=(',', ':')).encode()).hexdigest()}
    write_json(args.build_root / 'candidate-plan.private.json', plan)
    print(json.dumps({'candidate_tree': plan['tree']['sha256'], 'changed_artifacts': changed}, sort_keys=True))


def seal(args, environment):
    source_check(args)
    plan = json.loads((args.build_root / 'candidate-plan.private.json').read_text())
    if tree_identity(args.root) != plan['tree']:
        raise RuntimeError('candidate_tree_changed')
    test = args.build_root / 'test_touch_flags'
    if not test.is_file():
        raise RuntimeError('mapping_test_absent')
    subprocess.run([str(test)], check=True, timeout=10)
    smoke = args.build_root / 'unlicensed-smoke.private'
    if smoke.exists() or smoke.is_symlink():
        raise RuntimeError('unlicensed_smoke_exists')
    for name in ('', 'compatdata', 'home', 'client', 'cache', 'config', 'data', 'tmp', 'runtime-var'):
        (smoke / name).mkdir(mode=0o700)
    user = os.environ['USER']
    env = {'HOME': str(smoke / 'home'), 'USER': user, 'LOGNAME': user, 'PATH': '/usr/bin:/bin',
           'LANG': 'C.UTF-8', 'XDG_RUNTIME_DIR': f'/run/user/{os.getuid()}',
           'XDG_CACHE_HOME': str(smoke / 'cache'), 'XDG_CONFIG_HOME': str(smoke / 'config'),
           'XDG_DATA_HOME': str(smoke / 'data'), 'TMPDIR': str(smoke / 'tmp'),
           'STEAM_COMPAT_APP_ID': '0', 'SteamAppId': '0', 'SteamGameId': '0',
           'STEAM_COMPAT_CLIENT_INSTALL_PATH': str(smoke / 'client'),
           'STEAM_COMPAT_DATA_PATH': str(smoke / 'compatdata'),
           'PRESSURE_VESSEL_VARIABLE_DIR': str(smoke / 'runtime-var'), 'STEAM_ZENITY': '', 'PROTON_LOG': '0'}
    session = subprocess.run(['systemctl', '--user', 'show-environment'], text=True, capture_output=True, check=True, timeout=10)
    for line in session.stdout.splitlines():
        key, _, value = line.partition('=')
        if key in ('DISPLAY', 'XAUTHORITY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS'):
            env[key] = value
    command = [environment['runner']['entry_point'], '--verb=run', '--', str(args.root / 'proton')]
    prefix = smoke / 'compatdata/pfx'
    cleanup_env = dict(env, WINEPREFIX=str(prefix))
    try:
        initialized = subprocess.run([*command, 'getcompatpath', '/'], env=env, text=True,
                                     capture_output=True, timeout=180, start_new_session=True)
        if initialized.returncode:
            raise RuntimeError('unlicensed_runner_initialize_failed')
        completed = subprocess.run([*command, 'runinprefix', 'cmd.exe', '/d', '/c', 'ver'], env=env,
                                   text=True, capture_output=True, timeout=60, start_new_session=True)
    finally:
        if prefix.is_dir():
            wineserver = str(args.root / 'files/bin/wineserver')
            try:
                subprocess.run([wineserver, '-w'], env=cleanup_env, capture_output=True,
                               timeout=2, check=True)
            except subprocess.TimeoutExpired:
                subprocess.run([wineserver, '-k'], env=cleanup_env, capture_output=True,
                               timeout=15, check=True)
                subprocess.run([wineserver, '-w'], env=cleanup_env, capture_output=True,
                               timeout=15, check=True)
    if completed.returncode or 'Microsoft Windows' not in completed.stdout:
        raise RuntimeError('unlicensed_runner_smoke_failed')
    if not prefix.is_dir():
        raise RuntimeError('unlicensed_smoke_prefix_absent')
    if tree_identity(args.root) != plan['tree']:
        raise RuntimeError('candidate_tree_changed_during_smoke')
    receipt_path = args.build_root / 'touch-build-receipt.private.json'
    receipt = {'schema': 1, 'proton_distribution_source_commit': PROTON_SOURCE, 'wine_commit': WINE_SOURCE,
               'wine_tree': WINE_TREE, 'patch_sha256': PATCH_SHA, 'patched_mouse_sha256': MOUSE_SHA,
               'touch_header_sha256': HEADER_SHA, 'sdk_image_sha256': SDK_SHA,
               'configure': ['--enable-archs=i386,x86_64'], 'install_prefix': str(args.stage),
               'configure_log': artifact(args.build_root / 'configure.private.log'),
               'build_log': artifact(args.build_root / 'build.private.log'),
               'install_log': artifact(args.build_root / 'install.private.log'),
               'changed_artifacts': {path: value['sha256'] for path, value in plan['changed_artifacts'].items()},
               'candidate_tree_sha256': plan['tree']['sha256'], 'mapping_test_passed': True,
               'build_passed': True, 'unlicensed_smoke_passed': True,
               'unlicensed_smoke_initialize_exit': initialized.returncode,
               'unlicensed_smoke_cmd_exit': completed.returncode,
               'unlicensed_smoke_cleanup_confirmed': False,
               'wow64_no_i386_unix_driver': True}
    receipt['unlicensed_smoke_cleanup_confirmed'] = True
    write_json(receipt_path, receipt)
    manifest = {'schema': 1, 'kind': 'x11_touch_routing_reference_runner',
                'environment': environment['id'], 'base_runner_id': environment['runner']['id'],
                'base_runner_sha256': plan['base_runner_sha256'],
                'source': {'proton_distribution_source_commit': PROTON_SOURCE,
                           'wine_commit': WINE_SOURCE, 'wine_tree': WINE_TREE},
                'patch': artifact(args.patch), 'build_receipt': artifact(receipt_path),
                'root': str(args.root), 'tree': plan['tree'], 'runner': plan['runner'],
                'changed_artifacts': plan['changed_artifacts']}
    manifest_path = args.build_root / 'touch-candidate-manifest.private.json'
    write_json(manifest_path, manifest)
    print(json.dumps({'manifest_sha256': sha(manifest_path), 'candidate_tree': plan['tree']['sha256']}, sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=('compose', 'seal'))
    for name in ('environment-json', 'source', 'stage', 'patch', 'build-root', 'root'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    if any(not value.is_absolute() for value in (args.environment_json, args.source, args.stage, args.patch, args.build_root, args.root)) \
            or args.source != args.build_root / 'source' or args.stage != args.build_root / 'stage' \
            or args.patch != args.build_root / 'wine-x11-window-touch-v2.patch' \
            or args.root != args.build_root / ('candidate-' + RUNNER_ID):
        raise RuntimeError('private_build_paths')
    environment = json.loads(args.environment_json.read_text())
    (compose if args.mode == 'compose' else seal)(args, environment)
