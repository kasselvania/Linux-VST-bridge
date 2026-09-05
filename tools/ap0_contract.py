"""Independent AP0 checker. Zero tolerance, fixed recipe, every float32 sample."""
from __future__ import annotations
import math
import struct

MODE = 'ap0-offline-again-processing'
FRAMES = 16
GUARD = 0x4b123456
CALLS = ['set_bus_arrangements','setup_processing','activate_audio_input',
         'activate_audio_output','deactivate_event_input','set_active_true',
         'set_processing_true','set_processing_false','set_active_false',
         'deactivate_audio_output','deactivate_audio_input']

def require(condition, message):
    if not condition:
        raise ValueError('AP0: '+message)

def bits(value):
    return struct.unpack('<I',struct.pack('<f',value))[0]

def sample(word):
    require(type(word) is int and 0<=word<2**32,'invalid float32 word')
    return struct.unpack('<f',struct.pack('<I',word))[0]

def inputs(block, channel):
    # Mathematical recipe independent of host buffers and AGain implementation.
    if block == 2:
        return [0.0]*FRAMES
    return [( ((i+2*block)%9-4) if channel==0 else ((3*i+block+2)%11-5) )/8
            for i in range(FRAMES)]

def compare_blocks(blocks):
    require(type(blocks) is list and len(blocks)==3,'three blocks required')
    errors=[]
    for b,block in enumerate(blocks):
        require(block['block']==b and block['frames']==FRAMES and block['sample_rate']==48000,
                'configuration or block order differs')
        gain=0.5 if b==0 else 0.25
        require(block['gain']==gain and block['process_mode']=='kOffline'
                and block['sample_format']=='kSample32','gain/format differs')
        require(type(block['process_result']) is int and block['process_result']==0
                and block['worker_thread'] is True,'process failed or wrong thread')
        require(block['input_silence_flags']==(3 if b==2 else 0)
                and block['output_silence_flags']==(3 if b==2 else 0),'silence flags differ')
        for key in ('input_bits','output_bits'):
            require(type(block[key]) is list and len(block[key])==2,'two channels required')
            for channel in block[key]:
                require(type(channel) is list and len(channel)==FRAMES+2,'missing/extra samples')
                require(channel[0]==GUARD and channel[-1]==GUARD,'buffer guard overwritten')
        for ch in range(2):
            expected_inputs=inputs(b,ch)
            require(block['input_bits'][ch][1:-1]==[bits(v) for v in expected_inputs],
                    'input changed or recipe differs')
            for i,value in enumerate(block['output_bits'][ch][1:-1]):
                actual=sample(value)
                require(math.isfinite(actual),f'nonfinite/unwritten output block {b} channel {ch} sample {i}')
                expected=expected_inputs[i]*gain
                error=abs(actual-expected);errors.append(error)
                require(error==0.0,f'sample mismatch block {b} channel {ch} sample {i}; max_error={max(errors)}')
    return {'comparison_rule':'exact numerical equality; tolerance 0',
            'samples_compared':len(errors),'maximum_absolute_error':max(errors)}

def lifecycle_result_ok(operation, result):
    # Exact pinned Windows SDK kNotImplemented; never accept it for process,
    # activation or setup. The raw notification result remains in evidence.
    return type(result) is int and (result == 0 or (
        operation in {'set_processing_true', 'set_processing_false'}
        and result == -2147467263))

