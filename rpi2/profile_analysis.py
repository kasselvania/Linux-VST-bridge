"""Reduce one retained RPI2 perf capture without publishing vendor data or maps."""
import argparse
from collections import Counter
import heapq
import json
import math
from pathlib import Path
import re
import struct


SAMPLE = re.compile(r'^\s*(.*?)\s+(\d+)/(\d+)\s+(\d+)\.(\d+):\s+(\d+)\s+([a-fA-F0-9]+)\s+(.*?)\s+\(([^()]*)\)\s*$')
GUEST = re.compile(r'([^\\/:]+\.(?:vst3|dll|exe))\+0x([0-9a-fA-F]+)', re.I)
ARITHMETIC_FAMILY = {('pigmentsprocessor.dll',f'0x{rva:x}') for rva in (0x7354e0,0x735530,0x7356e0)}


def parse_samples(path):
    rows = []
    for number, line in enumerate(path.read_text().splitlines(), 1):
        match = SAMPLE.match(line)
        if not match:
            raise ValueError(f'unparsed perf sample at line {number}')
        comm, pid, tid, seconds, fraction, period, ip, symbol, dso = match.groups()
        guest = GUEST.search(symbol)
        owner = (guest[1].lower() if guest else
                 'fex_dispatcher' if symbol.startswith('Dispatch_') else
                 'unresolved_jit' if dso.endswith('.map') or dso == '[unknown]' else
                 Path(dso).name)
        rows.append(dict(comm=comm, pid=int(pid), tid=int(tid),
                         ns=int(seconds)*1_000_000_000+int(fraction.ljust(9, '0')),
                         period=int(period), ip=int(ip, 16), owner=owner,
                         jit_map=dso.endswith('.map'),
                         guest_offset='0x'+guest[2].lower() if guest else None))
    if not rows:
        raise ValueError('no perf samples')
    return rows


def profile_window(rows, begin_ns, end_ns):
    held = [r for r in rows if begin_ns <= r['ns'] < end_ns]
    weight = sum(r['period'] for r in held)
    if not weight:
        raise ValueError('no weighted samples in held window')
    owners = Counter()
    blocks = Counter()
    threads = Counter()
    jit_period = 0
    labeled_jit_period = 0
    for row in held:
        owners[row['owner']] += row['period']
        threads[(row['comm'], row['tid'])] += row['period']
        if row['jit_map']:
            jit_period += row['period']
            if row['guest_offset']:
                labeled_jit_period += row['period']
        if row['guest_offset']:
            blocks[(row['comm'], row['owner'], row['guest_offset'])] += row['period']
    return dict(samples=len(held), weighted_period=weight,
                labeled_jit_percent=round(100*labeled_jit_period/jit_period, 3) if jit_period else None,
                perf_selected_owners_percent={name:round(100*value/weight, 3) for name, value in owners.most_common()},
                threads=[dict(name=name, tid=tid, weighted_period=period)
                         for (name, tid), period in threads.most_common()],
                perf_selected_guest_blocks=[dict(thread=name, module=module, image_rva=offset,
                                       percent_of_all_held_user_cycles=round(100*period/weight, 3))
                                  for (name, module, offset), period in blocks.most_common(8)])


def annotate_map_candidates(path, rows):
    """Check overlapping FEX symbols; retain only normalized module/RVA candidates."""
    entries=[]
    malformed=0
    starts=Counter()
    for line in path.read_text().splitlines():
        match=re.match(r'^0x([0-9a-fA-F]+) ([0-9a-fA-F]+) (\S.*)$',line)
        if not match:
            malformed+=1
            continue
        start=int(match[1],16)
        starts[start]+=1
        size=int(match[2],16)
        guest=GUEST.search(match[3])
        entries.append((start,start+size,
                        (guest[1].lower(),'0x'+guest[2].lower()) if guest else None))
    entries.sort(key=lambda entry:entry[0])
    end_heap=[]
    active={}
    cursor=0
    for row in sorted((r for r in rows if r['jit_map']),key=lambda r:r['ip']):
        ip=row['ip']
        while cursor<len(entries) and entries[cursor][0]<=ip:
            start,end,label=entries[cursor]
            active[cursor]=label
            heapq.heappush(end_heap,(end,cursor))
            cursor+=1
        while end_heap and end_heap[0][0]<=ip:
            _,index=heapq.heappop(end_heap)
            active.pop(index,None)
        row['map_candidates']=set(label for label in active.values() if label is not None)
    return dict(valid_lines=len(entries),malformed_lines=malformed,
                duplicate_starts=sum(count-1 for count in starts.values()))


