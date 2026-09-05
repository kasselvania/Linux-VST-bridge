"""AP3 bounded record admission; independently reconstruct the host timeline."""
import struct,math
from functools import lru_cache
from ap0_contract import require,lifecycle_result_ok,CALLS
from pc0_contract import pc0_validate_contract,pc0_validate_call_facts,PC0_OPERATIONS
MODE='ap3-queued-audio-preview'
MASK=(1<<64)-1
CLEAN={'owned_descendants_zero':True,'process_group_empty':True}

def fold(h,value):
 word=struct.pack('<f',0. if value==0. else value)
 for byte in word:h=((h^byte)*1099511628211)&MASK
 return h

@lru_cache(maxsize=8)
def reconstruct(seed,n,persistence):
 require(type(seed)is int and 0<=seed<=MASK,'AP3 host seed')
 x=seed;gain=.25 if persistence else 1.;position=0;input_hash=output_hash=14695981039346656037
 reference=[[0.]*4096 for _ in range(2)];zero_gain=one_silent=inplace=flushes=0
 # Input generation order and output consumption order are deliberately
 # different; this consumer reconstructs both from the retained host seed.
 for b in range((1440000+1024)//n):
  update=b%5!=1 and b!=0
  if update:gain=(.5,.25,0.,.75)[(b//5)%4]
  if b%19==18:gain=.25;update=False;flushes+=1
  flags=3 if position>=1440000 or b%17==8 else 1 if b%13==4 else 0
  for i in range(n):
   for ch in range(2):
    x=(x^(x<<13))&MASK;x^=x>>7;x=(x^(x<<17))&MASK
    value=0. if flags&(1<<ch) else (x%65-32)/256.
    reference[ch][(position+i)%4096]=value*gain;input_hash=fold(input_hash,value)
  for ch in range(2):
   for i in range(n):output_hash=fold(output_hash,0. if position+i<1024 else reference[ch][(position+i-1024)%4096])
  zero_gain+=gain==0. and flags==0;one_silent+=flags==1;inplace+=b%7==3;position+=n
 return dict(input_fnv1a64=input_hash,output_fnv1a64=output_hash,zero_gain_blocks=zero_gain,one_silent_blocks=one_silent,inplace_blocks=inplace,zero_frame_flushes=flushes)

def compare(records):
 ready=[r for r in records if r['event']=='ap3_host_ready'];done=[r for r in records if r['event']=='ap3_host_compared']
 require(len(ready)==len(done)==2,'AP3 two paced streams')
 for index,(r,d,n) in enumerate(zip(ready,done,(128,256))):
  require(r['seed_after_activation'] is True,'AP3 input selection before activation')
  require(d['frames_per_callback']==n and d['active_frames']==1440000 and d['samples']==2882048 and d['latency_samples']==1024,'AP3 measured extent/delay')
  require(d['max_error']==0. and d['callback_overruns']==0 and d['callback_effects']==0 and d['fault_observed'] is False,'AP3 numerical/deadline/callback failure')
  require(0<=d['callback_median_ns']<=d['callback_p99_ns']<=d['callback_max_ns']<=n*1000000000//48000,'AP3 timing bounds')
  expected=reconstruct(r['seed'],n,index==1)
  require(all(d[k]==v for k,v in expected.items()),'AP3 independent input/output timeline digest differs')
 stats=[r for r in records if r['event']=='ap3_proxy_stats'];closed=[r for r in records if r['event']=='ap3_host_closed']
 require(len(stats)==len(closed)==1,'AP3 one native session')
 s=stats[0];require(s['fault']==0 and s['processed']==16887 and s['position']==1441024 and s['epoch']==2 and 1<=s['request_high']<=2048 and 1<=s['result_high']<=2048,'AP3 queue ownership/count/fault')
 require(closed[0]['terminate_result']==0 and closed[0]['module_unloaded'] is True and closed[0]['references_released'] is True,'AP3 native unload')
 return dict(samples_compared=5764096,maximum_absolute_error=0.,latency_samples=1024,streams=done,queue=s)

def normalize(observed):
 caller=observed['caller'];require(caller['raw_exit']==0 and caller['cleanup']==CLEAN,'AP3 caller containment')
 comparison=compare(caller['records']);require(observed['raw_exit']==0 and observed['classification']=='scanner_completed' and observed['cleanup']==CLEAN,'AP3 Windows containment')
 records=observed['records'];terminal=[r for r in records if r.get('state')=='scanner_completed'];require(len(terminal)==1,'AP3 terminal')
 component=terminal[0]['component_session'];pc0_validate_contract(component['processing_contract'])
 require(component['object_quiescence'] is True and component['audio_processor_lease']['audio_interface_quiescence'] is True and component['callbacks']['wrong_thread'] is False,'AP3 component/interface quiescence')
 shutdown=observed['inherited_shutdown'];require(shutdown['clean_in_process_shutdown'] is True and shutdown['physical_containment_only'] is False,'AP3 ordinary shutdown')
 ledger=[{**r,**{k:r.get(k) for k in ('interface','ordinal','tier')}} for r in records if r.get('event') in {'call_started','call_completed'}]
 facts={'ledger':ledger,'pc0_operation_counts':{op:sum(r.get('operation')==op and r['event']=='call_started' for r in ledger) for op in PC0_OPERATIONS}};pc0_validate_call_facts(facts)
 calls=[r for r in records if r.get('state') in {'ap0_call_started','ap0_call_completed'}]
 expected=CALLS[:6]+CALLS[6:8]*2+CALLS[8:]
 require([r.get('operation') for r in calls]==[v for op in expected for v in (op,op)],'AP3 Windows lifecycle order')
 for i,r in enumerate(calls):
  require(r['state']==('ap0_call_started' if i%2==0 else 'ap0_call_completed'),'AP3 call pairing')
  if i%2:require(lifecycle_result_ok(r['operation'],r['result']),'AP3 lifecycle result')
  else:require(r['owner_thread'] is (r['operation'] not in CALLS[6:8]),'AP3 lifecycle thread')
 joins=[r for r in records if r.get('state')=='ap0_thread_joined'];threads=[r for r in records if r.get('state')=='ap0_processing_thread_started'];summary=[r for r in records if r.get('state')=='ap3_processing_summary']
 require(len(joins)==len(threads)==2 and all(r['joined'] is True and r['processing_stopped'] is True and r['worker_exception'] is False for r in joins) and all(r['distinct_from_owner'] is True for r in threads),'AP3 processing join/thread')
 require(len(summary)==1 and summary[0]['processed_blocks']==16887 and summary[0]['intervals']==2 and summary[0]['process_mode']=='kRealtime','AP3 actual realtime Windows processing')
 mapping=[r for r in records if r.get('state')=='ap1_mapping_ready'];closed=[r for r in records if r.get('state')=='ap1_endpoint_closed'];ack=[r for r in records if r.get('state')=='ap2_lifecycle_ack']
 require(len(mapping)==len(closed)==1 and mapping[0]['mapping_count']==mapping[0]['connection_count']==1 and mapping[0]['mapping_witness'] is True and closed[0]['mapping_unmapped'] is True,'AP3 mapping reuse/retirement')
 require([(r['kind'],r['next_sequence']) for r in ack]==[(9,1),(11,1),(13,11259),(11,11259),(13,16888),(15,16888)],'AP3 lifecycle sequence acknowledgements')
 require(joins[0]['sequence']<ack[2]['sequence']<ack[3]['sequence'] and joins[1]['sequence']<ack[4]['sequence']<ack[5]['sequence'],'AP3 restart before quiescence')
 require(closed[0]['sequence']>next(r['sequence'] for r in ledger if r.get('operation')=='free_library' and r['event']=='call_completed'),'AP3 mapping closed before plugin unload')
 return dict(raw=public_observation(observed,ledger,terminal),comparison=comparison,cleanup=observed['cleanup'])

def public_observation(observed,ledger,terminal):
 # Public protocol facts only; no topology, runtime paths or private launch data.
 caller=observed['caller'];records=observed['records']
 retained={k:observed[k] for k in ('raw_exit','classification','cleanup','run_id','inherited_shutdown','stdout_sha256','stderr_sha256')}
 retained['records']=sorted(ledger+[r for r in records if str(r.get('state','')).startswith(('ap0_','ap1_','ap2_','ap3_'))]+[terminal[0]],key=lambda r:r['sequence'])
 retained['caller']={k:caller[k] for k in ('raw_exit','cleanup','records')}
 return retained

def validate_summary(summary):
 require(summary['stage_absent'] is True and summary['protected_before_sha256']==summary['protected_after_sha256'],'AP3 stage/protected state')
 result=normalize(summary['raw']);require(result['comparison']==summary['comparison'],'AP3 retained comparison differs');return summary
