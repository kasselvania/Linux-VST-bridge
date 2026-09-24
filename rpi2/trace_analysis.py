"""Offline reduction of one same-binary OFF/ON/OFF; proprietary captures stay private."""
import argparse
import json
from pathlib import Path
import numpy as np
from governor_phase_analysis import blocks, coverage, cpu_window, distribution, interpolate

WINDOWS = {'idle': (.3, 1.3), 'held_including_attack': (2, 14), 'stable_hold': (3, 14)}


def audio(path):
    data = np.fromfile(path, dtype='<f4')
    if data.size != 20 * 48000 * 2:
        raise ValueError('Incomplete stereo capture')
    data = data.reshape(-1, 2)
    silent = np.all(data == 0, axis=1)
    out = {'frames': len(data), 'nonfinite_samples': int((~np.isfinite(data)).sum()),
           'peak': float(np.max(np.abs(data))), 'windows': {}}
    active = np.flatnonzero(~silent[96000:672000])
    out['first_nonzero_after_scheduled_note_on_seconds'] = None if not len(active) else float((active[0]+96000)/48000)
    out['scheduled_note_on_capture_frame'] = 96000
    out['scheduled_note_off_capture_frame'] = 672000
    for name, (a,b) in WINDOWS.items():
        chunk = data[int(a*48000):int(b*48000)]
        mask = np.all(chunk == 0, axis=1)
        edges = np.flatnonzero(np.diff(np.r_[False,mask,False].astype(np.int8)))
        runs = list(zip(edges[::2], edges[1::2]))
        initial = runs[0][1] if runs and runs[0][0] == 0 else 0
        internal = [(x,y) for x,y in runs if x > 0 and y < len(mask)]
        out['windows'][name] = {'capture_seconds': [a,b], 'exact_zero_frames': int(mask.sum()),
            'initial_zero_frames': int(initial), 'longest_zero_run_frames': int(max((y-x for x,y in runs),default=0)),
            'internal_zero_runs': [{'start_seconds': float(a+x/48000), 'frames': int(y-x)} for x,y in internal],
            'rms': float(np.sqrt(np.mean(chunk.astype(np.float64)**2)))}
    return out


def common_cpu(samples, start, end):
    result = cpu_window(samples, start, end, 48000)
    # No matched block cohort is available OFF. Do not turn wall duration into rendered frames.
    del result['cohort_cpu_seconds_per_rendered_second']
    inside=[s for s in samples if start<=s['monotonic_ns']<=end]
    result['sampled_arm_hz_minimum']=min((s['arm_clock_hz'] for s in inside),default=None)
    def delta(getter):
        x,y=interpolate(samples,start,getter),interpolate(samples,end,getter)
        return None if x is None or y is None else y-x
    tids=set.intersection(*(set(s['native_threads']) for s in samples))
    result['native_threads'] = []
    for tid in tids:
        ns=delta(lambda s:s['native_threads'][tid]['cpu_ns'])
        if ns is not None:
            result['native_threads'].append({'name':samples[0]['native_threads'][tid]['name'],
                'cpu_seconds':ns/1e9,'one_core_percent':100*ns/(end-start)})
    result['native_threads'].sort(key=lambda r:r['cpu_seconds'],reverse=True)
    result['native_io_delta']={k:delta(lambda s:int(s['native_io'][k])) for k in ['syscw','wchar','write_bytes']}
    return result


