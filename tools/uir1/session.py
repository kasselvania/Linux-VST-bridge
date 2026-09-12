"""KWin-launched private fixture session. No vendor or operator display access."""
import json
import os
from pathlib import Path
import sys
import time

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'uio1'))
sys.path.insert(0, str(HERE.parent.parent / 'bridge-manager/runtime'))
import ownership
from launch import Helper, private_json, sealed_bytes
from observe import Mapping, Observer
from x11 import X11
from xrecord import Recorder
from isolation import runner_environment

FIELDS = dict(pid=16, start=24, root=32, child=40, frequency=48, ready=56, command=64,
              phase=72, phase_qpc=80, submitted=88, posted=96, sent=104, send_failures=112,
              down=120, up=128, down_qpc=136, up_qpc=144, turns=152, max_turn_qpc=160,
              finished=168, error=176, active_chains=184,
              loaded_down_posts=192, loaded_up_posts=200, loaded_down_chains=208,
              loaded_up_chains=216, loaded_down_qpc=224, loaded_up_qpc=232,
              paints=240, timers=248, loaded_paints=256, loaded_timers=264,
              moves=272, quit_preserved=280, bound_preserved=288)


def finish_helper(report, key, helper):
    # finish() returns failed exit/overflow/containment as data. Retain that
    # data first, but never let it authorize a completed diagnostic session.
    result = helper.finish()
    report[key] = result
    cleanup = result.get('cleanup', {})
    if (type(result.get('exit')) is not int or result['exit'] != 0 or
        type(result.get('overflow')) is not int or result['overflow'] != 0 or
        not isinstance(cleanup, dict) or
        cleanup.get('owned_descendants_zero') is not True or
        cleanup.get('process_group_empty') is not True):
        raise RuntimeError(key + ': helper exit, output or cleanup incomplete')


