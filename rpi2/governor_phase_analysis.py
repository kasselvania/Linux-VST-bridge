"""Offline phase reduction; no live mutation or instrumentation installation."""
import argparse
import bisect
import json
from pathlib import Path
import statistics


def distribution(values):
    values = sorted(values)
    if not values: return None
    def percentile(p): return values[min(len(values)-1, int((len(values)-1)*p))]
    return dict(count=len(values), mean=statistics.mean(values), p50=percentile(.5),
                p95=percentile(.95), p99=percentile(.99), maximum=values[-1])


def blocks(path, start, end):
    selected, pending = {}, {}
    dropped = []
    phases = {'worker_request_observed','worker_process_begin','worker_validated',
              'worker_result_published'}
    with Path(path).open() as source:
        for line in source:
            row = json.loads(line)
            if row.get('event') == 'rpi1_phase_dropped':
                dropped.append(dict(producer=row['producer'], count=row['count']))
            if row.get('event') != 'rpi1_phase': continue
            key = (row['callback_sequence'], row['bridge_position'])
            if row['phase'] == 'request_published' and start <= row['monotonic_ns'] < end:
                if key in selected: raise ValueError('Duplicate published request key')
                selected[key] = {'queued':row}
            elif row['phase'] in phases and start <= row['monotonic_ns'] < end+30_000_000_000:
                pending.setdefault(key,{})[row['phase']] = row
    result = []
    for key, row in selected.items():
        row.update(pending.get(key,{}))
        queued = row['queued']; observed = row.get('worker_request_observed')
        if observed and (observed['detail'] != 3 or observed['value_1'] != queued['value_1']):
            raise ValueError('Published/observed request identity mismatch')
        if not all(k in row for k in phases): continue
        begin, done = row['worker_process_begin'], row['worker_validated']
        wait = observed['monotonic_ns'] - queued['monotonic_ns']
        service = done['monotonic_ns'] - begin['monotonic_ns']
        if min(wait, service, done['value_2']) < 0: raise ValueError('Negative duration')
        result.append(dict(position=key[1], queued_ns=queued['monotonic_ns'],
            begin_ns=begin['monotonic_ns'], done_ns=done['monotonic_ns'],
            frames=queued['value_2'], vendor_ms=done['value_2']/1e6,
            service_ms=service/1e6, queue_ms=wait/1e6))
    return result, [r['queued'] for r in selected.values()], dropped


def coverage(published, completed, seconds, drops):
    positions = sorted(r['bridge_position'] for r in published)
    if any(r['value_2'] != 256 for r in published):
        raise ValueError('Unexpected process quantum')
    nominal = round(seconds*48000/256)
    span = (positions[-1]-positions[0])//256+1 if positions else 0
    holes = span-len(positions)
    complete = (abs(len(positions)-nominal)<=2 and holes==0
                and len(completed)==len(published) and not drops)
    return dict(nominal_blocks=nominal, boundary_tolerance_blocks=2,
                position_span_expected_blocks=span, retained_published_blocks=len(published),
                matched_completed_blocks=len(completed), missing_positions_inside_span=holes,
                complete=complete)


def interpolate(samples, timestamp, getter):
    times = [s['monotonic_ns'] for s in samples]
    i = bisect.bisect_right(times, timestamp)
    if not 0 < i < len(times): return None
    a,b = samples[i-1], samples[i]
    x,y = getter(a), getter(b)
    if x is None or y is None: return None
    return x+(y-x)*(timestamp-times[i-1])/(times[i]-times[i-1])


def cpu_window(samples, start, end, frames):
    result = {'start_ns':start,'end_ns':end,'wall_seconds':(end-start)/1e9}
    def delta(getter):
        a,b = interpolate(samples,start,getter),interpolate(samples,end,getter)
        return None if a is None or b is None else b-a
    usage = delta(lambda s:int(s['cohort_cpu_stat']['usage_usec']))
    result['cohort_cpu_seconds'] = None if usage is None else usage/1e6
    result['cohort_cpu_seconds_per_rendered_second'] = None if usage is None else usage/1e6/(frames/48000)
    result['cohort_one_core_percent'] = None if usage is None else 100*usage/1e6/result['wall_seconds']
    total = delta(lambda s:sum(s['cpu'])); idle = delta(lambda s:sum(s['cpu'][3:5]))
    result['whole_pi_cpu_percent'] = None if not total or idle is None else 100*(total-idle)/total
    inside = [s for s in samples if start <= s['monotonic_ns'] <= end]
    result['sampled_arm_hz'] = distribution([s['arm_clock_hz'] for s in inside])
    result['temperature_c'] = distribution([s['temperature_c'] for s in inside])
    result['throttled_flags'] = sorted(set(s['throttled'] for s in inside))
    ids = set.intersection(*(set(s['threads']) for s in samples)) if samples else set()
    threads = []
    for tid in ids:
        cpu = delta(lambda s:s['threads'][tid]['cpu_ns'])
        wait = delta(lambda s:s['threads'][tid]['wait_ns'])
        if cpu is not None:
            threads.append(dict(name=samples[0]['threads'][tid]['name'],cpu_seconds=cpu/1e9,
                one_core_percent=100*cpu/(end-start), scheduler_wait_seconds=None if wait is None else wait/1e9))
    result['surviving_threads_by_cpu'] = sorted(threads,key=lambda t:t['cpu_seconds'],reverse=True)[:8]
    return result


