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
def reconstruct(seed,n,persistence,active_frames=1440000):
 require(type(seed)is int and 0<=seed<=MASK,'AP3 host seed')
 x=seed;gain=.25 if persistence else 1.;position=0;input_hash=output_hash=14695981039346656037
 reference=[[0.]*4096 for _ in range(2)];zero_gain=one_silent=inplace=flushes=0
 # Input generation order and output consumption order are deliberately
 # different; this consumer reconstructs both from the retained host seed.
 b=0
 while position<active_frames+1024:
  length=min(n,active_frames+1024-position)
  if position<active_frames:length=min(length,active_frames-position)
  update=b%5!=1 and b!=0
  if update:gain=(.5,.25,0.,.75)[(b//5)%4]
  if b%19==18:gain=.25;update=False;flushes+=1
  flags=3 if position>=active_frames or b%17==8 else 1 if b%13==4 else 0
  for i in range(length):
   for ch in range(2):
    x=(x^(x<<13))&MASK;x^=x>>7;x=(x^(x<<17))&MASK
    value=0. if flags&(1<<ch) else (x%65-32)/256.
    reference[ch][(position+i)%4096]=value*gain;input_hash=fold(input_hash,value)
  for ch in range(2):
   for i in range(length):output_hash=fold(output_hash,0. if position+i<1024 else reference[ch][(position+i-1024)%4096])
  zero_gain+=gain==0. and flags==0;one_silent+=flags==1;inplace+=b%7==3;position+=length;b+=1
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

def normalize_session(observed,variant="core"):
 caller=observed['caller'];require(caller['raw_exit']==0 and caller['cleanup']==CLEAN,'AP3 caller containment')
 comparison=(compare(caller['records']) if variant=='core' else compare_short(caller['records']) if variant=='stream_active' else compare_gui(caller['records'],variant));require(observed['raw_exit']==0 and observed['classification']=='scanner_completed' and observed['cleanup']==CLEAN,'AP3 Windows containment')
 records=observed['records'];terminal=[r for r in records if r.get('state')=='scanner_completed'];require(len(terminal)==1,'AP3 terminal')
 component=terminal[0]['component_session'];pc0_validate_contract(component['processing_contract'])
 require(component['object_quiescence'] is True and component['audio_processor_lease']['audio_interface_quiescence'] is True and component['callbacks']['wrong_thread'] is False,'AP3 component/interface quiescence')
 shutdown=observed['inherited_shutdown'];require(shutdown['clean_in_process_shutdown'] is True and shutdown['physical_containment_only'] is False,'AP3 ordinary shutdown')
 ledger=[{**r,**{k:r.get(k) for k in ('interface','ordinal','tier')}} for r in records if r.get('event') in {'call_started','call_completed'}]
 facts={'ledger':ledger,'pc0_operation_counts':{op:sum(r.get('operation')==op and r['event']=='call_started' for r in ledger) for op in PC0_OPERATIONS}};pc0_validate_call_facts(facts)
 calls=[r for r in records if r.get('state') in {'ap0_call_started','ap0_call_completed'}]
 intervals=comparison.get("intervals",2)
 expected=CALLS[:6]+CALLS[6:8]*intervals+CALLS[8:]
 require([r.get('operation') for r in calls]==[v for op in expected for v in (op,op)],'AP3 Windows lifecycle order')
 for i,r in enumerate(calls):
  require(r['state']==('ap0_call_started' if i%2==0 else 'ap0_call_completed'),'AP3 call pairing')
  if i%2:require(lifecycle_result_ok(r['operation'],r['result']),'AP3 lifecycle result')
  else:require(r['owner_thread'] is (r['operation'] not in CALLS[6:8]),'AP3 lifecycle thread')
 joins=[r for r in records if r.get('state')=='ap0_thread_joined'];threads=[r for r in records if r.get('state')=='ap0_processing_thread_started'];summary=[r for r in records if r.get('state')=='ap3_processing_summary']
 require(len(joins)==len(threads)==intervals and all(r['joined'] is True and r['processing_stopped'] is True and r['worker_exception'] is False for r in joins) and all(r['distinct_from_owner'] is True for r in threads),'AP3 processing join/thread')
 require(len(summary)==1 and summary[0]['processed_blocks']==comparison['queue']['processed'] and summary[0]['intervals']==intervals and summary[0]['process_mode']=='kRealtime','AP3 actual realtime Windows processing')
 mapping=[r for r in records if r.get('state')=='ap1_mapping_ready'];closed=[r for r in records if r.get('state')=='ap1_endpoint_closed'];ack=[r for r in records if r.get('state')=='ap2_lifecycle_ack']
 require(len(mapping)==len(closed)==1 and mapping[0]['mapping_count']==mapping[0]['connection_count']==1 and mapping[0]['mapping_witness'] is True and closed[0]['mapping_unmapped'] is True,'AP3 mapping reuse/retirement')
 require(len(ack)==2+2*intervals and ack[0]['kind']==9 and ack[0]['next_sequence']==1 and ack[-1]['kind']==15 and ack[-1]['next_sequence']==comparison['queue']['processed']+1,'AP3 lifecycle acknowledgement extent')
 next_sequence=1
 for index in range(intervals):
  start,stop=ack[1+index*2:3+index*2]
  require(start['kind']==11 and stop['kind']==13 and start['next_sequence']==next_sequence and stop['next_sequence']>=next_sequence,'AP3 lifecycle sequence')
  require(start['sequence']<joins[index]['sequence']<stop['sequence'],'AP3 stop before join')
  if index:require(ack[index*2]['sequence']<start['sequence'],'AP3 restart before quiescence')
  next_sequence=stop['next_sequence']
 require(next_sequence==ack[-1]['next_sequence'],'AP3 final sequence')
 if variant=='core':require(ack[2]['next_sequence']==11259,'AP3 first stream extent')
 require(closed[0]['sequence']>next(r['sequence'] for r in ledger if r.get('operation')=='free_library' and r['event']=='call_completed'),'AP3 mapping closed before plugin unload')
 return dict(raw=public_observation(observed,ledger,terminal),comparison=comparison,cleanup=observed['cleanup'])

def compare_short(records):
 ready=[r for r in records if r['event']=='ap3_host_ready'];done=[r for r in records if r['event']=='ap3_host_compared']
 stats=[r for r in records if r['event']=='ap3_proxy_stats'];closed=[r for r in records if r['event']=='ap3_host_closed']
 require(len(ready)==len(done)==len(stats)==len(closed)==1,'AP3 short stream extent')
 d=done[0];q=stats[0]
 require(ready[0]['seed_after_activation'] is True and d['active_frames']==300000 and d['frames_per_callback']==128 and d['samples']==602048,'AP3 short stream inputs')
 require(d['max_error']==0 and d['callback_overruns']==0 and d['callback_effects']==0 and d['fault_observed'] is False,'AP3 stream-active comparison/timing')
 require(all(d[k]==v for k,v in reconstruct(ready[0]['seed'],128,False,300000).items()),'AP3 independent short timeline')
 require(q['fault']==0 and q['processed']==2352 and q['epoch']==1 and q['position']==301024 and 1<=q['request_high']<=2048 and 1<=q['result_high']<=2048,'AP3 short queue result')
 require(closed[0]['terminate_result']==0 and closed[0]['module_unloaded'] is True and closed[0]['references_released'] is True,'AP3 short unload')
 return dict(samples_compared=602048,maximum_absolute_error=0.,streams=done,queue=q,intervals=1)

def compare_gui(records,label):
 stats=[r for r in records if r['event']=='ap3_proxy_stats'];life=[r for r in records if r['event']=='ap3_proxy_lifecycle' and r.get('blocks',0)>0];ui=[r for r in records if r['event']=='ap3_bitwig_ui']
 require(len(stats)==len(life)==len(ui)==1,'AP3 one Bitwig bridge instance/report')
 q,l,u=stats[0],life[0],ui[0]
 require(u['case']==label and all(u.get(k) is True for k in ('scan_load','playback','gain_changed','muted','stopped','removed','responsive','moonlight_control')),'AP3 direct GUI observations incomplete')
 require(l['clean'] is True and l['callback_rejections']==0 and l['requested_rate']==48000 and 1<=l['requested_maximum']<=256 and l['requested_mode']==0 and l['frames']>=960000 and l['gain_min']==0 and l['gain_max']>=.5 and l['zero_gain_blocks']>0,'AP3 Bitwig processing/control/lifecycle')
 require(q['fault']==0 and q['processed']==l['blocks'] and 1<=q['epoch']<=32 and 1<=q['request_high']<=2048 and 1<=q['result_high']<=2048,'AP3 Bitwig queued processing failure')
 return dict(queue=q,processing=l,gui=u,intervals=q['epoch'],numerical_oracle=False)

def normalize(observed):
 if 'segments' not in observed:return normalize_session(observed)
 require(set(observed['segments'])=={'core','stream_active','bitwig_first','bitwig_reopen'} and observed['cleanup']==CLEAN,'AP3 complete batch')
 sessions={k:normalize_session(v,k) for k,v in observed['segments'].items()}
 return dict(raw=dict(segments={k:v['raw'] for k,v in sessions.items()},cleanup=CLEAN),comparison=sessions['core']['comparison'],stream_active=sessions['stream_active']['comparison'],bitwig=[sessions[k]['comparison'] for k in ('bitwig_first','bitwig_reopen')],cleanup=CLEAN)

def public_observation(observed,ledger,terminal):
 # Public protocol facts only; no topology, runtime paths or private launch data.
 caller=observed['caller'];records=observed['records']
 retained={k:observed[k] for k in ('raw_exit','classification','cleanup','run_id','inherited_shutdown','stdout_sha256','stderr_sha256')}
 retained['records']=sorted(ledger+[r for r in records if str(r.get('state','')).startswith(('ap0_','ap1_','ap2_','ap3_'))]+[terminal[0]],key=lambda r:r['sequence'])
 retained['caller']={k:caller[k] for k in ('raw_exit','cleanup','records')}
 return retained

def validate_summary(summary):
 require(summary['stage_absent'] is True and summary['protected_before_sha256']==summary['protected_after_sha256'],'AP3 stage/protected state')
 result=normalize(summary['raw']);require(all(result[k]==summary[k] for k in result if k!='raw'),'AP3 retained comparison differs');return summary