def validate_lifecycle(observed, expected_blocks=3):
    records=observed['records']
    require(observed['raw_exit']==0 and observed['classification']=='scanner_completed','scanner did not complete')
    require(observed['cleanup']=={'owned_descendants_zero':True,'process_group_empty':True},'containment incomplete')
    done=[r for r in records if r.get('state')=='scanner_completed']
    require(len(done)==1,'missing final scanner record')
    calls=[r for r in records if r.get('state') in {'ap0_call_started','ap0_call_completed'}]
    require([r.get('operation') for r in calls]==[v for op in CALLS for v in (op,op)],'processing call order differs')
    for i,r in enumerate(calls):
        require(r['state']==('ap0_call_started' if i%2==0 else 'ap0_call_completed'),'unpaired call')
        if i%2:
            require(lifecycle_result_ok(r['operation'], r['result']),'processing lifecycle call failed')
        else:
            require(r['owner_thread'] is (r['operation'] not in {'set_processing_true','set_processing_false'}),'lifecycle thread differs')
    joined=[r for r in records if r.get('state')=='ap0_thread_joined']
    threads=[r for r in records if r.get('state')=='ap0_processing_thread_started']
    require(len(joined)==len(threads)==1 and threads[0]['distinct_from_owner'] is True
            and joined[0]['joined'] is True and joined[0]['processing_stopped'] is True
            and joined[0]['worker_exception'] is False,'processing thread not stopped/joined')
    require(calls[15]['sequence'] < joined[0]['sequence'] < calls[16]['sequence'],'deactivated before stop/join')
    process=[r for r in records if r.get('state') in {'ap0_process_started','ap0_process_completed'}]
    require([(r['state'],r['block']) for r in process]==
            [(state,b) for b in range(expected_blocks) for state in ('ap0_process_started','ap0_process_completed')], 'process calls differ')
    require(calls[13]['sequence']<process[0]['sequence']<process[-1]['sequence']<calls[14]['sequence'],'process outside Processing state')
    require(all(r['result']==0 for r in process[1::2]),'process returned failure')
    component=done[0]['component_session'];lease=component['audio_processor_lease']
    require(component['object_quiescence'] is True and lease['audio_interface_quiescence'] is True
            and component['callbacks']['wrong_thread'] is False,'component/interface/callback quiescence failed')
    shutdown=observed['inherited_shutdown']
    require(shutdown['clean_in_process_shutdown'] is True and shutdown['physical_containment_only'] is False,'shutdown incomplete')
    from pc0_contract import pc0_validate_contract, pc0_validate_call_facts, PC0_OPERATIONS
    ledger=[{**r, **{key:r.get(key) for key in ('interface','ordinal','tier')}}
            for r in records if r.get('event') in {'call_started','call_completed'}]
    facts={'ledger':ledger,'pc0_operation_counts':{op:sum(r.get('operation')==op and r.get('event')=='call_started' for r in ledger) for op in PC0_OPERATIONS}}
    pc0_validate_call_facts(facts)
    pc0_validate_contract(component['processing_contract'])
    return {'run_id':observed['run_id'],'call_facts':facts,
            'lifecycle':[r for r in records if str(r.get('state','')).startswith('ap0_') and r.get('state')!='ap0_samples'],
            'component_session':component,'shutdown':shutdown,'cleanup':observed['cleanup'],
            'raw_exit':observed['raw_exit'],'stdout_sha256':observed['stdout_sha256'],
            'stderr_sha256':observed['stderr_sha256']}

def normalize(observed):
    summary=validate_lifecycle(observed)
    blocks=[r for r in observed['records'] if r.get('state')=='ap0_samples']
    summary.update(blocks=blocks,comparison=compare_blocks(blocks))
    return summary

def validate_summary(summary):
    require(summary['comparison']==compare_blocks(summary['blocks']),'comparison differs')
    require(summary['stage_absent'] is True and summary['protected_before_sha256']==summary['protected_after_sha256'],
            'retirement/protected state differs')
    # Re-run lifecycle/ownership admission on retained data, not a saved PASS flag.
    final={'state':'scanner_completed','component_session':summary['component_session']}
    observed={'records':sorted(summary['call_facts']['ledger']+summary['lifecycle']+summary['blocks'],key=lambda r:r['sequence'])+[final],
              'run_id':summary['run_id'],'raw_exit':summary['raw_exit'],'classification':'scanner_completed',
              'cleanup':summary['cleanup'],'inherited_shutdown':summary['shutdown'],
              'stdout_sha256':summary['stdout_sha256'],'stderr_sha256':summary['stderr_sha256']}
    normalize(observed)
    return summary
