#!/usr/bin/python3
"""Original/buffered recorder comparison; same ondemand, no privileged guard."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import threading
import time
from pigments_preset_identity import identify

CONTROL = Path('/run/lvb-rpi2-governor-guard')


def guard(expected):
    governor = Path('/sys/devices/system/cpu/cpufreq/policy0/scaling_governor').read_text().strip()
    sched = Path('/proc/sys/kernel/sched_schedstats').read_text().strip()
    if governor != 'ondemand' or sched != '0':
        raise RuntimeError('Expected unchanged ondemand and disabled scheduler statistics')
    return dict(phase=expected, governor=governor, schedstats=sched)


def status_values(lines):
    out = {}
    for line in lines:
        if line.startswith(('RPI1_AUDIO ', 'RPI1_CALLBACK_LIVE ')):
            for k, v in re.findall(r'(\w+)=([^ ]+)', line):
                try: out[k] = float(v) if '.' in v else int(v)
                except ValueError: out[k] = v
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--attempt', default='01')
    parser.add_argument('--original', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9]{2}', args.attempt):
        raise ValueError('Attempt must be two digits')
    root, output = args.root, args.output
    output.mkdir(mode=0o700)
    env = root / 'pigments-arm'
    original = env / 'logs/preset-buttons-01/physical-navigation-final.state'
    selected = env / 'logs/preset-sweep-01/005-trial-start.state'
    report = {'conditions': [], 'original_identity': identify(original.read_bytes()),
              'selected_identity': identify(selected.read_bytes()), 'errors': []}
    report['clock_ticks_per_second'] = os.sysconf('SC_CLK_TCK')
    expected = None
    session = None
    fifo = None
    log = None

    def send(command):
        fd = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
        try: os.write(fd, (command + '\n').encode())
        finally: os.close(fd)

    def request(command, marker):
        offset = log.stat().st_size
        send(command)
        end = time.monotonic() + 12
        while time.monotonic() < end:
            lines = log.read_text()[offset:].splitlines()
            if any(marker in line for line in lines):
                return [line for line in lines if line.startswith('RPI1_')]
            if session.poll() is not None: raise RuntimeError('Session ended')
            time.sleep(.1)
        raise RuntimeError('Response timeout: ' + command.split()[0])

    def safety():
        state = guard(expected)
        temp = int(Path('/sys/class/thermal/thermal_zone0/temp').read_text()) / 1000
        flags = int(subprocess.check_output(['vcgencmd', 'get_throttled'], text=True).split('=')[1], 16)
        if temp >= 75 or flags & 15: raise RuntimeError('Thermal/power stop')
        return state, temp, flags

    def snapshot(cgroup):
        state, temp, flags = safety()
        result = {'monotonic_ns': time.monotonic_ns(), 'temperature_c': temp,
                  'throttled': flags, 'schedstats_enabled': Path('/proc/sys/kernel/sched_schedstats').read_text().strip() == '1',
                  'arm_clock_hz': int(subprocess.check_output(['vcgencmd', 'measure_clock', 'arm'], text=True).split('=')[1]),
                  'governor': state['governor'], 'threads': {},
                  'phase_file_bytes': phase.stat().st_size,
                  'cpu': [int(x) for x in Path('/proc/stat').read_text().splitlines()[0].split()[1:9]],
                  'cohort_cpu_stat': dict(line.split() for line in (cgroup/'cpu.stat').read_text().splitlines())}
        result['native_io'] = dict(line.split(': ',1) for line in Path('/proc',native_pid,'io').read_text().splitlines())
        result['native_threads'] = {}
        for task in Path('/proc',native_pid,'task').glob('*'):
            try:
                raw=(task/'stat').read_text(); fields=raw[raw.rfind(')')+2:].split()
                result['native_threads'][task.name]=dict(name=raw[raw.find('(')+1:raw.rfind(')')],ticks=int(fields[11])+int(fields[12]),cpu_ns=int((task/'schedstat').read_text().split()[0]))
            except (FileNotFoundError,ProcessLookupError): pass
        for procfile in cgroup.rglob('cgroup.procs'):
            for pid in procfile.read_text().split():
                for task in Path('/proc', pid, 'task').glob('*'):
                    try:
                        raw = (task/'stat').read_text(); fields = raw[raw.rfind(')')+2:].split()
                        sched = list(map(int, (task/'schedstat').read_text().split()))
                        result['threads'][pid+':'+task.name] = dict(name=raw[raw.find('(')+1:raw.rfind(')')],
                            ticks=int(fields[11])+int(fields[12]), cpu_ns=sched[0],
                            wait_ns=sched[1] if result['schedstats_enabled'] else None)
                    except (FileNotFoundError, ProcessLookupError): pass
        return result

    def trial(name, run, cgroup):
        row = {'name': name, 'samples': [], 'before': status_values(request('status','RPI1_STATUS '))}
        row['flush_marks_ns'] = []
        def flush_mark():
            request('mark', 'RPI1_OBSERVER_MARKER accepted')
            row['flush_marks_ns'].append(time.monotonic_ns())
        flush_mark()
        fixture = root/'staging/four-notes-20260923/qualification-four-notes'
        lines = []
        process = subprocess.Popen([str(fixture), 'four-notes', '--capture', str(run/(name+'.f32le'))],
            env=dict(os.environ, LVB_QUALIFICATION_CLIENT='lvb-arm-pigments'), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        def read_output():
            for line in process.stdout:
                if line.startswith('RPI1_QUALIFICATION_GRAPH '): row['graph_observed_ns'] = time.monotonic_ns()
                lines.append(line)
        reader = threading.Thread(target=read_output); reader.start()
        started = time.monotonic()
        next_flush = started + 5
        try:
            while process.poll() is None:
                row['samples'].append(snapshot(cgroup))
                if time.monotonic() >= next_flush:
                    flush_mark()
                    next_flush += 5
                if time.monotonic()-started > 35: raise RuntimeError('Stimulus timeout')
                time.sleep(.5)
        finally:
            if process.poll() is None: process.terminate(); process.wait(timeout=5)
            reader.join(timeout=3)
        row['fixture_exit'] = process.returncode
        flush_mark()
        row['fixture_output'] = ''.join(lines)
        results = [line for line in lines if line.startswith('RPI1_QUALIFICATION_RESULT ')]
        fields = dict(re.findall(r'(\w+)=([^ ]+)', results[0].strip())) if len(results)==1 else {}
        row['capture_observation'] = dict(
            bytes=(run/(name+'.f32le')).stat().st_size,
            expected_bytes=20*48000*2*4,
            fixture_completed=fields.get('completed') == 'true',
            fixture_stereo_nonzero=fields.get('stereo_nonzero') == 'true',
            fixture_result=fields,
            limitation='Nonzero samples and process exit do not establish continuous audio; analyze held-window recording separately.')
        row['after'] = status_values(request('status', 'RPI1_STATUS '))
        if 'graph_observed_ns' not in row: raise RuntimeError('Stimulus alignment absent')
        return row

    try:
        for index, expected in enumerate(('original', 'buffered')):
            state, _, _ = safety()
            row = {'condition': expected, 'guard_before': state, 'trials': []}
            report['conditions'].append(row)
            label = 'obs-' + expected.replace('_','-') + '-' + args.attempt
            run = env/'logs'/label
            fifo, log = run/'commands.fifo', run/'outer.log'
            before = set((env/'evidence').glob('*-phase.jsonl'))
            with (output/(label+'-session.log')).open('x') as sink:
                session = subprocess.Popen(['flock', '--nonblock', str(root.parent/'rpi1-private/operation.lock'),
                    '/usr/bin/python3', str(env/'plugin_session.py'), label, '--config', str(env/'preset-navigation.conf'),
                    '--binary', str(args.original if index==0 else args.candidate)], stdout=sink, stderr=subprocess.STDOUT)
                end = time.monotonic()+100
                try:
                    while not log.exists() or 'RPI1_READY ' not in log.read_text():
                        safety()
                        if time.monotonic()>end or session.poll() is not None: raise RuntimeError('Startup failed')
                        time.sleep(.5)
                    phases = set((env/'evidence').glob('*-phase.jsonl'))-before
                    if len(phases)!=1: raise RuntimeError('Phase ownership ambiguous')
                    phase = phases.pop(); row['phase_path'] = str(phase); row['run_path'] = str(run)
                    units = subprocess.check_output(['systemctl','--user','list-units','--state=running','--plain','--no-legend','lvb-rpi1-*.service'], text=True).splitlines()
                    if len(units)!=1: raise RuntimeError('Windows cohort ownership ambiguous')
                    group = subprocess.check_output(['systemctl','--user','show',units[0].split()[0],'-p','ControlGroup','--value'],text=True).strip()
                    cgroup = Path('/sys/fs/cgroup')/group.lstrip('/')
                    native_pid=subprocess.check_output(['systemctl','--user','show','lvb-rpi2-'+label+'.service','-p','MainPID','--value'],text=True).strip()
                    row['restore'] = request('restore '+str(selected),'RPI1_STATE_RESTORED ')
                    time.sleep(4)
                    row['master'] = request('parameter 0','RPI1_PARAMETER_READBACK id=0 ')
                    row['trials'].append(trial('warmup',run,cgroup))
                    row['drain_checks'] = []
                    good = 0
                    for attempt in range(30):
                        safety(); time.sleep(1)
                        began=time.monotonic_ns()
                        counters=status_values(request('status','RPI1_STATUS '))
                        elapsed=time.monotonic_ns()-began
                        outstanding=counters['callbacks']*512-counters['paused_frames']-counters['bridge_processed']*256
                        check=dict(counters=counters,request_elapsed_ns=elapsed,outstanding_frames_estimate=outstanding)
                        row['drain_checks'].append(check)
                        healthy=(counters['process_failures']==0 and counters['fault']==0 and counters['failures']==0)
                        good=good+1 if healthy and 0<=outstanding<=1024 and elapsed<250_000_000 else 0
                        if good>=3: break
                    else: raise RuntimeError('No comparable live-counter drained start within 30 samples')
                    time.sleep(1)
                    row['guard_at_measurement'] = guard(expected)
                    measured = trial('measured',run,cgroup); row['trials'].append(measured)
                    # Retain post-release CPU/wait samples while outstanding work can finish.
                    for _ in range(10):
                        measured['samples'].append(snapshot(cgroup)); time.sleep(.5)
                    request('mark','RPI1_OBSERVER_MARKER accepted')
                    row['after_identity_response'] = request('save '+str(run/'after.state'),'RPI1_STATE_SAVED ')
                    row['after_identity'] = identify((run/'after.state').read_bytes())
                    row['guard_after'] = guard(expected)
                finally:
                    if session.poll() is None and fifo.exists():
                        try:
                            row['original_restore'] = request('restore '+str(original),'RPI1_STATE_RESTORED ')
                            row['restored_master'] = request('parameter 0','RPI1_PARAMETER_READBACK id=0 ')
                            request('save '+str(run/'restored.state'),'RPI1_STATE_SAVED ')
                            row['restored_identity'] = identify((run/'restored.state').read_bytes())
                        finally: send('quit')
                    session.wait(timeout=35)
                    if (run/'run.json').exists(): row['session'] = json.loads((run/'run.json').read_text())
                    (output/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
            if not row.get('session',{}).get('clean_shutdown'): raise RuntimeError('Session did not close cleanly')
            if row['restored_identity'] != report['original_identity']: raise RuntimeError('State restoration mismatch')
            if not any('normalized=0.48033079504966736 ' in line for line in row['restored_master']): raise RuntimeError('Master restoration mismatch')
            print('CONDITION_DONE '+expected+' '+json.dumps(measured['after']),flush=True)
        report['guard_final'] = guard('unchanged')
    except Exception as exc:
        report['errors'].append(str(exc)); print('EXPERIMENT_STOP '+str(exc),flush=True)
        report['guard_final'] = guard('unchanged')
    finally:
        (output/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
    print('EXPERIMENT_DONE '+json.dumps(report['guard_final']),flush=True)


if __name__ == '__main__':
    main()
