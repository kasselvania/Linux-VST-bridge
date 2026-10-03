"""Conservative attribution from one retained request and kernel switch window.

No whole-session QPC/monotonic offset is assumed. The request publication and
reply receipt bracket this exchange's clock offset. Missing scheduler coverage
is an error, never evidence that the render thread stayed on CPU.
"""
import json
import sys


def attribute(witness):
    linux=witness['request_monotonic_ns'];windows=witness['windows_reply']
    events=witness['switches'];uncertainty=witness['clock_uncertainty_ns']
    if (len(linux)!=7 or len(windows)!=15 or windows[14]<=0 or uncertainty<0
            or witness['lost_events'] or len(events)<2):
        raise ValueError('incomplete timing witness')
    if linux!=sorted(linux) or windows[:8]!=sorted(windows[:8]):
        raise ValueError('unordered request')
    floor=lambda tick:tick*1_000_000_000//windows[14]
    ceil=lambda tick:(tick*1_000_000_000+windows[14]-1)//windows[14]
    # prepared precedes publication; received follows receive/decode.
    # Windows reply publication precedes the Linux reply observation.
    low=linux[2]-ceil(windows[1])-uncertainty
    high=linux[4]-floor(windows[7])+uncertainty
    if low>high:raise ValueError('inconsistent clock brackets')
    possible=[floor(windows[3])+low,ceil(windows[4])+high]
    common=[ceil(windows[3])+high,floor(windows[4])+low]
    if events[0][0]>possible[0] or events[-1][0]<possible[1]:
        raise ValueError('scheduler does not cover this SDK call')
    spans=[];pending=None;previous=None
    for index,(stamp,direction,preempted) in enumerate(events):
        if previous is not None and stamp<previous:raise ValueError('unordered switches')
        previous=stamp
        if direction=='OUT':
            if pending is not None:raise ValueError('missing switch in')
            pending=(stamp,preempted)
        elif direction=='IN':
            if pending is None:
                if index:raise ValueError('missing switch out')
            else:
                spans.append([pending[0],stamp,pending[1]]);pending=None
        else:raise ValueError('unknown switch direction')
    def overlap(window,only_preempted):
        return sum(max(0,min(end,window[1])-max(begin,window[0]))
            for begin,end,preempted in spans if not only_preempted or preempted)
    gap=witness['gap_monotonic_ns']
    across=[span for span in spans if span[2]
        and common[0]<=span[0]<gap-uncertainty
        and gap+uncertainty<span[1]<=common[1]]
    return {'clock_offset_bounds_ns':[low,high],'sdk_possible_interval_ns':possible,
        'sdk_common_interval_ns':common,
        'off_cpu_lower_bound_ns':overlap(common,False),
        'preemption_lower_bound_ns':overlap(common,True),
        'preemption_upper_bound_ns':min(overlap(possible,True),ceil(windows[4])-floor(windows[3])),
        'preemptions_inside_sdk_across_deadline':across}


if __name__=='__main__':
    with open(sys.argv[1]) as source:witness=json.load(source)
    print(json.dumps(attribute(witness),indent=2))