def main():
    p=argparse.ArgumentParser();p.add_argument('comparison',type=Path);p.add_argument('output',type=Path)
    args=p.parse_args();raw=json.loads(args.comparison.read_text())
    report={'conditions':[],'experiment_errors':raw['errors'],'guard_final':raw.get('guard_final')}
    for condition in raw['conditions']:
        out={k:condition[k] for k in ('condition','guard_before','drain_checks','restored_identity','restored_master') if k in condition}
        report['conditions'].append(out)
        session=condition.get('session',{})
        out['session']={k:session[k] for k in ('ready','ready_seconds','elapsed_seconds','exit_code','clean_shutdown') if k in session}
        out['session']['graph_restored']=not any('lvb-' in l or l.startswith(' ') for l in session.get('jack_graph_after','').splitlines())
        if session.get('samples'):
            out['session']['temperature_peak_c']=max(s['temperature_c'] for s in session['samples'])
            out['session']['throttled_flags']=sorted(set(s['throttled'] for s in session['samples']))
        measured=next((t for t in condition['trials'] if t['name']=='measured'),None)
        if measured is None:continue
        out['guard_at_measurement']=condition['guard_at_measurement'];out['guard_after']=condition['guard_after']
        out['fixture_exit']=measured['fixture_exit']
        out['counter_delta']={k:measured['after'][k]-measured['before'][k] for k in
            ('bridge_processed','midi_accepted','missing_frames','gaps','xruns','process_failures','delivered_frames')}
        anchor=measured['graph_observed_ns'];out['stimulus_graph_observed_ns']=anchor
        allblocks,published,dropped=blocks(condition['phase_path'],anchor,anchor+20_000_000_000)
        out.update(published_blocks=len(published),matched_completed_blocks=len(allblocks),phase_drops=dropped)
        out['capture_observation']=measured.get('capture_observation')
        out['flush_marks_ns']=measured.get('flush_marks_ns')
        out['windows']={}
        for name,(a,b) in {'idle':(.3,1.3),'note_active':(3,13),'release':(15,17)}.items():
            cohort=[x for x in allblocks if anchor+a*1e9<=x['queued_ns']<anchor+b*1e9]
            queued=[x for x in published if anchor+a*1e9<=x['monotonic_ns']<anchor+b*1e9]
            retained=coverage(queued,cohort,b-a,dropped)
            if not cohort:
                out['windows'][name]={'unavailable':'No matched completed blocks','coverage':retained};continue
            frames=sum(x['frames'] for x in cohort)
            start,end=min(x['begin_ns'] for x in cohort),max(x['done_ns'] for x in cohort)
            out['windows'][name]=dict(queued_stimulus_window_seconds=[a,b],blocks=len(cohort),frames=frames,
                rendered_seconds=frames/48000,coverage=retained,position_first=cohort[0]['position'],position_last=cohort[-1]['position'],
                vendor_ms=distribution([x['vendor_ms'] for x in cohort]),service_ms=distribution([x['service_ms'] for x in cohort]),
                queue_ms=distribution([x['queue_ms'] for x in cohort]),
                slow_vendor_blocks=sum(x['vendor_ms']>x['frames']/48 for x in cohort),
                slow_service_blocks=sum(x['service_ms']>x['frames']/48 for x in cohort),
                cpu=cpu_window(measured['samples'],start,end,frames))
        out['queue_all_measured_ms']=distribution([x['queue_ms'] for x in allblocks])
        out['active_coverage_valid']=out['windows']['note_active']['coverage']['complete']
    report['measurement_limits']=[
        'Stimulus anchor is observation of the existing fixture graph-ready stdout, immediately before arming. Exact MIDI callback timestamp is unavailable; conservative interior windows exclude note edges.',
        'Blocks are grouped by queued stimulus time and joined by callback sequence/bridge position with matching request IDs. Processing time is associated with those blocks despite later execution.',
        'CPU and scheduler wait use interpolated cumulative samples across each selected cohort execution span, not pure vendor-call CPU. Other cohort threads and small gaps between blocks are included.',
        'CPU per rendered second uses only matched completed frames; it does not imply measured energy savings.',
        'Per-thread list covers threads present across all samples; short-lived threads may be absent. Cgroup CPU includes the full Windows runtime cohort.',
        'Detailed Windows caller CPU and FEX internal counters were not enabled or collected. Runtime and synchronization backend are unchanged.',
        'One ordered A/B/A comparison is not randomized repeated or sustained-performance qualification.'
        ,'Identical explicit marks request phase flushes before/after each capture and every five seconds during it; marker dispatch, serialization and disk sync add observer cost. Actual mark times are retained.'
        ,'Coverage includes nominal block count, retained position-span holes, matching completions and phase drops. Incomplete coverage must not be interpreted as faster processing or reduced CPU work.'
    ]
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    for row in report['conditions']:
        active=row.get('windows',{}).get('note_active',{})
        print(row['condition'],json.dumps(active),flush=True)


if __name__=='__main__':main()
