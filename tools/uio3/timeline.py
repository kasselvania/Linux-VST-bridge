"""Scalar-only release attribution; missing observations are never root causes."""

SOURCE={1:'queue_retrieval',2:'sent_call_hook',3:'sent_return_hook',4:'heartbeat',5:'mouse_hook_entry',6:'mouse_hook_result',7:'mouse_hook_peek',8:'mouse_hook_peek_result',11:'pointer_detail',12:'touch_detail',13:'getmessage_downstream',14:'wndproc_entry',15:'wndproc_return'}
UP={0x202,0x247};DOWN={0x201,0x246}

def projected(qpc,brackets):
    if not brackets:return None
    b=min(brackets,key=lambda b:abs(qpc-b['windows_qpc'])/b['frequency'])
    delta=(qpc-b['windows_qpc'])*1_000_000_000//b['frequency']
    if abs(delta)>10_000_000_000:return None
    drift=abs(delta)//10000 # 100 ppm conservative allowance, as UIO1
    return [b['linux_before_ns']+delta-drift,b['linux_after_ns']+delta+drift]


def summarize(raw):
    """Reports observation boundaries only. Payloads/raw IDs are never projected."""
    if raw.get('schema')!=1:raise ValueError('UIO3 timeline schema')
    actions=[]
    for action in (1,2):
        if action not in [x['action'] for x in raw['actions']]:continue
        x=[r for r in raw['x11'] if r['action']==action]
        native=[r for r in raw['raw'] if r['action']==action]
        win=[r for r in raw['win32'] if r['action']==action]
        gui=[r for r in raw['gui'] if r['action']==action]
        up=[r for r in x if r['kind']=='core_up']
        end=[r for r in native if r['kind']=='raw_touch_end']+[r for r in x if r['kind']=='xi_touch_end']
        begin=[r for r in gui if r['kind']==101]
        params={r['parameter'] for r in begin}
        if len(params)>1:boundary='multiple_control_gestures';parameter=None
        else:boundary=None;parameter=next(iter(params),None)
        gestures=[r for r in gui if r['parameter']==parameter and r['kind'] in (101,102,103)]
        release_observed=max([r['observed_ns'] for r in end+up],default=None)
        # A pointer-up route does not require emulated core mouse release.
        touch_up=[r for r in win if r['source']==12 and r['message']==0x240 and r.get('key_class')==1 and r.get('buttons',0)&4]
        entry=[r for r in win if r['source']==14 and r['message'] in UP]+touch_up
        ret=[r for r in win if r['source']==15 and (r['message'] in UP or (touch_up and r['message']==0x240 and r['qpc']>=touch_up[0]['qpc']))]
        hooked=[r for r in win if r['source']==5 and r['message']==0x202]
        swallowed=any(r['source']==6 and r['message']==0x202 and r['result']!=0 for r in win)
        rewritten=any(r['source']==13 and r['message'] in UP and r['result']==1 for r in win)
        pending=0
        for r in gestures:
            if r['kind']==101:pending+=1
            elif r['kind']==103:pending=max(0,pending-1)
        ended=bool(begin) and pending==0 and any(r['kind']==103 for r in gestures)
        values_after=[r for r in gestures if r['kind']==102 and release_observed is not None and r.get('poll_before_ns',0)>release_observed]
        captures_after=[]
        for r in win:
            interval=projected(r['qpc'],raw['brackets'])
            if release_observed is not None and interval and interval[0]>release_observed:captures_after.append(r['capture']!=0)
        heartbeat=[r for r in win if r['source']==4]
        frequency=raw.get('frequency',0)
        latency=max((r['result']*1000/frequency for r in heartbeat),default=None) if frequency else None
        if boundary is None:
            if raw.get('terminal'):boundary='terminal_instance_failure'
            elif any(raw.get('drops',{}).values()):boundary='incomplete_observation_capacity'
            elif not end and not up and not entry:boundary='release_not_observed'
            elif swallowed:boundary='downstream_mouse_hook_nonzero'
            elif rewritten:boundary='retrieved_release_rewritten_to_null'
            elif entry:
                if not ret:boundary='wndproc_release_return_unobserved'
                elif begin and not ended:
                    boundary='capture_clear_gesture_end_unobserved' if captures_after and not any(captures_after[-3:]) else 'wndproc_release_gesture_end_unobserved'
                elif ret and ret[-1]['capture']!=0:boundary='wndproc_release_capture_still_present'
                else:boundary='release_and_gesture_end_observed' if ended else 'release_delivered_no_parameter_gesture'
            elif up:boundary='core_release_without_win32_dispatch'
            elif end:boundary='xi_touch_end_without_core_or_win32_release'
            else:boundary='insufficient_observation'
        # Delay bounds use latest X11 observation and earliest Win32 observation.
        lower=None
        if up and hooked:
            intervals=[projected(r['qpc'],raw['brackets']) for r in hooked];intervals=[r for r in intervals if r]
            if intervals:lower=max(0,min(r[0] for r in intervals)-max(r['observed_ns'] for r in up))/1e6
        # Multiple touch contacts must not be paired by an unordered first/last
        # shortcut or by equating Linux and Windows identifier namespaces.
        # Retain a directly observed tail after the entire X11 release sequence.
        tails=[]
        if release_observed is not None:
            for r in entry:
                interval=projected(r['qpc'],raw['brackets'])
                if interval and interval[0]>release_observed:
                    tails.append((interval[0]-release_observed)/1e6)
        actions.append(dict(action=action,condition='without_held_note' if action==1 else 'with_held_note',
            condition_basis='operator_action_label_not_MIDI_measurement',boundary=boundary,
            core_release_count=len(up),windows_pointer_up_count=sum(r['source']==14 and r['message']==0x247 for r in win),
            windows_touch_up_count=len(touch_up),gesture_begin_count=len(begin),gesture_open_count=pending,
            windows_release_tail_after_last_x11_release_lower_ms=max(tails,default=None),
            release_tail_basis='observed_after_all_X11_releases_not_cross_namespace_contact_pairing',
            xi_touch_end_observed=bool(end),core_release_observed=bool(up),win32_release_entry=bool(entry),win32_release_return=bool(ret),
            pointer_up_observed=any(r['message']==0x247 for r in entry),touch_up_observed=bool(touch_up),gesture_parameter=parameter,
            gesture_sequence=[{101:'begin',102:'value',103:'end'}[r['kind']] for r in gestures],
            parameter_values_observed_after_release=len(values_after),capture_present_after_release=captures_after[-8:],
            x11_to_win32_retrieval_lower_ms=lower,heartbeat_max_ms=latency,
            local_changed_frames=sum(r['action']==action and r['changed_sample_percent']>0 for r in raw['frames'])))
    return dict(schema=1,product_profile=raw['profile_fingerprint'],actions=actions,
        cleanup={k:raw.get('cleanup',{}).get('helper',{}).get('cleanup',{}).get(k) for k in ('owned_descendants_zero','process_group_empty')},drops=raw.get('drops',{}),completed=raw.get('completed',False),
        classification='observed_boundary_not_root_cause',observation_stop=(raw.get('stop','operator_stop') if raw.get('stop','operator_stop') in ('operator_stop','first_useful_release_boundary','cancellation_or_time_bound') else 'observer_error_details_private'),
        limitations=['Raw XI2 events have no delivery target; associated pointer state is a later query.',
          'XWayland may not expose physical touch as XI2 touch events.',
          'Multiple contacts are not a controlled single-drag comparison; identifier equality alone does not prove cross-system contact identity.',
          'Capture still present alone is not proof of a defect.',
          'Whole-window pixel change does not identify the macro or prove rendering latency.',
          'Parameter observation time is a bounded poll interval, not vendor emission time.',
          'Normal cleanup does not establish historical symptom fixed.'])


def readable(summary):
    rows=['UIO3 touch-release observation (not a root-cause finding)']
    for a in summary['actions']:
        rows.extend([f"Action {a['action']} ({a['condition']}): {a['boundary']}",
                     f"  XI touch end: {a['xi_touch_end_observed']}; core release: {a['core_release_observed']}; Win32 procedure entry: {a['win32_release_entry']}",
                     '  Gesture: '+' -> '.join(a['gesture_sequence']),
                     f"  Values observed after release: {a['parameter_values_observed_after_release']}"])
    rows.append('Unavailable or missing observations do not establish the internal cause.')
    return '\n'.join(rows)+'\n'
