"""Allowlisted projection input for the Rust UIO1 report owner.

Private HWNDs/process/session IDs, paths, frames and logs never enter the public
report. Pixel summaries are hashes only. Posted-message retrieval and sent-call
entry/return remain distinct; a next-hook result is not a window-proc result.
"""
import collections
import json
import pathlib
import sys

def project(raw):
    origin=raw['process_before']['at'];q0=min([b['windows_qpc'] for b in raw['brackets']]+[r['qpc'] for r in raw['win32']])
    windows=[r for r in raw['windows'] if r['type']=='window'];aliases={r['hwnd']:i+1 for i,r in enumerate(windows)}
    starts={r['action']:r['interval_ns'][0] for r in raw['inputs'] if r['kind']=='action_begin'}
    report=dict(schema=1,profile_fingerprint=raw['profile_fingerprint'],adapter=raw.get('adapter','xtest'),diagnostic_enabled=True,
        capacity=16384,dropped=raw['observer_status']['dropped'],
        actions=[dict(id=a,kind=k,witnesses=[]) for a,k in [(1,'host_parameter'),(2,'parameter_drag'),(3,'page_click'),(4,'idle')] if a in starts],
        brackets=[dict(linux_before_ns=b['linux_before_ns']-origin,linux_after_ns=b['linux_after_ns']-origin,
            windows_qpc=b['windows_qpc']-q0,frequency=b['frequency']) for b in raw['brackets']])
    observations=[]
    def add(action,kind,*,interval=None,tick=None,target=1,message=None,parameter=None,value=None,**kw):
        if action not in starts:return
        witness=dict(kind=kind,linux_interval_ns=[0,0],target_alias=target,message=message,parameter=parameter,value=value,**kw)
        t=dict(domain='windows_qpc',ticks=tick-q0) if tick is not None else dict(domain='linux',interval=[max(0,v-origin) for v in interval])
        observations.append(dict(action=action,time=t,witness=witness))
    for r in raw['inputs']:
        if r['kind'] not in ('action_begin','activation','motion_settled'):add(r['action'],'input_issued',interval=r['interval_ns'])
    # X server time is a separate domain. The public Linux interval conservatively
    # spans action admission to observer receipt, not a false synchronized stamp.
    for r in raw['x11']:
        add(r['action'],'x11_received',interval=[starts[r['action']],r['observed_ns']],message=r['kind'],client_coordinates=r['client'])
    kinds={1:'win32_retrieved',2:'dispatch_entered',3:'dispatch_returned',4:'heartbeat_acknowledged',
        5:'mouse_hook_entered',6:'mouse_hook_returned',7:'mouse_hook_entered',8:'mouse_hook_returned'}
    for r in raw['win32']:
        if r['source'] not in kinds:continue
        add(r['action'],kinds[r['source']],tick=r['qpc'],target=aliases.get(r['hwnd'],0),message=r['message'],
            focus_alias=aliases.get(r['focus'],0),active_alias=aliases.get(r['active'],0),capture_alias=aliases.get(r['capture'],0),
            client_coordinates=[r['x'],r['y']],result=r['result'])
    kinds={3:'host_value',101:'gesture_begin',102:'parameter_value',103:'gesture_end'}
    for r in raw['gui']:
        if r['kind'] not in kinds or r['action'] not in starts:continue
        add(r['action'],kinds[r['kind']],interval=[starts[r['action']],r['observed_ns']],parameter=r['parameter'],value=r['value'])
    for action in starts:
        frames=[f for f in raw['frames'] if f['action']==action]
        if not frames:continue
        if raw.get('adapter')=='human' and action!=3:continue # FX macro ROI is not the Play-page macro.
        region=1 if action==3 else 0;prior=frames[0]['region_hashes'][region]
        for i,f in enumerate(frames[1:],1):
            if f['region_hashes'][region]!=prior:
                add(action,'first_pixel_change',interval=[frames[i-1]['interval_ns'][0],f['interval_ns'][1]]);break
        # A stable hash is not declared to be a semantic page signature. Retain
        # that distinction in facts; no inferred "Play selected" event is added.
    status_fields=('committed','dropped','ready','closed','hook_calls','hook_ticks',
        'max_hook_ticks','filtered','heartbeat_errors','scope_errors','unhook_errors','detached')
    cleanup=raw.get('helper_cleanup') or {}
    cleanup_public={k:cleanup.get(k) for k in ('exit','kept','overflow','stderr_bytes')}
    cleanup_public['cleanup']={k:cleanup.get('cleanup',{}).get(k) for k in ('owned_descendants_zero','process_group_empty')}
    def cpu_interval(before,after):
        a,b=raw[before],raw[after];elapsed=b['at']-a['at']
        ticks=(b['utime']+b['stime'])-(a['utime']+a['stime'])
        return dict(elapsed_ms=elapsed/1e6,cpu_ms=ticks*1000/a['hz'],
            cores=ticks*1e9/(a['hz']*elapsed),clock_tick_ms=1000/a['hz'])
    facts=dict(schema=1,clock_domains=['linux_monotonic_ns','windows_qpc','x11_server_ms'],
        clock_projection='Rust measured round-trip bracket plus 100 ppm drift allowance within 10 seconds',
        gui_times='conservative action-to-poll interval; production GUI messages have no event timestamp',
        region_limit='manual comparison started on Play: FX macro region is not a macro-latency witness' if raw.get('adapter')=='human' else 'calibrated FX macro and navigation regions; hashes alone do not establish semantic state',
        x11_times='server timestamps retained privately; public intervals end at Linux observation',
        window_count=len(windows),same_ui_thread=all(w['tid']==windows[0]['tid'] for w in windows),
        renderer_modules=raw['renderer'],renderer_presentation_api='not measured; loaded OpenGL is a path indicator',
        observer_status={k:raw['observer_status'][k] for k in status_fields},frequency=raw['frequency'],
        x11_records=len(raw['x11']),x11_dropped=raw['x11_dropped'],x11_unparsed=raw['x11_unparsed'],
        gui_dropped=raw['gui_dropped'],frame_dropped=raw['frame_dropped'],
        capture=dict(frames=len(raw['frames']),interval_ms=raw['frame_interval_ms'],
            maximum_cpu_ms=max((f['cpu_ns']/1e6 for f in raw['frames']),default=0),
            mean_cpu_ms=sum(f['cpu_ns']/1e6 for f in raw['frames'])/max(1,len(raw['frames'])),
            backend=sorted(set(f['capture_backend'] for f in raw['frames']))),
        cpu_context=dict(measure='whole Windows-process CPU, all threads; not UI-thread CPU',
            trace_off=cpu_interval('process_before','process_trace_off_end'),
            diagnostic_idle_including_helper_startup=cpu_interval('process_trace_off_end','process_diagnostic_idle_end'),
            total_python_cpu_ms=raw['diagnostic_python_cpu_ns']/1e6,
            causal_overhead_estimate=False),
        actions=[],helper_cleanup=cleanup_public)
    for action in starts:
        w=[r for r in raw['win32'] if r['action']==action];g=[r for r in raw['gui'] if r['action']==action];f=[r for r in raw['frames'] if r['action']==action]
        facts['actions'].append(dict(id=action,
            message_counts=[dict(source=s,message=m,count=n) for (s,m),n in sorted(collections.Counter((r['source'],r['message']) for r in w).items())],
            max_heartbeat_ms=max((r['result']*1000/raw['frequency'] for r in w if r['source']==4),default=None),
            mouse_chain_consumed=sum(r['source']==6 and bool(r['result']) for r in w),
            gesture_begin=sum(r['kind']==101 for r in g),gesture_end=sum(r['kind']==103 for r in g),
            host_parameter_values=[r['value'] for r in g if r['kind']==3 and r['parameter']==1],
            regions=[dict(index=i,distinct_hashes=len(set(r['region_hashes'][i] for r in f)),
                first_hash=f[0]['region_hashes'][i] if f else None,
                last_hash=f[-1]['region_hashes'][i] if f else None) for i in range(3)],
            frame_sample_change=dict(maximum_percent=max((r['changed_sample_percent'] for r in f),default=0),
                frames_with_change=sum(bool(r['changed_sample_percent']) for r in f),
                sample_step_pixels=8)))
    return dict(report=report,witnesses=observations),facts

if __name__=='__main__':
    raw=json.loads(pathlib.Path(sys.argv[1]).read_text());projection,facts=project(raw)
    out=pathlib.Path(sys.argv[2]);out.mkdir(exist_ok=True)
    (out/'projection.json').write_text(json.dumps(projection,indent=2)+'\n')
    (out/'facts.json').write_text(json.dumps(facts,indent=2)+'\n')