def map_ambiguity(rows,begin_ns,end_ns):
    held=[r for r in rows if begin_ns<=r['ns']<end_ns]
    weight=sum(r['period'] for r in held)
    ambiguous_rva=ambiguous_module=family_only=0
    lower=Counter()
    for row in held:
        if not row['jit_map']:
            lower[row['owner']]+=row['period']
            continue
        candidates=row.get('map_candidates',set())
        modules=set(module for module,_ in candidates)
        if len(candidates)>1:ambiguous_rva+=row['period']
        if len(modules)>1:ambiguous_module+=row['period']
        if candidates and candidates.issubset(ARITHMETIC_FAMILY):family_only+=row['period']
        if len(modules)==1:lower[next(iter(modules))]+=row['period']
    return dict(multiple_guest_rva_percent=round(100*ambiguous_rva/weight,3),
                cross_module_ambiguous_percent=round(100*ambiguous_module/weight,3),
                arithmetic_family_only_candidates_percent=round(100*family_only/weight,3),
                unambiguous_module_lower_bound_percent_of_all_user_cycles={
                    module:round(100*period/weight,3) for module,period in lower.most_common()})


def first_snapshot_after(samples, anchor, seconds):
    return next((s for s in samples if s['monotonic_ns'] >= anchor+seconds*1_000_000_000), None)


def last_snapshot_before_status_deadline(samples, anchor, seconds):
    return next((s for s in reversed(samples) if s['counter_read_end_ns'] <= anchor+seconds*1_000_000_000), None)


def aligned_work(trial, first_second, last_second, caller_tid, worker_tid):
    """Use observed snapshots, exposing the delayed status-read brackets."""
    anchor = trial['graph_observed_ns']
    first = first_snapshot_after(trial['samples'], anchor, first_second)
    last = last_snapshot_before_status_deadline(trial['samples'], anchor, last_second)
    if first is None or last is None or first is last:
        raise ValueError('aligned snapshot bracket unavailable')
    frames = last['current_counters']['bridge_processed_frames']-first['current_counters']['bridge_processed_frames']
    if frames <= 0 or frames % 256:
        raise ValueError('completed frames missing or not quantum aligned')
    calls = frames//256
    def thread(tid):
        key=f"{trial['profile']['linux_pid']}:{tid}"
        a,b=first['threads'][key],last['threads'][key]
        cpu=b['cpu_ns']-a['cpu_ns']
        wait=b['wait_ns']-a['wait_ns']
        if cpu < 0 or wait < 0:
            raise ValueError('thread counters reset')
        return dict(cpu_seconds=round(cpu/1e9, 6),
                    runnable_wait_seconds=round(wait/1e9, 6),
                    cpu_ms_per_completed_256_frames=round(cpu/calls/1e6, 4),
                    user_ticks=b['user_ticks']-a['user_ticks'],
                    system_ticks=b['system_ticks']-a['system_ticks'],
                    minor_faults=b['minor_faults']-a['minor_faults'],
                    major_faults=b['major_faults']-a['major_faults'])
    def read_bracket(s):
        return dict(snapshot_seconds_after_graph=round((s['monotonic_ns']-anchor)/1e9, 6),
                    status_read_begin_seconds_after_graph=round((s['counter_read_begin_ns']-anchor)/1e9, 6),
                    status_read_end_seconds_after_graph=round((s['counter_read_end_ns']-anchor)/1e9, 6))
    return dict(first=read_bracket(first), last=read_bracket(last),
                snapshot_wall_seconds=round((last['monotonic_ns']-first['monotonic_ns'])/1e9, 6),
                completed_frames=frames, completed_calls=calls,
                delivered_frames=last['current_counters']['delivered_frames']-first['current_counters']['delivered_frames'],
                missing_frames=last['current_counters']['missing_frames']-first['current_counters']['missing_frames'],
                added_gaps=last['current_counters']['gaps']-first['current_counters']['gaps'],
                caller=thread(caller_tid), worker=thread(worker_tid))


