#!/usr/bin/python3
"""Bounded recorder pair or same-binary phase OFF/ON/OFF; unchanged ondemand."""
import argparse
import json
import os
from pathlib import Path
import re
import hashlib
import shutil
import subprocess
import threading
import time
from pigments_preset_identity import identify

CONTROL = Path('/run/lvb-rpi2-governor-guard')


def guard(expected):
    governor = Path('/sys/devices/system/cpu/cpufreq/policy0/scaling_governor').read_text().strip()
    sched = Path('/proc/sys/kernel/sched_schedstats').read_text().strip()
    wanted=('performance','1') if expected=='profile' else ('ondemand','0')
    if (governor,sched) != wanted:
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


def outstanding_frames(counters, quantum=None):
    if quantum is None:
        rendered = counters['bridge_processed'] * 256
    else:
        if counters.get('processing_quantum') != quantum:
            raise ValueError('Runtime quantum differs from selected condition')
        rendered = counters['bridge_processed_frames']
    return counters['callbacks'] * 512 - counters['paused_frames'] - rendered


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--attempt', default='01')
    parser.add_argument('--original', type=Path)
    modes=parser.add_mutually_exclusive_group()
    modes.add_argument('--trace-comparison', action='store_true')
    modes.add_argument('--fex-stats', action='store_true')
    modes.add_argument('--quantum-comparison', action='store_true')
    modes.add_argument('--critical-profile', action='store_true')
    parser.add_argument('--config', type=Path)
    parser.add_argument('--candidate', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9]{2}', args.attempt):
        raise ValueError('Attempt must be two digits')
    same_candidate=args.trace_comparison or args.fex_stats or args.quantum_comparison or args.critical_profile
    if not same_candidate and args.original is None:
        parser.error('--original is required for the original/buffered pair')
    root, output = args.root, args.output
    output.mkdir(mode=0o700)
    env = root / 'pigments-arm'
    config = args.config or env/'preset-navigation.conf'
    original = env / 'logs/preset-buttons-01/physical-navigation-final.state'
    selected = env / 'logs/preset-sweep-01/005-trial-start.state'
    report = {'conditions': [], 'original_identity': identify(original.read_bytes()),
              'selected_identity': identify(selected.read_bytes()), 'errors': []}
    report['clock_ticks_per_second'] = os.sysconf('SC_CLK_TCK')
    expected = None
    session = None
    fifo = None
    log = None
    fex = None
    preference_backup = None
    profile = None
    if args.critical_profile:
        from critical_profile import Profile, finish_guard
    if args.fex_stats:
        from fex_stats import Reader
        if hashlib.sha256(args.candidate.read_bytes()).hexdigest() != '85780f4856f2bac9f605628ca7decd80568cca0ad36fa92db70960bf74dd1a62':
            raise ValueError('Pinned candidate changed')
    if args.fex_stats or args.quantum_comparison or args.critical_profile:
        preference_path=env/'compatdata/pfx/drive_c/ProgramData/Arturia/Pigments/tmp/plugin.pref.xml'
        preference_backup=output/'plugin.pref.original.xml'
        shutil.copy2(preference_path,preference_backup)
        report['preference_original_sha256']=hashlib.sha256(preference_backup.read_bytes()).hexdigest()

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
                  'phase_file_bytes': phase.stat().st_size if phase else None,
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
                            wait_ns=sched[1] if result['schedstats_enabled'] else None,
                            user_ticks=int(fields[11]),system_ticks=int(fields[12]),minor_faults=int(fields[7]),major_faults=int(fields[9]))
                    except (FileNotFoundError, ProcessLookupError): pass
        if fex is not None:
            result['fex'] = fex.snapshot()
        if profile is not None:
            result['progress']=profile.progress()
            started=time.monotonic_ns()
            result['current_counters']=status_values(request('status','RPI1_STATUS '))
            result['counter_read_begin_ns']=started
            result['counter_read_end_ns']=time.monotonic_ns()
            result['outstanding_frames_estimate']=outstanding_frames(result['current_counters'],256)
        return result

    def trial(name, run, cgroup):
        row = {'name': name, 'samples': [], 'before': status_values(request('status','RPI1_STATUS '))}
        active_trials.append(row)
        row['flush_marks_ns'] = []
        def flush_mark():
            request('mark', 'RPI1_OBSERVER_MARKER accepted')
            row['flush_marks_ns'].append(time.monotonic_ns())
        if trace_on: flush_mark()
        fixture = root/'staging/four-notes-20260923/qualification-four-notes'
        if profile is not None:profile.start(run,name)
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
                if trace_on and time.monotonic() >= next_flush:
                    flush_mark()
                    next_flush += 5
                if time.monotonic()-started > 35: raise RuntimeError('Stimulus timeout')
                time.sleep(.5)
        finally:
            if process.poll() is None: process.terminate(); process.wait(timeout=5)
            reader.join(timeout=3)
            if profile is not None:row['profile']=profile.stop()
        row['fixture_exit'] = process.returncode
        if trace_on: flush_mark()
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
        conditions = ('profile',) if args.critical_profile else ('q256-first','q512','q256-last') if args.quantum_comparison else ('fex',) if args.fex_stats else ('off1', 'on', 'off2') if args.trace_comparison else ('original', 'buffered')
        report['comparison'] = 'critical-path-profile' if args.critical_profile else 'same-pair-quantum-256-512-256' if args.quantum_comparison else 'fex-existing-stats' if args.fex_stats else 'same-binary-phase-off-on-off' if args.trace_comparison else 'original-buffered'
        for index, expected in enumerate(conditions):
            trace_on = not same_candidate or expected == 'on'
            quantum = 512 if expected == 'q512' else 256
            state, _, _ = safety()
            row = {'condition': expected, 'guard_before': state, 'trials': []}
            report['conditions'].append(row)
            active_trials = row['trials']
            label = ('critical-' if args.critical_profile else 'quantum-' if args.quantum_comparison else 'fex-observe-' if args.fex_stats else 'trace-' if args.trace_comparison else 'obs-') + expected.replace('_','-') + '-' + args.attempt
            run = env/'logs'/label
            fifo, log = run/'commands.fifo', run/'outer.log'
            before = set((env/'evidence').glob('*-phase.jsonl'))
            with (output/(label+'-session.log')).open('x') as sink:
                session = subprocess.Popen(['flock', '--nonblock', str(root.parent/'rpi1-private/operation.lock'),
                    '/usr/bin/python3', str(env/('quantum_session.py' if args.quantum_comparison or args.critical_profile else 'trace_session.py' if same_candidate else 'plugin_session.py')), label, '--config', str(config),
                    '--binary', str(args.candidate if same_candidate or index else args.original),
                    *(['--phase-trace', 'on' if trace_on else 'off'] if same_candidate else []),
                    *(['--processing-quantum',str(quantum)] if args.quantum_comparison or args.critical_profile else [])], stdout=sink, stderr=subprocess.STDOUT)
                end = time.monotonic()+100
                try:
                    while (not log.exists() or 'RPI1_READY ' not in log.read_text()
                           or (args.quantum_comparison or args.critical_profile) and 'RPI2_PROCESSING ' not in log.read_text()):
                        safety()
                        if time.monotonic()>end or session.poll() is not None: raise RuntimeError('Startup failed')
                        time.sleep(.5)
                    if args.quantum_comparison or args.critical_profile:
                        wanted=f'RPI2_PROCESSING quantum={quantum} map_version=2 capacity=512 sample_rate=48000'
                        if wanted not in log.read_text(): raise RuntimeError('Processing quantum/layout readback mismatch')
                        row['processing_readback']=wanted
                        row['quantum']=quantum
                    phases = set((env/'evidence').glob('*-phase.jsonl'))-before
                    if len(phases) != int(trace_on): raise RuntimeError('Phase ownership/mode mismatch')
                    phase = phases.pop() if trace_on else None
                    row['phase_path'] = str(phase) if phase else None
                    row['run_path'] = str(run)
                    if same_candidate:
                        wanted = 'RPI1_PHASE_TRACE mode=on recorder=true file=true' if trace_on else 'RPI1_PHASE_TRACE mode=off recorder=false file=false'
                        if wanted not in log.read_text(): raise RuntimeError('Trace startup readback mismatch')
                        row['trace_readback'] = wanted
                        if not trace_on:
                            row['disabled_mark'] = request('mark','RPI1_OBSERVER_MARKER unavailable phase_trace=off')
                    units = subprocess.check_output(['systemctl','--user','list-units','--state=running','--plain','--no-legend','lvb-rpi1-*.service'], text=True).splitlines()
                    if len(units)!=1: raise RuntimeError('Windows cohort ownership ambiguous')
                    group = subprocess.check_output(['systemctl','--user','show',units[0].split()[0],'-p','ControlGroup','--value'],text=True).strip()
                    cgroup = Path('/sys/fs/cgroup')/group.lstrip('/')
                    native_pid=subprocess.check_output(['systemctl','--user','show','lvb-rpi2-'+label+'.service','-p','MainPID','--value'],text=True).strip()
                    row['native_threads_at_ready'] = [task.joinpath('comm').read_text().strip() for task in Path('/proc',native_pid,'task').glob('*')]
                    if same_candidate and ('rpi1-phase-drai' in row['native_threads_at_ready'] or 'rpi1-phase-drain' in row['native_threads_at_ready']) != trace_on:
                        raise RuntimeError('Phase recorder thread/mode mismatch')
                    if args.quantum_comparison or args.critical_profile:
                        module=(env/'compatdata/pfx/drive_c/Program Files/Common Files/VST3/Pigments.vst3').stat()
                        matches=[]
                        for procfile in cgroup.rglob('cgroup.procs'):
                            for pid in procfile.read_text().split():
                                try:
                                    for mapped in Path('/proc',pid,'maps').read_text().splitlines():
                                        fields=mapped.split(); dev=fields[3].split(':')
                                        if int(fields[4])==module.st_ino and int(dev[0],16)==os.major(module.st_dev) and int(dev[1],16)==os.minor(module.st_dev):
                                            matches.append(pid);break
                                except (FileNotFoundError,ProcessLookupError):pass
                        if len(set(matches))!=1:raise RuntimeError('Pigments process identity ambiguous')
                        row['pigments_pid']=matches[0]
                        if args.critical_profile:
                            sid=units[0].split()[0].removeprefix('lvb-rpi1-').removesuffix('.service')
                            profile=Profile(root,matches[0],sid)
                    if args.fex_stats:
                        try:
                            fex=Reader.for_cohort(cgroup,env/'compatdata/pfx/drive_c/Program Files/Common Files/VST3/Pigments.vst3')
                            row['fex_at_ready']=fex.snapshot()
                            print('FEX_STATS_AVAILABLE '+str(fex.pid),flush=True)
                        except (OSError,ValueError) as exc:
                            row['fex_unavailable']=str(exc)
                            raise RuntimeError('FEX stats unavailable after READY; no notes sent') from exc
                    row['restore'] = request('restore '+str(selected),'RPI1_STATE_RESTORED ')
                    time.sleep(4)
                    row['master'] = request('parameter 0','RPI1_PARAMETER_READBACK id=0 ')
                    trial('warmup',run,cgroup)
                    row['drain_checks'] = []
                    good = 0
                    for attempt in range(30):
                        safety(); time.sleep(1)
                        began=time.monotonic_ns()
                        counters=status_values(request('status','RPI1_STATUS '))
                        elapsed=time.monotonic_ns()-began
                        outstanding=outstanding_frames(counters, quantum if args.quantum_comparison or args.critical_profile else None)
                        check=dict(counters=counters,request_elapsed_ns=elapsed,outstanding_frames_estimate=outstanding)
                        row['drain_checks'].append(check)
                        healthy=(counters['process_failures']==0 and counters['fault']==0 and counters['failures']==0)
                        good=good+1 if healthy and 0<=outstanding<=1024 and elapsed<250_000_000 else 0
                        if good>=3: break
                    else: raise RuntimeError('No comparable live-counter drained start within 30 samples')
                    time.sleep(1)
                    row['guard_at_measurement'] = guard(expected)
                    measured = trial('measured',run,cgroup)
                    # Retain post-release CPU/wait samples while outstanding work can finish.
                    for _ in range(10):
                        measured['samples'].append(snapshot(cgroup)); time.sleep(.5)
                    if trace_on: request('mark','RPI1_OBSERVER_MARKER accepted')
                    row['after_identity_response'] = request('save '+str(run/'after.state'),'RPI1_STATE_SAVED ')
                    row['after_identity'] = identify((run/'after.state').read_bytes())
                    row['guard_after'] = guard(expected)
                    row['phase_file_count_after'] = len(set((env/'evidence').glob('*-phase.jsonl'))-before)
                    if same_candidate and row['phase_file_count_after'] != int(trace_on):
                        raise RuntimeError('Phase file appeared/disappeared during trial')
                finally:
                    if session.poll() is None and fifo.exists():
                        try:
                            row['original_restore'] = request('restore '+str(original),'RPI1_STATE_RESTORED ')
                            row['restored_master'] = request('parameter 0','RPI1_PARAMETER_READBACK id=0 ')
                            request('save '+str(run/'restored.state'),'RPI1_STATE_SAVED ')
                            row['restored_identity'] = identify((run/'restored.state').read_bytes())
                        finally: send('quit')
                    session.wait(timeout=35)
                    if fex is not None: fex.close(); fex=None
                    if (run/'run.json').exists(): row['session'] = json.loads((run/'run.json').read_text())
                    (output/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
            if not row.get('session',{}).get('clean_shutdown'): raise RuntimeError('Session did not close cleanly')
            if row['restored_identity'] != report['original_identity']: raise RuntimeError('State restoration mismatch')
            if not any('normalized=0.48033079504966736 ' in line for line in row['restored_master']): raise RuntimeError('Master restoration mismatch')
            print('CONDITION_DONE '+expected+' '+json.dumps(measured['after']),flush=True)
        if args.critical_profile:report['guard_restore']=finish_guard()
        report['guard_final'] = guard('unchanged')
    except Exception as exc:
        report['errors'].append(str(exc)); print('EXPERIMENT_STOP '+str(exc),flush=True)
        if args.critical_profile:report['guard_restore']=finish_guard()
        report['guard_final'] = guard('unchanged')
    finally:
        if preference_backup is not None:
            if session is not None and session.poll() is None:
                raise RuntimeError('Cannot restore preference while owned session is live')
            units=subprocess.check_output(['systemctl','--user','list-units','--state=running','--plain','--no-legend','lvb-rpi1-*.service'],text=True)
            if units.strip():raise RuntimeError('Cannot restore preference with an owned Windows cohort live')
            shutil.copy2(preference_backup,preference_path)
            report['preference_restored_exact']=preference_path.read_bytes()==preference_backup.read_bytes()
        (output/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
    print('EXPERIMENT_DONE '+json.dumps(report['guard_final']),flush=True)


if __name__ == '__main__':
    main()
