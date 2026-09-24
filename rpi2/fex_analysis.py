"""Offline process-level FEX counter deltas alongside existing audio/CPU reduction."""
import argparse
import json
from pathlib import Path
from fex_stats import COUNTERS, interval
from governor_phase_analysis import interpolate

FREQUENCY=54_000_000
WINDOWS={'idle':(.3,1.3),'attack':(2,3),'held_including_attack':(2,14),'stable_hold':(3,14),'release':(14,20)}


def total(rows):
    out={k:0 for k in COUNTERS}
    for row in rows:
        for k,v in row['matched_slot_deltas'].items():out[k]+=v
    return out


def reduce_trial(trial, pigments_pid=None):
    anchor=trial['graph_observed_ns'];samples=[s['fex'] for s in trial['samples']]
    if len(samples)<2:
        return {'name':trial['name'],'snapshots':len(samples),'unavailable':'Fewer than two FEX snapshots','timeline':[],'windows':{}}
    rows=[]
    for a,b in zip(samples,samples[1:]):
        row=interval(a['stats'],b['stats'])
        row.update(begin_seconds=(a['end_ns']-anchor)/1e9,end_seconds=(b['end_ns']-anchor)/1e9)
        row['timer_elapsed_seconds']={k:v/FREQUENCY for k,v in row['matched_slot_deltas'].items() if k.endswith('_ticks')}
        rows.append(row)
    stable=[r for r in rows if r['observed_identity_stable']]
    result={'name':trial['name'],'snapshots':len(samples),'reader_cpu_seconds':sum(s['observer_cpu_ns'] for s in samples)/1e9,
            'maximum_read_wall_ms':max((s['end_ns']-s['begin_ns'])/1e6 for s in samples),
            'stable_intervals':len(stable),'unstable_intervals':len(rows)-len(stable),'timeline':rows,'windows':{}}
    for name,(a,b) in WINDOWS.items():
        inside=[r for r in stable if a<=r['begin_seconds'] and r['end_seconds']<=b]
        touching=[r for r in stable if r['end_seconds']>a and r['begin_seconds']<b]
        result['windows'][name]={'requested_capture_seconds':[a,b],
            'contained_stable_intervals':len(inside),'contained_seconds':sum(r['end_seconds']-r['begin_seconds'] for r in inside),
            'contained_counter_deltas':total(inside),'overlapping_stable_counter_deltas':total(touching),
            'limitation':'Contained intervals omit edge activity; overlapping intervals include activity outside the window. They do not assign an increment to an exact note/callback.'}
    if pigments_pid is not None and all('threads' in row for row in trial['samples']):
        result.update(held_process_cpu(trial,pigments_pid))
    return result


def held_process_cpu(trial,pigments_pid):
    result={};anchor=trial['graph_observed_ns']
    raw=trial['samples'];begin=anchor+2_000_000_000;end=anchor+14_000_000_000
    ids=set.intersection(*(set(s['threads']) for s in raw))
    companion={key.split(':')[0] for s in raw for key,v in s['threads'].items() if v['name']=='ArturiaSoftware'}
    roles={'pigments_host':0.0,'arturia_named_other_process':0.0,'other_runtime':0.0}
    for key in ids:
        a=interpolate(raw,begin,lambda s:s['threads'][key]['cpu_ns'])
        b=interpolate(raw,end,lambda s:s['threads'][key]['cpu_ns'])
        if a is None or b is None:continue
        pid=key.split(':')[0]
        role='pigments_host' if pid==str(pigments_pid) else 'arturia_named_other_process' if pid in companion else 'other_runtime'
        roles[role]+=(b-a)/12e9*100
    result['held_process_cpu_surviving_threads_one_core_percent']=roles
    result['arturia_named_thread_is_separate_process']=bool(companion-{str(pigments_pid)})
    return result


def zero_runs(path):
    import numpy as np
    data=np.fromfile(path,dtype='<f4')
    if data.size!=960000*2:raise ValueError('Incomplete audio capture')
    silence=np.all(data.reshape(-1,2)==0,axis=1)
    edges=np.flatnonzero(np.diff(np.r_[False,silence,False].astype(np.int8)))
    return [{'begin_seconds':int(a)/48000,'end_seconds':int(b)/48000,'frames':int(b-a)} for a,b in zip(edges[::2],edges[1::2])]


def main():
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);args=p.parse_args()
    raw=json.loads((args.directory/'comparison.json').read_text())
    out={'errors':raw['errors'],'conditions':[],'counter_source_commit':'82510eb452b258959ef982be58a9c3c1bafc82a4',
         'timer':{'raw_unit':'CNTVCT reference timer ticks','conversion_hz':FREQUENCY,
                  'source':'Current boot kernel: arch_timer: cp15 timer running at 54.00MHz (phys). Clocksource arch_sys_counter.',
                  'precision':'Derived seconds use boot-reported frequency; raw ticks retained.'},
         'semantics':{
             'jit_path_ticks':'Elapsed CompileBlock path including precompile, invalidation lock, memory/disk lookup/load and actual compile; not pure compilation CPU time.',
             'jit_count':'CompileCode attempt count, not every cache lookup or necessarily unique successful block.',
             'cache_miss_count':'L1 miss count; an L2/L3 hit can still increment this counter. Not retranslation count.',
             'cache_read_lock_ticks':'Time acquiring shared L2/L3 lookup lock; not whole cache lookup work.',
             'cache_write_lock_ticks':'Time acquiring cache write lock.',
             'counter_times':'Elapsed counters may overlap; never sum as exclusive CPU categories.',
             'identity':'Shared file owned Linux process; slots hold Windows TIDs. No per-audio-thread or Linux TID attribution.',
             'sampling':'Two bounded reads per snapshot with matching topology, but counters are not an atomic whole-process snapshot. Stable observed slots can still hide very rapid reuse.'}}
    for c in raw['conditions']:
        row={'condition':c['condition'],'unavailable':c.get('fex_unavailable')}
        if c.get('fex_at_ready'):
            first=c['fex_at_ready'];h=first['stats']
            row['schema']={k:h[k] for k in ['version','app_type','slot_size','size','fex_build']}
            row['file_namespace']='owned_process_mount' if first['path'].startswith('/proc/') else 'host'
            row['pid_namespace_differs']=len(set(first['namespace_pids']))>1
            row['at_ready_live_slots']=len(h['rows'])
        row['trials']=[reduce_trial(t,c.get('fex_at_ready',{}).get('linux_pid')) for t in c['trials'] if all('fex' in s for s in t['samples'])]
        for trial in row['trials']:
            path=args.directory/(c['condition']+'-'+trial['name']+'.f32le')
            if path.exists():trial['full_capture_zero_runs']=zero_runs(path)
        out['conditions'].append(row)
    (args.directory/'reduced-fex.json').write_text(json.dumps(out,indent=2)+'\n')
    for c in out['conditions']:
        for t in c['trials']:
            if 'unavailable' in t:
                print(t['name'],t['unavailable']);continue
            print(t['name'], 'reader CPU', t['reader_cpu_seconds'], 'stable intervals',t['stable_intervals'], 'unstable',t['unstable_intervals'])
            for name,w in t['windows'].items():print(name,json.dumps(w['contained_counter_deltas']))


if __name__=='__main__':main()
