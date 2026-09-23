"""Bounded five-selection headless experiment using the installed note fixture.

Run alongside pigments_session.py. All captures/state stay in its private run
directory. The script never edits vendor presets or infers navigation from CC
delivery. No physical input or editor is opened by this observer.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time
from pigments_preset_identity import identify


def main():
    p = argparse.ArgumentParser()
    p.add_argument('run', type=Path)
    p.add_argument('starting_state', type=Path)
    p.add_argument('fixture', type=Path)
    args = p.parse_args()
    run, log = args.run, args.run / 'outer.log'
    report = dict(notes=[60, 64, 67, 71], velocity=96, hold_seconds=12,
                  capture_seconds=20, normalized_master=0.35, editor_opened=False,
                  selections=[], navigation=[], starting_state_restored=False)
    sequence = 0
    deadline = None

    def save_report():
        (run / 'sweep.json').write_text(json.dumps(report, indent=2) + '\n')

    def send(command):
        fd = os.open(run / 'commands.fifo', os.O_WRONLY | os.O_NONBLOCK)
        try:
            os.write(fd, (command + '\n').encode())
        finally:
            os.close(fd)

    def request(command, marker, seconds=12):
        offset = log.stat().st_size
        send(command)
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            lines = log.read_text()[offset:].splitlines()
            if any(marker in line for line in lines):
                return [line for line in lines if line.startswith('RPI1_')]
            if (run / 'run.json').exists():
                raise RuntimeError('Host session ended')
            time.sleep(.1)
        raise RuntimeError('Response timeout: ' + command.split()[0])

    def snapshot(label):
        nonlocal sequence
        sequence += 1
        path = run / f'{sequence:03d}-{label}.state'
        request('save ' + str(path), 'RPI1_STATE_SAVED')
        return identify(path.read_bytes())

    def status():
        result = {}
        for line in request('status', 'RPI1_STATUS'):
            if line.startswith(('RPI1_AUDIO ', 'RPI1_CALLBACK_LIVE ')):
                for k, v in re.findall(r'(\w+)=([^ ]+)', line):
                    try:
                        result[k] = float(v) if '.' in v else int(v)
                    except ValueError:
                        result[k] = v
        return result

    def safety():
        temperature = int(Path('/sys/class/thermal/thermal_zone0/temp').read_text()) / 1000
        flags = int(subprocess.check_output(['vcgencmd', 'get_throttled'], text=True).strip().split('=')[1], 16)
        hz = int(subprocess.check_output(['vcgencmd', 'measure_clock', 'arm'], text=True).strip().split('=')[1])
        if temperature >= 75 or flags & 15:
            raise RuntimeError('Thermal/power stop')
        return dict(temperature_c=temperature, throttled=flags, arm_clock_hz=hz)

    def cpu():
        return {l.split()[0]:list(map(int, l.split()[1:9])) for l in Path('/proc/stat').read_text().splitlines() if re.match(r'^cpu[0-9]+ ', l)}

    def advance(previous, initializing=False):
        row = dict(from_preset=previous, initializing=initializing, edges=[], before=status())
        report['navigation'].append(row)
        start = time.monotonic()
        # A fresh browser may start at a different list position after restore.
        # Establish and label that first selection; never call it adjacent.
        current = previous
        for attempt in range(2):
            changed = []
            for value in [1, 0]:
                request(f'parameter 2395 {value}', 'RPI1_PARAMETER_QUEUED')
                time.sleep(.4)
                selected = snapshot('next-edge')
                row['edges'].append(dict(attempt=attempt + 1, value=value, selected=selected))
                if selected != current:
                    changed.append(selected)
                current = selected
            if len(changed) > 1:
                raise RuntimeError('More than one observed selection in one pulse')
            if len(changed) == 1 and current != previous:
                break
        else:
            raise RuntimeError('Next did not change selected preset after two pulses')
        time.sleep(2)
        if snapshot('settled-next') != current:
            raise RuntimeError('Selection changed again during settling')
        row.update(selected=current, navigation_and_settle_seconds=time.monotonic() - start, after=status())
        save_report()
        return current

    try:
        end = time.monotonic() + 90
        while time.monotonic() < end:
            if log.exists() and 'RPI1_READY ' in log.read_text():
                break
            if (run / 'run.json').exists():
                raise RuntimeError('Host ended before readiness')
            time.sleep(.5)
        else:
            raise RuntimeError('Readiness timeout')
        deadline = time.monotonic() + 220
        report['original_identity'] = identify(args.starting_state.read_bytes())
        request('restore ' + str(args.starting_state), 'RPI1_STATE_RESTORED')
        selected = snapshot('restored')
        if selected != report['original_identity']:
            raise RuntimeError('Restored identity differs')
        selected = advance(selected, initializing=True)
        for index in range(5):
            if time.monotonic() + 32 > deadline:
                raise RuntimeError('Insufficient remaining session time for another full trial')
            if any(r['identity'] == selected for r in report['selections']):
                raise RuntimeError('Preset repeated; refusing duplicate trial')
            request('parameter 0 0.35', 'RPI1_PARAMETER_QUEUED')
            time.sleep(2)
            offset = log.stat().st_size
            request('parameter 0', 'RPI1_PARAMETER_READBACK id=0')
            values = re.findall(r'RPI1_PARAMETER_READBACK id=0 normalized=([0-9.]+)', log.read_text()[offset:])
            if not values or abs(float(values[-1]) - .35) > 1e-5:
                raise RuntimeError('Conservative master readback differs')
            if snapshot('trial-start') != selected:
                raise RuntimeError('Preset changed before notes')
            row = dict(index=index + 1, identity=selected, before=status(), samples=[])
            report['selections'].append(row)
            first = cpu()
            start = time.monotonic()
            label = f'preset-{index+1:02d}'
            print('PLAY ' + json.dumps(selected), flush=True)
            with (run / (label + '.log')).open('x') as out:
                process = subprocess.Popen([str(args.fixture), 'four-notes', '--capture', str(run / (label + '.f32le'))],
                    env=dict(os.environ, LVB_QUALIFICATION_CLIENT='lvb-arm-pigments'), stdout=out, stderr=subprocess.STDOUT)
                try:
                    while process.poll() is None:
                        sample = safety()
                        sample['seconds'] = time.monotonic() - start
                        row['samples'].append(sample)
                        if sample['seconds'] > 30:
                            raise RuntimeError('Note fixture timeout')
                        time.sleep(1)
                    row['fixture_exit'] = process.returncode
                finally:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=5)
            last = cpu()
            row['elapsed_seconds'] = time.monotonic() - start
            row['cpu_core_busy_percent'] = {}
            for core, before in first.items():
                delta = [a-b for a,b in zip(last[core], before)]
                row['cpu_core_busy_percent'][core] = round(100 * (sum(delta)-delta[3]-delta[4])/sum(delta), 2)
            row['after'] = status()
            row['fixture_output'] = (run / (label + '.log')).read_text()
            if snapshot('trial-end') != selected:
                raise RuntimeError('Preset changed during note trial')
            save_report()
            print('DONE ' + json.dumps(dict(identity=selected, before=row['before'], after=row['after'])), flush=True)
            if row['fixture_exit'] != 0 or row['after'].get('fault') or row['after'].get('process_failures', 0) > row['before'].get('process_failures', 0):
                raise RuntimeError('Audio fixture or processing failure; stopping sweep')
            if index < 4:
                selected = advance(selected)
        report['completed_five'] = True
    except Exception as e:
        report['error'] = str(e)
        print('SWEEP_STOP ' + str(e), flush=True)
    finally:
        try:
            request('restore ' + str(args.starting_state), 'RPI1_STATE_RESTORED')
            report['starting_state_restored'] = snapshot('final-restored') == report.get('original_identity')
        except Exception as e:
            report['restore_error'] = str(e)
        try:
            send('quit')
        except OSError:
            pass
        save_report()


if __name__ == '__main__':
    main()
