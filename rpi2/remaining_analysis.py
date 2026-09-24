"""Sanitize the one retained RPI2 kernel/JIT follow-on capture."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re

from profile_analysis import (aligned_work, annotate_map_candidates,
                              first_snapshot_after, last_snapshot_before_status_deadline,
                              map_ambiguity, parse_samples)


HEADER=re.compile(r'^Processing Thre\s+(\d+)/(\d+)\s+(\d+)\.(\d+):\s+(\d+)\s*$')
FRAME=re.compile(r'^\s*[0-9a-f]+\s+(.*?)\s+\(')


def kernel_events(path,worker_tid):
    events=[]
    for block in path.read_text().strip().split('\n\n'):
        lines=block.splitlines()
        match=HEADER.match(lines[0])
        if not match or int(match[2])!=worker_tid:
            raise ValueError('kernel sample identity or format differs')
        symbols=[]
        for line in lines[1:]:
            frame=FRAME.match(line)
            if frame:symbols.append(frame[1].split('+')[0])
        names=set(symbols)
        if any('getrusage' in name for name in names):kind='getrusage'
        elif any('sched_yield' in name for name in names):kind='sched_yield'
        elif any('futex' in name for name in names):kind='futex_wait'
        elif any('fpsimd' in name for name in names):kind='fpsimd_state'
        else:kind='unclassified'
        timestamp=int(match[3])*1_000_000_000+int(match[4].ljust(9,'0'))
        events.append((timestamp,int(match[5]),kind))
    if not events:raise ValueError('no kernel samples')
    return events


def kernel_window(events,begin,end):
    subset=[(period,kind) for timestamp,period,kind in events if begin<=timestamp<end]
    total=sum(period for period,_ in subset)
    if not total:raise ValueError('no kernel cycles in selected window')
    counts=Counter(kind for _,kind in subset)
    weighted=Counter()
    for period,kind in subset:weighted[kind]+=period
    return dict(samples=len(subset),weighted_period=total,
                categories={key:dict(samples=counts[key],weighted_percent=round(100*value/total,2))
                            for key,value in weighted.most_common()})


def reduce(directory):
    report=json.loads((directory/'comparison.json').read_text())
    condition=report['conditions'][0]
    warmup,measured=condition['trials']
    identity=condition['remaining_prepare']['identity']
    profile=measured['profile']
    if identity['pid']!=profile['user']['linux_pid'] or not profile['user']['valid_stop']:
        raise ValueError('user profiler identity/stop differs')
    if not profile['kernel']['perf_exit_code'] in (0,-2):
        raise ValueError('kernel profiler did not finalize')
    if not all(profile['block_bytes_unchanged']):
        raise ValueError('JIT block changed across measured capture')
    if condition['restored_identity']!=report['original_identity'] or not report['preference_restored_exact']:
        raise ValueError('original state/preference restoration missing')
    if report['guard_restore']['phase']!='restored' or not condition['session']['clean_shutdown']:
        raise ValueError('guard/session restoration missing')
    if measured.get('fixture_exit') is not None or not report['errors']:
        raise ValueError('expected partial/aborted measured fixture')
    trial=dict(measured,profile=profile['user'])
    progress=aligned_work(trial,4,6.5,identity['caller_tid'],identity['worker_tid'])
    rows=parse_samples(directory/'user.parsed.samples.txt')
    map_coverage=annotate_map_candidates(directory/'measured.guest.map',rows)
    graph=measured['graph_observed_ns']
    user_window=(graph+2_000_000_000,graph+12_000_000_000)
    user=[row for row in rows if user_window[0]<=row['ns']<user_window[1]]
    total=sum(row['period'] for row in user)
    blocks={(block['begin'],block['tail'],block['end']) for block in profile['validated_blocks']}
    exact=sum(row['period'] for row in user if any(begin+4<=row['ip']<tail for begin,tail,_ in blocks))
    kernel=kernel_events(directory/'kernel.samples.txt',identity['worker_tid'])
    first_row=first_snapshot_after(measured['samples'],graph,4)
    last_row=last_snapshot_before_status_deadline(measured['samples'],graph,6.5)
    first=first_row['monotonic_ns'];last=last_row['monotonic_ns']
    progress_user=[row for row in rows if first<=row['ns']<last]
    progress_total=sum(row['period'] for row in progress_user)
    progress_exact=sum(row['period'] for row in progress_user if any(begin+4<=row['ip']<tail for begin,tail,_ in blocks))
    after=measured['samples'][-1]['current_counters']
    before=measured['samples'][0]['current_counters']
    warm_before=warmup['before'];warm_after=warmup['after']
    return dict(
        fixture='Pigments 7.0.1.6772, 24 AM Poly 4 master 0.35, automated four-note chord, 48 kHz/JACK 512/reserve 2048, vendor quantum 256',
        installed_wine_ntdll_sha256='6ba770ec0df52311b760a0866226a83c177f67227598f990942382cad42b4bc3',
        capture_status='partial: live audio-statistics guard contention made one status response unavailable; controller stopped the measured four-note fixture',
        restoration=dict(state=True,preference=True,governor='ondemand',schedstats='0',session_clean=True),
        warmup=dict(missing_frames_added=warm_after['missing_frames']-warm_before['missing_frames'],
                    gaps_added=warm_after['gaps']-warm_before['gaps'],perf_active=False),
        measured_observed=dict(snapshot_seconds_after_graph=round((measured['samples'][-1]['monotonic_ns']-graph)/1e9,3),
                               missing_frames_added=after['missing_frames']-before['missing_frames'],
                               gaps_added=after['gaps']-before['gaps'],
                               maximum_sampled_temperature_c=max(row['temperature_c'] for row in measured['samples'])),
        aligned_progress=progress,
        observer=dict(user_perf_cpu_seconds_before_stop=profile['user']['perf_cpu_seconds_before_stop'],
                      kernel_perf_cpu_seconds_before_stop=profile['kernel']['perf_cpu_seconds_before_stop'],
                      kernel_perf_bytes=profile['kernel']['perf_bytes']),
        kernel_capture=kernel_window(kernel,0,10**20),
        kernel_progress=kernel_window(kernel,first,last),
        jit=dict(distinct_validated_blocks=len(blocks),all_block_bytes_unchanged=True,
                 validated_block_size_bytes=sorted({end-begin for begin,_,end in blocks}),
                 emitted_arm_code_bytes=sorted({tail-begin-4 for begin,tail,_ in blocks}),
                 measured_user_samples=len(user),
                 exact_validated_block_percent_of_user_cycle_period=round(100*exact/total,2),
                 aligned_progress_user_samples=len(progress_user),
                 aligned_progress_exact_block_percent=round(100*progress_exact/progress_total,2),
                 arithmetic_family_map_candidate_percent=map_ambiguity(rows,*user_window)['arithmetic_family_only_candidates_percent'],
                 map_valid_lines=map_coverage['valid_lines'],map_malformed_lines=map_coverage['malformed_lines'],
                 limitation='FEX map overlaps; exact validated blocks cover only selected code ranges, and sparse RIP entries do not identify each guest instruction.'),
        limits=['Kernel stack categories are cycle-period samples, not blocked duration or exact call totals.',
                'ARM samples can skid; this aborted run cannot establish a diagnostics-disabled improvement.'])


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('private_directory',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    args.output.write_text(json.dumps(reduce(args.private_directory),indent=2)+'\n')