def capture_zero_summary(path):
    data=path.read_bytes()
    if len(data)!=20*48000*2*4:
        raise ValueError('incomplete stereo capture')
    counts={name:dict(exact_zero_frames=0,longest_zero_run_frames=0) for name in ('capture_2_to_14_s','capture_4_to_14_s')}
    current={name:0 for name in counts}
    nonfinite=0
    for frame,(left,right) in enumerate(struct.iter_unpack('<ff',data)):
        nonfinite+=int(not math.isfinite(left))+int(not math.isfinite(right))
        for name,start in (('capture_2_to_14_s',2*48000),('capture_4_to_14_s',4*48000)):
            if start<=frame<14*48000:
                if left==0.0 and right==0.0:
                    counts[name]['exact_zero_frames']+=1
                    current[name]+=1
                    counts[name]['longest_zero_run_frames']=max(counts[name]['longest_zero_run_frames'],current[name])
                else:
                    current[name]=0
    if nonfinite:
        raise ValueError(f'capture has {nonfinite} nonfinite samples')
    return counts


def reduce(directory):
    raw=json.loads((directory/'comparison.json').read_text())
    if raw['errors'] or raw.get('guard_final',{}).get('governor') != 'ondemand' or raw.get('guard_final',{}).get('schedstats') != '0':
        raise ValueError('capture errors or settings not restored')
    if not raw.get('preference_restored_exact'):
        raise ValueError('preference restoration unconfirmed')
    condition, = raw['conditions']
    if condition['condition'] != 'profile' or condition['quantum'] != 256:
        raise ValueError('unexpected profile condition')
    result={'schema':'rpi2-critical-profile/v1', 'fixture':dict(processing_quantum=256, map_capacity=512,
              sample_rate_hz=48000, jack_frames=512, reserve_frames=2048,
              notes=[60,64,67,71], note_velocity=96, held_seconds=12,
              preset='24 AM Poly 4', master=0.35, native_phase_trace='off',
              pigments_version='7.0.1.6772', runner='GE-Proton11-7-aarch64',
              umu_version='1.4.4', steam_runtime='steamrt4-arm64',
              fex_version='2609-41-g82510eb', kernel='6.18.50+rpt-rpi-v8',
              native_candidate_sha256='c9709c05f54b3ab665c9cf783b79d83c17645537184fcfbd658d54ca25c40d46',
              windows_candidate_sha256='9a1e93f5fa1c144984f2123d9bdbdb9d7c26744aebfdbfe933085336d329182f'),
            'restoration':dict(preference_exact=True, original_state_matches=condition.get('restored_identity')==raw.get('original_identity'),
                   settings=raw['guard_final'], clean_shutdown=condition.get('session',{}).get('clean_shutdown'),
                   graph_without_owned_nodes=not any('lvb-' in node for node in condition.get('session',{}).get('jack_graph_after','').splitlines())),
            'environment_observation':dict(
                session_temperature_peak_c=max(s['temperature_c'] for s in condition['session']['samples']),
                throttle_flags=sorted(set(s['throttled'] for s in condition['session']['samples']))),
            'trials':[],
            'limitations':[
                'Graph-ready stdout reception approximates capture start; status requests complete about 100 ms after each CPU snapshot. Per-frame CPU is approximate and includes thread work outside processor.process().',
                'Perf samples only user-mode execution; kernel CPU is bounded separately by thread system ticks, without kernel stacks. Scheduler runnable-wait excludes time blocked on a futex or other wait. Weighted samples classify JIT blocks by image, not by FEX translation overhead within a block.',
                'The warm-up profiler consumed substantial CPU and perturbed its failed audio; it is not an unperturbed warm-up baseline. The repeat profiler was light.',
                'Completed processing is distinct from timely delivery and nonzero capture. This 256-frame Pigments fixture is not qualified by the measured repeat.'
            ]}
    for trial in condition['trials']:
        name=trial['name']
        samples=parse_samples(directory/(name+'.samples.txt'))
        map_result=annotate_map_candidates(directory/(name+'.guest.map'),samples)
        anchor=trial['graph_observed_ns']
        held=profile_window(samples,anchor+2_000_000_000,anchor+14_000_000_000)
        held['map_ambiguity']=map_ambiguity(samples,anchor+2_000_000_000,anchor+14_000_000_000)
        held['caller_map_ambiguity']=map_ambiguity([r for r in samples if r['comm']=='lvb-audio'],
                                                   anchor+2_000_000_000,anchor+14_000_000_000)
        capture=capture_zero_summary(directory/(name+'.f32le'))
        delta={key:trial['after'][key]-trial['before'][key] for key in
               ('bridge_processed_frames','delivered_frames','missing_frames','gaps','midi_accepted','xruns','process_failures')}
        row=dict(name=name, counter_delta=delta,
                 held_audio=capture,
                 profiler_cpu_seconds_before_stop=trial['profile']['perf_cpu_seconds_before_stop'],
                 profiler_wakeups=int(re.search(r'Woken up (\d+) times',trial['profile']['stderr']).group(1)),
                 jit_map_quality=map_result,
                 held_profile=held)
        row['temperature_peak_c']=max(s['temperature_c'] for s in trial['samples'])
        row['held_arm_clock_hz_range']=[min(s['arm_clock_hz'] for s in trial['samples'] if anchor+2_000_000_000<=s['monotonic_ns']<anchor+14_000_000_000),
                                         max(s['arm_clock_hz'] for s in trial['samples'] if anchor+2_000_000_000<=s['monotonic_ns']<anchor+14_000_000_000)]
        if name=='measured':
            caller=next(r['tid'] for r in held['threads'] if r['name']=='lvb-audio')
            worker=next(r['tid'] for r in held['threads'] if r['name']=='Processing Thre')
            row['stable_aligned_work']=aligned_work(trial,4,14,caller,worker)
            first=first_snapshot_after(trial['samples'],anchor,4)
            last=last_snapshot_before_status_deadline(trial['samples'],anchor,14)
            row['stable_aligned_profile']=profile_window(samples,first['monotonic_ns'],last['monotonic_ns'])
            row['stable_aligned_profile']['map_ambiguity']=map_ambiguity(samples,first['monotonic_ns'],last['monotonic_ns'])
            before=trial['before']['missing_frames']
            points=[s for s in trial['samples'] if 1 <= (s['monotonic_ns']-anchor)/1e9 <= 7]
            first_loss=next(s for s in points if s['current_counters']['missing_frames']>before)
            peak=max(points,key=lambda s:s['outstanding_frames_estimate'])
            recovered=next(s for s in points if s['monotonic_ns']>peak['monotonic_ns'] and s['outstanding_frames_estimate']<=512)
            row['backlog_observations']=[dict(event=event,
                seconds_after_graph=round((s['monotonic_ns']-anchor)/1e9,3),
                missing_frames_since_trial_start=s['current_counters']['missing_frames']-before,
                current_outstanding_frames_estimate=s['outstanding_frames_estimate'])
                for event,s in (('first_observed_loss',first_loss),('observed_peak',peak),('recovered_to_512_or_less',recovered))]
        result['trials'].append(row)
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('output',type=Path);args=p.parse_args()
    args.output.write_text(json.dumps(reduce(args.directory),indent=2)+'\n')
