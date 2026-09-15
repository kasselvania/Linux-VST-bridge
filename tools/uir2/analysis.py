"""UIR2 bounded offline interpretation. Timestamps and queue hints are not admission."""
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent.parent/'uio3'))
from timeline import projected

INSUFFICIENT='UIR2_INSUFFICIENT_PHYSICAL_COVERAGE'

def delta32(later,earlier):
    if any(type(n) is not int or not 0<=n<2**32 for n in (later,earlier)):
        raise ValueError('32-bit timestamp required')
    d=(later-earlier)&0xffffffff
    if d==0x80000000:raise ValueError('ambiguous half-wrap')
    return d if d<0x80000000 else d-2**32

def unfold32(value,anchor):
    """Nearest anchor epoch only; caller must prove separation < half wrap."""
    if type(anchor) is not int or anchor<0:raise ValueError('timestamp anchor')
    return anchor+delta32(value,anchor&0xffffffff)

def queue_words(value):
    if type(value) is not int or not 0<=value<2**32:raise ValueError('queue word')
    return dict(changed=value&65535,current=value>>16,admission_proved=False)

def classify(f):
    """Closed evidence decisions; a queue bit alone cannot select A or B.

    Future exact availability evidence must identify the particular release.
    Repeated absent samples cannot prove absence between samples. A late source
    timestamp is not a measured Wine message-creation time without that proof.
    """
    if (type(f.get('contacts')) is not int or f.get('contacts')!=1 or f.get('coverage_complete') is not True or
        f.get('clock_validated') is not True or f.get('procedure_after_removal') is not True):
        return INSUFFICIENT
    if f.get('removal_upper_ms',float('inf'))<=100 and f.get('procedure_gap_upper_ms',float('inf'))<=20:
        return 'UIR2_EMBEDDED_PRODUCT_ROUTE_REQUIRED'
    if (f.get('exact_release_available') is True and f.get('available_upper_ms',float('inf'))<=100 and
        f.get('removal_lower_ms',0)>500 and f.get('procedure_gap_upper_ms',float('inf'))<=20):
        return 'UIR2_HOST_POINTER_RETRIEVAL_DELAY'
    if (f.get('creation_clock_validated') is True and f.get('creation_lower_ms',0)>500 and
        f.get('exact_release_absent_until_creation') is True and f.get('pump_live') is True):
        return 'UIR2_WINE_XWAYLAND_ADMISSION_DELAY'
    return INSUFFICIENT

def offline(raw):
    if raw.get('schema')!=1 or len(raw.get('win32',[]))>16384:raise ValueError('retained timeline schema/bound')
    linux=[r for r in raw['x11'] if r['kind']=='core_up' or r['kind']=='xi_touch_end']
    records=[]
    for r in raw['win32']:
        release=r['message'] in (0x202,0x247) or (r['source']==12 and r['message']==0x240 and r['buttons']&4)
        if not release:continue
        records.append(dict(r,monotonic_interval=projected(r['qpc'],raw['brackets']),
             timestamp_status='retained' if r['source']==1 else 'not_a_MSG_time'))
    # No cross-contact association or direct cross-clock subtraction.
    return dict(schema=1,linux=linux,windows=records,
        conclusion='phase_a_cannot_select_boundary',
        reasons=['No GetQueueStatus or release-specific availability observation.',
                 'No independent X-server/MSG.time clock calibration in retained source-owned records.',
                 'Three contacts cannot be paired across systems by count or identifier equality.',
                 'An old MSG.time may preserve source-event time; it is not by itself observed queue admission.'])

if __name__=='__main__':
    import os
    os.umask(0o077)
    data=Path(sys.argv[1]).read_bytes()
    if len(data)>32*1024*1024:raise SystemExit('private timeline bound')
    result=offline(json.loads(data))
    with Path(sys.argv[2]).open('x') as f:json.dump(result,f,indent=2)
    print(json.dumps({'schema':1,'conclusion':result['conclusion'],'linux_release_records':len(result['linux']),
                      'windows_release_records':len(result['windows']),'reasons':result['reasons']}))