def main():
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);args=p.parse_args()
    raw=json.loads((args.directory/'comparison.json').read_text())
    out={'comparison':raw['comparison'],'errors':raw['errors'],'guard_final':raw.get('guard_final'),
         'conditions':[], 'limitations':[
        'Common CPU windows use graph-ready stdout observation, not an exact MIDI callback timestamp. Audio windows use capture frame positions and retain actual first nonzero output.',
        'First nonzero after frame96000 is not new-note onset when a residual tail is already nonzero before the stimulus. Idle RMS is retained to expose this.',
        'Four note-ons are scheduled at capture frame96000 and note-offs at672000 by the same capture sample counter; this does not establish plugin execution or audible onset at those frames.',
        'Native phase tracing OFF retains callback deadline/scalar output checks, transport/fault clocks, commercial native observer and Windows diagnostics. This is not a fully uninstrumented runtime.',
        'Only ON creates phase records and has before/after/five-second flush marks, including disk sync. These are the intended observer difference.',
        'Before/after status counter deltas span wider than the stimulus hold and include control/marker response time. They are not exact active-block attribution.',
        'Off has no phase attribution. Common wall-window CPU is not CPU per rendered second. Scheduler wait is unavailable with schedstats disabled.',
        'Exact-zero audio frames can include onset latency or preset behavior. Counter-reported missing frames are reported separately; neither nonzero output nor exit0 proves continuity.',
        'One ordered OFF/ON/OFF exposes repeat drift but is not randomized or sustained-performance qualification. No subjective listening or battery energy measurement was made.'
    ]}
    for condition in raw['conditions']:
        row={k:condition[k] for k in ['condition','guard_before','guard_at_measurement','guard_after','trace_readback','native_threads_at_ready','phase_file_count_after','restored_identity','restored_master'] if k in condition}
        row['phase_attribution']='available_on_only' if condition.get('phase_path') else 'unavailable_trace_off'
        row['disabled_mark_reported_unavailable']=any('unavailable phase_trace=off' in s for s in condition.get('disabled_mark',[])) if not condition.get('phase_path') else None
        row['drain_checks']=[{'outstanding_frames_estimate':s['outstanding_frames_estimate'],'request_elapsed_ns':s['request_elapsed_ns']} for s in condition.get('drain_checks',[])]
        session=condition.get('session',{});row['session']={k:session[k] for k in ['ready_seconds','elapsed_seconds','exit_code','clean_shutdown'] if k in session}
        row['session']['graph_restored']=not any('lvb-' in s or s.startswith(' ') for s in session.get('jack_graph_after','').splitlines())
        row['session']['temperature_peak_c']=max((s['temperature_c'] for s in session.get('samples',[])),default=None)
        row['session']['throttled_flags']=sorted(set(s['throttled'] for s in session.get('samples',[])))
        row['trials']=[];out['conditions'].append(row)
        for trial in condition['trials']:
            t={'name':trial['name'],'fixture_exit':trial['fixture_exit'],'capture_observation':trial['capture_observation'],
               'counter_delta':{k:trial['after'][k]-trial['before'][k] for k in ['bridge_processed','missing_frames','gaps','midi_accepted','xruns','process_failures','delivered_frames']},
               'flush_marks_count':len(trial['flush_marks_ns']), 'audio':audio(args.directory/(condition['condition']+'-'+trial['name']+'.f32le'))}
            anchor=trial['graph_observed_ns'];t['common_wall_windows']={}
            t['clock_samples_below_2390mhz']=[{'seconds_after_graph_observation':(s['monotonic_ns']-anchor)/1e9,
                'hz':s['arm_clock_hz']} for s in trial['samples'] if s['arm_clock_hz']<2_390_000_000]
            for name,(a,b) in WINDOWS.items():
                t['common_wall_windows'][name]=common_cpu(trial['samples'],anchor+int(a*1e9),anchor+int(b*1e9))
            if condition.get('phase_path'):
                path=args.directory/Path(condition['phase_path']).name
                bs,pubs,drops=blocks(path,anchor,anchor+20_000_000_000)
                t['on_phase_windows']={}
                for name,(a,b) in WINDOWS.items():
                    subset=[s for s in bs if anchor+a*1e9<=s['queued_ns']<anchor+b*1e9]
                    published=[s for s in pubs if anchor+a*1e9<=s['monotonic_ns']<anchor+b*1e9]
                    t['on_phase_windows'][name]={'coverage':coverage(published,subset,b-a,drops),
                        'vendor_ms':distribution([s['vendor_ms'] for s in subset]),
                        'service_ms':distribution([s['service_ms'] for s in subset]),
                        'queue_ms':distribution([s['queue_ms'] for s in subset])}
            row['trials'].append(t)
    (args.directory/'reduced-trace.json').write_text(json.dumps(out,indent=2)+'\n')
    for c in out['conditions']:
        for t in c['trials']:
            print(c['condition'],t['name'],json.dumps({'counters':t['counter_delta'], 'audio':t['audio'],
                'held_cpu':t['common_wall_windows']['held_including_attack']['cohort_one_core_percent'],
                'stable_cpu':t['common_wall_windows']['stable_hold']['cohort_one_core_percent']}))


if __name__=='__main__':main()
