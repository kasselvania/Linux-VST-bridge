"""Independent admission of Linux-read mapped sample words and host lifecycle."""
import struct
from ap0_contract import require,validate_lifecycle
MODE='ap1-linux-windows-audio-roundtrip'
LENGTHS=[1,16,63,256,16,63,1,256,16,63]
GAINS=[.5,.25,.75,.5,.75,.25,.5,.75,0.,.5]
SILENCE=[0,0,0,0,0,3,0,0,0,1]
GUARD=0x4b123456;POISON=0x7fc12345;MASK=(1<<64)-1

def input_words(seed,b,ch):
    x=seed^(((b+1)*0x9e3779b97f4a7c15)&MASK)^((ch+1)<<48)
    result=[GUARD]+[0]*256+[GUARD]
    for i in range(LENGTHS[b]):
        x=(x^(x<<13))&MASK;x^=x>>7;x=(x^(x<<17))&MASK
        value=0. if SILENCE[b] & (1<<ch) else (x%129-64)/128
        result[i+1]=struct.unpack('<I',struct.pack('<f',value))[0]
    if b==8:result[1]=struct.unpack("<I",struct.pack("<f",(ch+1)/4))[0]
    if b==9 and ch==1:result[1]=struct.unpack("<I",struct.pack("<f",-.5))[0]
    return result

def sample(word):
    require(type(word) is int and 0<=word<=0xffffffff,'AP1 float32 word')
    return struct.unpack('<f',struct.pack('<I',word))[0]

def compare_caller(caller):
    require(caller['raw_exit']==0 and caller['cleanup']=={'owned_descendants_zero':True,'process_group_empty':True},'native caller exit/containment')
    records=caller['records'];require(len(records)==12,'native record count')
    ready=records[0];require(ready['event']=='ap1_client_ready' and ready['seed_chosen_after_ready'] is True and ready['mapping_witness'] is True and ready['mapping_count']==ready['connection_count']==1,'native Ready/mapping')
    seed=ready['seed'];require(type(seed) is int and 0<=seed<=MASK,'native seed')
    errors=[]
    for b,record in enumerate(records[1:11]):
        require(record['event']=='ap1_client_block' and record['sequence']==b+1 and record['frames']==LENGTHS[b] and record['gain']==GAINS[b] and record['silent'] is (SILENCE[b]==3) and type(record['input_silence_flags']) is int and record['input_silence_flags']==SILENCE[b],'native request correlation')
        require(record['input_bits']==[input_words(seed,b,ch) for ch in range(2)],'Linux request/input/guard differs')
        flags=record['output_silence_flags'];require(type(flags) is int and 0<=flags<=3,'invalid output silence bits')
        out=record['output_bits'];require(type(out) is list and len(out)==2,'output channels')
        for ch in range(2):
            words=out[ch];require(type(words) is list and len(words)==258 and words[0]==words[-1]==GUARD,'mapped guard/extent')
            require(words[LENGTHS[b]+1:-1]==[POISON]*(256-LENGTHS[b]),'unused mapped output changed')
            for i in range(1,LENGTHS[b]+1):
                actual=sample(words[i]);expected=sample(record['input_bits'][ch][i])*GAINS[b]
                require(not (flags & (1<<ch)) or actual==0.,'output silence claim has nonzero sample')
                error=abs(actual-expected);require(error==0.,f'AP1 sample mismatch sequence {b+1} channel {ch} sample {i-1}');errors.append(error)
        require(record['maximum_absolute_error']==0.,'native comparison differs')
    closed=records[-1];require(closed['event']=='ap1_client_closed' and closed['blocks']==10 and closed['samples_compared']==len(errors) and closed['maximum_absolute_error']==0. and closed['closed_received'] is True and closed['mapping_unmapped'] is True and closed['replays']==0,'native close/comparison')
    return {'samples_compared':len(errors),'blocks':10,'maximum_absolute_error':max(errors),'comparison_rule':'independent Linux request recipe; exact equality; tolerance zero'}

def normalize(observed):
    summary=validate_lifecycle(observed,expected_blocks=10)
    caller=observed['caller'];comparison=compare_caller(caller)
    transport=[r for r in observed['records'] if str(r.get('state','')).startswith('ap1_')]
    mapping=[r for r in transport if r['state']=='ap1_mapping_ready'];closed=[r for r in transport if r['state']=='ap1_endpoint_closed'];buffers=[r for r in transport if r['state']=='ap1_private_buffers_valid']
    require(len(mapping)==len(closed)==1 and [r['block'] for r in buffers]==list(range(10)),'Windows mapping/close/private buffers')
    require(mapping[0]['mapping_count']==mapping[0]['connection_count']==1 and mapping[0]['mapping_witness'] is True and closed[0]['mapping_unmapped'] is True and closed[0]['instance_count']==1,'Windows session reuse')
    masks=[r for r in transport if r['state']=='ap1_output_silence']
    require([(r['block'],r['input_silence_flags'],r['output_silence_flags']) for r in masks]==[(b,r['input_silence_flags'],r['output_silence_flags']) for b,r in enumerate(caller['records'][1:11])],'native/returned silence flags differ')
    ledger=summary['call_facts']['ledger']
    load=next(r['sequence'] for r in ledger if r['operation']=='load_library' and r['event']=='call_started')
    unload=next(r['sequence'] for r in ledger if r['operation']=='free_library' and r['event']=='call_completed')
    require(mapping[0]['sequence']<load and closed[0]['sequence']>unload,'mapping/Closed lifecycle order')
    summary.update(caller=caller,comparison=comparison,transport_lifecycle=transport)
    return summary

def validate_summary(summary):
    require(summary['stage_absent'] is True and summary['protected_before_sha256']==summary['protected_after_sha256'],'AP1 retirement/protected state')
    final={'state':'scanner_completed','component_session':summary['component_session']}
    observed={'records':sorted(summary['call_facts']['ledger']+summary['lifecycle']+summary['transport_lifecycle'],key=lambda r:r['sequence'])+[final],
              'run_id':summary['run_id'],'raw_exit':summary['raw_exit'],'classification':'scanner_completed',
              'cleanup':summary['cleanup'],'inherited_shutdown':summary['shutdown'],
              'stdout_sha256':summary['stdout_sha256'],'stderr_sha256':summary['stderr_sha256'],'caller':summary['caller']}
    require(normalize(observed)['comparison']==summary['comparison'],'AP1 retained comparison differs')
    return summary
