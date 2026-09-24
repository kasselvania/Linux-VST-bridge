#!/usr/bin/python3
"""One foreground, sudo-authenticated RPI2 governor experiment; no commands run.

The benchmark remains unprivileged. Its only input is three literal lines sent
to a transient FIFO: performance, ondemand, finish, in that order. SIGKILL,
power loss and kernel failure cannot be handled by an in-process guard.
"""
import json
import os
from pathlib import Path
import select
import signal
import stat
import time

GOVERNOR = Path('/sys/devices/system/cpu/cpufreq/policy0/scaling_governor')
SCHEDSTATS = Path('/proc/sys/kernel/sched_schedstats')
CONTROL = Path('/run/lvb-rpi2-governor-guard')
TIMEOUT_SECONDS = 900
TRANSITIONS = ('performance', 'ondemand', 'finish')


def read_setting(path):
    return path.read_text().strip()


def write_setting(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        data = (value + '\n').encode('ascii')
        if os.write(fd, data) != len(data):
            raise RuntimeError('Short setting write')
    finally:
        os.close(fd)
    if read_setting(path) != value:
        raise RuntimeError('Setting readback mismatch')


class Settings:
    def __init__(self):
        self.original = {GOVERNOR: read_setting(GOVERNOR),
                         SCHEDSTATS: read_setting(SCHEDSTATS)}
        if self.original[GOVERNOR] != 'ondemand':
            raise RuntimeError('This fixed experiment requires original ondemand')
        if self.original[SCHEDSTATS] not in ('0', '1'):
            raise RuntimeError('Unexpected scheduler-statistics setting')
        self.changed = []

    def set(self, path, value):
        # Register restoration before writing, including a write/readback failure.
        if path not in self.changed:
            self.changed.append(path)
        write_setting(path, value)

    def restore(self):
        errors = []
        for path in reversed(self.changed):
            try:
                write_setting(path, self.original[path])
            except Exception as exc:
                errors.append(path.name + ': ' + str(exc))
        return errors


def transition(index, command, settings):
    if index >= len(TRANSITIONS) or command != TRANSITIONS[index]:
        raise ValueError('Unexpected phase command; stopping and restoring')
    if command != 'finish':
        settings.set(GOVERNOR, command)
    return index + 1


def exit_status(state):
    if state.get('phase') != 'restored' or state.get('restoration_errors'):
        return 1
    if state.get('stop_reason') != 'finished' or state.get('transition_count') != 3:
        return 2
    return 0


def publish(state):
    # CONTROL was created exclusively by this root process and is not writable
    # by the operator; neither status filename can be replaced by the benchmark.
    temporary = CONTROL / 'status.tmp'
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
    with os.fdopen(fd, 'w') as output:
        json.dump(state, output, indent=2)
        output.write('\n')
    os.replace(temporary, CONTROL / 'status.json')


def main():
    if os.geteuid() != 0:
        raise SystemExit('Run this fixed-purpose guard through sudo')
    uid = int(os.environ.get('SUDO_UID', '-1'))
    if uid <= 0:
        raise SystemExit('A non-root sudo operator is required')
    os.umask(0o022)
    settings = Settings()  # Read originals before any mutation or resource creation.
    CONTROL.mkdir(mode=0o755)  # Refuse an existing directory, including a symlink.
    info = CONTROL.stat()
    if info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o755:
        raise RuntimeError('Unexpected control-directory ownership/mode')
    fifo = CONTROL / 'request'
    descriptor = None
    stopped = None

    def stop(signum, _frame):
        nonlocal stopped
        stopped = 'signal_' + str(signum)

    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, stop)
    signal.alarm(TIMEOUT_SECONDS)
    deadline = time.monotonic() + TIMEOUT_SECONDS
    state = {'phase': 'starting', 'original_governor': settings.original[GOVERNOR],
             'original_schedstats': settings.original[SCHEDSTATS],
             'timeout_seconds': TIMEOUT_SECONDS, 'transition_count': 0}
    try:
        os.mkfifo(fifo, 0o600)
        os.chown(fifo, uid, -1)
        descriptor = os.open(fifo, os.O_RDWR | os.O_NONBLOCK | os.O_NOFOLLOW)
        settings.set(GOVERNOR, 'ondemand')
        try:
            settings.set(SCHEDSTATS, '1')
        except OSError as exc:
            state['schedstats_enable_error'] = str(exc)
        state.update(phase='ondemand_before', governor=read_setting(GOVERNOR),
                     schedstats=read_setting(SCHEDSTATS))
        publish(state)
        print(json.dumps(state), flush=True)
        pending = b''
        index = 0
        while not stopped and index < 3:
            if time.monotonic() >= deadline:
                stopped = 'hard_timeout'
                break
            if not select.select([descriptor], [], [], 0.25)[0]:
                continue
            pending += os.read(descriptor, 64)
            if len(pending) > 32:
                raise ValueError('Oversized phase request')
            if b'\n' not in pending:
                continue
            # One complete phase request at a time; no batching or arguments.
            if not pending.endswith(b'\n') or pending.count(b'\n') != 1:
                raise ValueError('Malformed phase request')
            command = pending[:-1].decode('ascii')
            pending = b''
            index = transition(index, command, settings)
            state.update(phase=('performance', 'ondemand_after', 'finishing')[index-1],
                         transition_count=index, governor=read_setting(GOVERNOR))
            publish(state)
            print(json.dumps(state), flush=True)
        state['stop_reason'] = stopped or 'finished'
    except Exception as exc:
        state['stop_reason'] = 'error: ' + str(exc)
    finally:
        signal.alarm(0)
        errors = settings.restore()
        if descriptor is not None:
            os.close(descriptor)
        fifo.unlink(missing_ok=True)
        state.update(phase='restored' if not errors else 'restore_failed',
                     restoration_errors=errors, governor=read_setting(GOVERNOR),
                     schedstats=read_setting(SCHEDSTATS))
        state['exit_code'] = exit_status(state)
        publish(state)
        print(json.dumps(state), flush=True)
    raise SystemExit(state['exit_code'])


if __name__ == '__main__':
    main()
