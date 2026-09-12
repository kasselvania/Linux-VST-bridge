"""Private headless display boundary for generated UIR1 instrumentation only.

The caller runs this beneath a dedicated systemd cgroup with a hard deadline.
No operator DISPLAY, session bus, runtime directory, HOME or authorized prefix is
passed to KWin or the scratch runner. Nothing here admits a product executable.
"""
import os
from pathlib import Path


def private_directory(path):
    path = Path(path)
    path.mkdir(mode=0o700)
    if path.resolve() != path or path.stat().st_uid != os.getuid():
        raise RuntimeError('private directory identity')
    return path


def compositor_environment(root):
    root = Path(root)
    result = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8',
              'QT_QPA_PLATFORM': 'offscreen'}
    for key, name in [('HOME', 'home'), ('XDG_RUNTIME_DIR', 'runtime'),
                      ('XDG_CONFIG_HOME', 'config'), ('XDG_CACHE_HOME', 'cache'),
                      ('XDG_DATA_HOME', 'data')]:
        result[key] = str(private_directory(root / name))
    return result


def runner_environment(root, display_env):
    root = Path(root)
    runtime = root / 'runtime'
    # KWin must have supplied a NEW display/socket inside our private runtime.
    if display_env.get('XDG_RUNTIME_DIR') != str(runtime):
        raise RuntimeError('foreign runtime directory')
    if display_env.get('WAYLAND_DISPLAY') != 'uir1-isolated':
        raise RuntimeError('foreign compositor socket')
    if not display_env.get('DISPLAY') or not display_env.get('DBUS_SESSION_BUS_ADDRESS'):
        raise RuntimeError('isolated Xwayland/bus unavailable')
    env = {k: display_env[k] for k in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS',
           'XDG_RUNTIME_DIR', 'HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME', 'XDG_DATA_HOME')}
    if display_env.get('XAUTHORITY'):
        if not Path(display_env['XAUTHORITY']).resolve().is_relative_to(root):
            raise RuntimeError('foreign X authority')
        env['XAUTHORITY'] = display_env['XAUTHORITY']
    env.update(PATH='/usr/bin:/bin', LANG='C.UTF-8', STEAM_COMPAT_APP_ID='0',
               SteamAppId='0', SteamGameId='0', STEAM_ZENITY='')
    for key, name in [('STEAM_COMPAT_DATA_PATH', 'compatdata'),
                      ('STEAM_COMPAT_CLIENT_INSTALL_PATH', 'client'),
                      ('PRESSURE_VESSEL_VARIABLE_DIR', 'runtime-var'), ('TMPDIR', 'tmp')]:
        env[key] = str(private_directory(root / name))
    return env
