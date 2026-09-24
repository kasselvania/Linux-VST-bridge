#!/usr/bin/env python3
"""One bounded, human-input-only capture on the current managed editor.

This composes the existing UIO1/2/3 scalar observers. It never sends input,
captures pixels, selects a new product, or changes an installed runner.
"""
import argparse
import dataclasses
import json
import os
from pathlib import Path
import signal
import sys
import time

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent / p) for p in ('uio1', 'uio2', 'uio3')]
from launch import Context, private_json
from capture import selected_window
from observe import GuiWitness, Observer, SIZE as OBSERVER_SIZE
from popup import Editor, WindowGraph, SIZE as GRAPH_SIZE
from x11 import X11
from xi2 import XI2, XIRecorder

SERUM_CLASS = '56534558667350736572756D20320000'
SERUM_C_PROFILE = '09af58ff7f66a2e75fcf608571f8805777c4ceb2537ebe47d96c7914b74107d1'
SECONDS = 23


def capture(admission, package, output, retained_editor=None):
    os.umask(0o077)
    c = Context(admission, SERUM_CLASS, package,
                manifest_path=HERE.parent / 'uio3/package.json',
                managed_observation=True)
    if c.admission['profile_fingerprint'] != SERUM_C_PROFILE or c.admission['maximum_seconds'] < SECONDS:
        raise RuntimeError('exact candidate-C diagnostic admission required')
    output.mkdir(mode=0o700, exist_ok=False)
    report = dict(schema=1,claim='candidate-C action-bound touch routing',
                  admission=c.admission, window_graphs=[], win32=[], xinput_raw=[],
                  xinput_delivered=[], gui=[], faults=[], clocks=[], drops={},
                  complete=False, input_injected=False)
    helper = observer = graph = gui = x = xi = record = None
    errors = []
    try:
        process, rows, snapshot = c.census(output / 'census.log')
        w, xid = selected_window(rows)
        for key in ('DISPLAY', 'XAUTHORITY'):
            if key in c.env: os.environ[key] = c.env[key]
        gui = GuiWitness(Path(c.session['directory']) / 'ap11.ui', c.session['session'])
        gui.cursors = [max(0, gui.read(at)-512) for at in (64, 80)]
        current = [r for r in gui.take() if r['lane'] == 1 and r['kind'] == 108]
        if current and current[-1]['target'] == xid:
            r = current[-1]
            editor = Editor(c.session['session'], r['native_view'], r['activation'],
                            snapshot['native']['generation'], r['editor_epoch'],
                            process['pid'], process['start'], w['tid'], w['hwnd'], xid)
        elif retained_editor:
            prior = json.loads(retained_editor.read_text())['editor']
            editor = Editor(**prior)
            if (editor.session != c.session['session'] or editor.generation != snapshot['native']['generation']
                    or (editor.pid, editor.start, editor.tid, editor.hwnd, editor.xid)
                    != (process['pid'], process['start'], w['tid'], w['hwnd'], xid)
                    or not snapshot['editor']['open'] or snapshot['editor']['failure']):
                raise RuntimeError('retained editor generation no longer exact')
        else:
            raise RuntimeError('native editor generation unavailable')
        editor.validate()
        report['editor'] = dataclasses.asdict(editor)
        report['before'] = snapshot
        x = X11(xid)
        pid = x.property(xid, '_NET_WM_PID')
        if len(pid) != 1: raise RuntimeError('editor X11 process identity unavailable')
        x.pid = pid[0]
        x.check_identity()
        path = output / 'observer.status'
        helper = c.helper('uio1-observer.exe',
                          ['observe-input', editor.hwnd, editor.start, 28,
                           c.runtime.windows(path, c.prefix)],
                          output / 'observer.log', 30)
        until = time.monotonic()+7
        while (not path.exists() or not Path(str(path)+'.windows').exists()
               or path.stat().st_size != OBSERVER_SIZE
               or Path(str(path)+'.windows').stat().st_size != GRAPH_SIZE):
            helper.poll()
            if time.monotonic() >= until: raise RuntimeError('observer startup timeout')
            time.sleep(.01)
        observer = Observer(path, editor.pid, editor.start, editor.hwnd)
        while not observer.status()['ready']:
            helper.poll()
            if time.monotonic() >= until: raise RuntimeError('observer handshake timeout')
            time.sleep(.01)
        if observer.read(232) != 1: raise RuntimeError('UIO3 input mode missing')
        graph = WindowGraph(Path(str(path)+'.windows'), editor)
        first = graph.snapshot()
        report['window_graphs'].append(first)
        targets = {row['xid'] for row in first['windows'] if row['pid'] == editor.pid and row['tid'] == editor.tid and row['xid']}
        targets.add(xid)
        xi = XI2(x, targets)
        record = XIRecorder(x, xi.opcode, xi.devices, targets)
        observer.action(1)
        xi.action = record.action = 1
        private_json(output / 'ready.json', dict(schema=1,ready=True,maximum_seconds=SECONDS,
                                                   human_input_only=True,session=c.session['session']))
        deadline = time.monotonic()+SECONDS
        next_graph = next_fault = next_clock = 0.0
        while time.monotonic() < deadline:
            helper.poll()
            xi.poll()
            record.devices = set(xi.devices)
            record.poll()
            report['win32'].extend(observer.take())
            report['gui'].extend(gui.take())
            now = time.monotonic()
            if now >= next_graph:
                current_graph = graph.snapshot()
                report['window_graphs'].append(current_graph)
                fresh = {row['xid'] for row in current_graph['windows'] if row['pid'] == editor.pid and row['tid'] == editor.tid and row['xid']}
                record.targets.update(fresh)
                xi.scope.targets.update(fresh)
                next_graph = now+.12
            if now >= next_fault:
                report['faults'].append(c.fault.snapshot())
                next_fault = now+.25
            if now >= next_clock:
                report['clocks'].append(observer.clock())
                next_clock = now+2
            status = observer.status()
            if any(status[key] for key in ('dropped', 'scope_errors', 'heartbeat_errors')):
                raise RuntimeError('observer evidence incomplete')
            time.sleep(.005)
        report['complete'] = True
    except Exception as exc:
        report['stop'] = type(exc).__name__ + ': ' + str(exc)
        errors.append(type(exc).__name__)
    finally:
        if record:
            try: record.close()
            except Exception as exc: errors.append(type(exc).__name__)
            report['xinput_delivered'] = record.records
            report['drops']['xrecord'] = record.dropped
            report['drops']['xrecord_unparsed'] = record.unparsed
        if xi:
            report['xinput_raw'] = xi.records
            report['drops']['xi_raw'] = xi.dropped
            report['xi_counters'] = xi.counters
            try: xi.close()
            except Exception as exc: errors.append(type(exc).__name__)
        if observer:
            try: observer.action(0); observer.stop()
            except Exception as exc: errors.append(type(exc).__name__)
        if helper:
            try:
                result = helper.finish()
                report['helper'] = result
                if result['exit'] != 0 or result['overflow'] or result['cleanup'].get('owned_descendants_zero') is not True or result['cleanup'].get('process_group_empty') is not True:
                    errors.append('helper_cleanup')
            except Exception as exc: errors.append(type(exc).__name__)
        if observer:
            try:
                report['win32'].extend(observer.take())
                report['observer_status'] = observer.status()
                report['drops']['win32'] = report['observer_status']['dropped']
                if report['observer_status']['detached'] != 1 or report['observer_status']['closed'] != 1:
                    errors.append('observer_detachment')
            except Exception as exc: errors.append(type(exc).__name__)
        if gui: report['drops']['gui_ring'] = sum(gui.dropped)
        for item in (graph, observer, gui, x):
            if item:
                try: item.close()
                except Exception as exc: errors.append(type(exc).__name__)
        try: report['after'] = c.fault.snapshot()
        except Exception as exc: errors.append(type(exc).__name__)
        c.fault.close()
        report['errors'] = errors
        if errors: report['complete'] = False
        private_json(output / 'timeline.private.json', report)
    return 0 if report['complete'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('admission', type=Path)
    parser.add_argument('package', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--editor-from', type=Path)
    args = parser.parse_args()
    raise SystemExit(capture(args.admission, args.package, args.output, args.editor_from))
