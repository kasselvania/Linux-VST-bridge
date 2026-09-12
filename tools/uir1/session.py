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
              finished=168, error=176, active_chains=184)


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
    report = {'schema': 1, 'display': {k: env.get(k) for k in ('DISPLAY', 'WAYLAND_DISPLAY', 'XDG_RUNTIME_DIR')},
              'clock_brackets': [], 'win32': [], 'xrecord': [], 'actions': [], 'snapshots': []}
    handles = []; status = observer = x = record = None
    def helper(name, args, seconds, label, first=False):
        h = Helper(ownership, [*base, 'run' if first else 'runinprefix',
                   str(target / name) if first else 'C:\\uir1\\' + name, *args],
                   env, target, root / (label + '.log'), seconds)
        handles.append(h); return h
    def snap(): return {k: status.read(v) for k, v in FIELDS.items()}
    def tick():
        for h in handles:
            if not h.closed: h.poll()
        if record: record.poll()
        if observer: report['win32'].extend(observer.take())
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
        fixture = helper('uir1-input-fixture.exe', ['C:\\uir1\\fixture.status'], 150, 'fixture', first=True)
        wait(lambda: (target / 'fixture.status').exists() and (target / 'fixture.status').stat().st_size == 4096, 100)
        status = Mapping(target / 'fixture.status', 4096)
        wait(lambda: status.read(56) == 1, 5)
        if status.map[:4] != b'UIR1' or status.read(8) != 1: raise RuntimeError('fixture status schema')
        identity = snap(); report['identity'] = identity
        census = helper('uio1-observer.exe', ['census', str(identity['pid'])], 10, 'census')
        wait(lambda: census.process.poll() is not None, 10)
        report['census_cleanup'] = census.finish()
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
        report['observer_cleanup'] = obshelper.finish()
        status.write(64, 3); wait(lambda: status.read(168) == 1, 3)
        report['final_status'] = snap()
        report['fixture_cleanup'] = fixture.finish()
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
        for h in reversed(handles):
            if not h.closed:
                try: h.finish()
                except Exception as e: report.setdefault('cleanup_errors', []).append(type(e).__name__)
        private_json(root / 'result-private.json', report)
    return 0 if report['completed'] else 1

if __name__ == '__main__': sys.exit(run(Path(sys.argv[1]), Path(sys.argv[2])))