def run(root, package):
    os.umask(0o077)
    env = runner_environment(root, os.environ)
    runner = json.loads((root / 'runner.json').read_text())
    files = json.loads((root / 'package.json').read_text())
    target = root / 'compatdata/pfx/drive_c/uir1'
    target.mkdir(parents=True, mode=0o700)
    for name, sha in files.items():
        with open(target / name, 'xb') as f: f.write(sealed_bytes(package / name, sha))
        os.chmod(target / name, 0o500)
    base = [runner['entry_point'], '--verb=run', '--', runner['proton']]
    report = {'schema': 2, 'display': {k: env.get(k) for k in ('DISPLAY', 'WAYLAND_DISPLAY', 'XDG_RUNTIME_DIR')},
              'clock_brackets': [], 'win32': [], 'xrecord': [], 'actions': [], 'snapshots': [],
              'traffic_samples': [], 'traffic_sample_overflow': 0}
    handles = []; status = observer = x = record = None
    def helper(name, args, seconds, label):
        h = Helper(ownership, [*base, 'runinprefix', 'C:\\uir1\\' + name, *args],
                   env, target, root / (label + '.log'), seconds)
        handles.append(h); return h
    def snap(): return {k: status.read(v) for k, v in FIELDS.items()}
    next_sample = 0
    def tick():
        nonlocal next_sample
        for h in handles:
            if not h.closed: h.poll()
        if record: record.poll()
        if observer: report['win32'].extend(observer.take())
        now = time.monotonic_ns()
        if status and now >= next_sample:
            next_sample = now + 50_000_000
            if len(report['traffic_samples']) < 1200:
                report['traffic_samples'].append(dict(observed_ns=now, **snap()))
            else: report['traffic_sample_overflow'] += 1
    def wait(test, seconds):
        end = time.monotonic() + seconds
        while not test():
            tick()
            if time.monotonic() > end: raise TimeoutError('bounded fixture transition')
            time.sleep(.01)
        tick()
    def interval(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end: tick(); time.sleep(.005)
    def click(action):
        observer.action(action); record.action = action
        report['actions'].append(dict(action=action, motion=x.move((.5, .5))))
        x.settle_pointer()
        report['actions'].append(dict(action=action, down=x.button(True)))
        interval(.08)
        report['actions'].append(dict(action=action, up=x.button(False)))
    try:
        # Pinned Proton source: getcompatpath performs init_session(True) and
        # setup_prefix, then winepath; unlike 'run' it does not launch steam.exe.
        init = Helper(ownership, [*base, 'getcompatpath', str(target)], env, target, root / 'initialize.log', 50)
        handles.append(init)
        finish_helper(report, 'initialization', init)
        fixture = helper('uir1-input-fixture.exe', ['C:\\uir1\\fixture.status'], 60, 'fixture')
        wait(lambda: (target / 'fixture.status').exists() and (target / 'fixture.status').stat().st_size == 4096, 100)
        status = Mapping(target / 'fixture.status', 4096)
        wait(lambda: status.read(56) == 1, 5)
        if status.map[:4] != b'UIR1' or status.read(8) != 2: raise RuntimeError('fixture status schema')
        identity = snap(); report['identity'] = identity
        census = helper('uio1-observer.exe', ['census', str(identity['pid'])], 10, 'census')
        wait(lambda: census.process.poll() is not None, 10)
        finish_helper(report, 'census_cleanup', census)
        rows = [json.loads(line) for line in (root / 'census.log').read_text().splitlines() if line.startswith('{')]
        report['census'] = rows
        matches = [r for r in rows if r.get('type') == 'x11_binding' and r['hwnd'] == identity['root']]
        if len(matches) != 1: raise RuntimeError('exact generated X11 binding absent')
        xid = matches[0]['xid']
        if not xid: raise RuntimeError('generated root has no X11 window')
        obshelper = helper('uio1-observer.exe', ['observe', str(identity['root']), str(identity['start']), '35', 'C:\\uir1\\observe.status'], 45, 'observer')
        wait(lambda: (target / 'observe.status').exists() and (target / 'observe.status').stat().st_size > 4096, 10)
        observer = Observer(target / 'observe.status', identity['pid'], identity['start'], identity['root'])
        wait(lambda: observer.status()['ready'] == 1, 5)
        report['clock_brackets'].append(observer.clock())
        x = X11(xid); x.activate(); wait(lambda: x.pointer()['active'] == [xid], 3)
        record = Recorder(x)
        interval(.4)
        click(1)
        wait(lambda: status.read(128) == 1, 3)
        interval(.4); report['snapshots'].append(dict(condition='idle', **snap()))
        report['clock_brackets'].append(observer.clock())
        status.write(64, 2)
        wait(lambda: status.read(72) == 2, 2)
        interval(.2)
        click(2)
        wait(lambda: status.read(128) == 2, 3)
        # Reverse fairness: bounded sustained server-level hardware motion while
        # the same four posted chains continue. No extra clicks or key input.
        sustained_before = snap()
        for i in range(400):
            x.move((.5 + (.025 if i % 2 else -.025), .5))
            tick(); time.sleep(.002)
        sustained_after = snap()
        report['sustained_input'] = dict(motions_issued=400, before=sustained_before, after=sustained_after)
        if (sustained_after['posted'] <= sustained_before['posted'] or
            sustained_after['moves'] <= sustained_before['moves'] or
            sustained_after['phase'] != 2):
            raise RuntimeError('posted/input reverse progress absent during finite traffic')
        wait(lambda: status.read(72) == 3, 12)
        wait(lambda: status.read(128) == 2, 3)
        interval(.4); report['snapshots'].append(dict(condition='traffic_complete', **snap()))
        report['clock_brackets'].append(observer.clock())
        click(3)
        wait(lambda: status.read(128) == 3, 3)
        interval(.4); report['snapshots'].append(dict(condition='post_traffic_idle', **snap()))
        observer.action(0); record.action = 0
        report['clock_brackets'].append(observer.clock())
        report['xrecord'] = record.records
        report['xrecord_status'] = dict(dropped=record.dropped, unparsed=record.unparsed)
        observer.stop(); wait(lambda: observer.status()['closed'] != 0, 3)
        report['observer_status'] = observer.status()
        finish_helper(report, 'observer_cleanup', obshelper)
        status.write(64, 3); wait(lambda: status.read(168) == 1, 3)
        report['final_status'] = snap()
        finish_helper(report, 'fixture_cleanup', fixture)
        report['completed'] = True
    except BaseException as error:
        report['completed'] = False; report['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        # Preserve partial bounded records; never repeat input after failure.
        if record:
            report['xrecord'] = record.records
            report['xrecord_status'] = dict(dropped=record.dropped, unparsed=record.unparsed)
            record.close()
        if x: x.close()
        if observer:
            observer.stop(); report['win32'].extend(observer.take())
            report['observer_status_final'] = observer.status(); observer.close()
        if status:
            status.write(64, 3); report['last_status'] = snap(); status.close()
        for i, h in enumerate(reversed(handles)):
            if not h.closed:
                try: finish_helper(report, 'remaining_helper_' + str(i), h)
                except Exception as e:
                    report['completed'] = False
                    report.setdefault('cleanup_errors', []).append(type(e).__name__ + ': ' + str(e))
        private_json(root / 'result-private.json', report)
    return 0 if report['completed'] else 1

if __name__ == '__main__': sys.exit(run(Path(sys.argv[1]), Path(sys.argv[2])))
