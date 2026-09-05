"""Independent verification of standard-host return words and actual Windows calls."""
import math
from ap0_contract import require,validate_lifecycle,bits,sample
MODE='ap2-native-vst3-offline-bridge'
LENGTHS=[1,16,63,256,16,63,1,256,16,63,17,31,47]
GAINS=[.5,.25,.75,.5,.75,.25,.5,.75,0.,.5,.5,.25,.75]
SILENCE=[0,0,0,0,0,3,0,0,0,1,0,0,0]
MASK=(1<<64)-1

def request_words(seed,b,n,flags):
 x=seed^(((b+1)*0x9e3779b97f4a7c15)&MASK);channels=[]
 for ch in range(2):
  values=[]
  for _ in range(n):
   x=(x^(x<<13))&MASK;x^=x>>7;x=(x^(x<<17))&MASK
   values.append(bits(0. if flags&(1<<ch) else (x%129-64)/128.))
  if not flags&(1<<ch):values[0]=bits(-.5 if ch else .25)
  channels.append(values)
 return channels

def compare(records,reopen=False):
 ready=[r for r in records if r['event']=='ap2_host_ready'];require(len(ready)==1,'AP2 Ready count')
 ready=ready[0];seed=ready['seed'];require(type(seed)is int and 0<=seed<=MASK and ready['seed_after_activation'] is True and ready['reopen'] is reopen,'AP2 host seed/activation')
 blocks=[r for r in records if r['event']=='ap2_host_block'];require(len(blocks)==(1 if reopen else 13),'AP2 host block count')
 total=0;maximum=0.
 for b,r in enumerate(blocks):
  n=19 if reopen else LENGTHS[b];gain=1. if reopen else GAINS[b];flags=0 if reopen else SILENCE[b]
  require(r['block']==b and r['frames']==n and r['gain']==gain and r['input_silence_flags']==flags and r['result']==0 and r['in_place'] is (not reopen and b==12),'AP2 request/return configuration')
  require(r['input_bits']==request_words(seed,b,n,flags),'AP2 independently reconstructed host input differs')
  out=r['output_bits'];mask=r['output_silence_flags'];require(type(mask)is int and 0<=mask<=3 and len(out)==2,'AP2 output mask/channels')
  for ch in range(2):
   require(len(out[ch])==n,'AP2 returned extent')
   for i,word in enumerate(out[ch]):
    actual=sample(word);expected=sample(r['input_bits'][ch][i])*gain;error=abs(actual-expected)
    require(math.isfinite(actual) and error==0. and (not mask&(1<<ch) or actual==0.),'AP2 host sample or silence claim differs')
    maximum=max(maximum,error);total+=1
  require(r['comparison_ok'] is True and r['max_error']==0.,'AP2 host comparison disagrees')
 done=[r for r in records if r['event']=='ap2_host_compared'];require(len(done)==1 and done[0]['samples']==total and done[0]['audio_calls']==len(blocks) and done[0]['max_error']==maximum,'AP2 host totals')
 close=[r for r in records if r['event']=='ap2_host_closed'];require(len(close)==1 and close[0]['case']==('reopen' if reopen else 'positive') and close[0]['terminate_result']==0 and close[0]['references_released'] is True and close[0]['module_unloaded'] is True,'AP2 host release/unload')
 if not reopen:
  flush=[r for r in records if r['event']=='ap2_host_flush'];require(len(flush)==1 and flush[0]['gain']==.25 and flush[0]['audio_calls']==11,'AP2 parameter flush sequence')
  rejects=[r for r in records if r['event']=='ap2_host_rejections'];require(len(rejects)==1 and rejects[0]['count']==10 and rejects[0]['audio_calls']==10,'AP2 rejected calls')
 return {'samples_compared':total,'blocks':len(blocks),'maximum_absolute_error':maximum}

def normalize(observed):
 caller=observed['caller'];require(caller['raw_exit']==0 and caller['cleanup']=={'owned_descendants_zero':True,'process_group_empty':True},'AP2 SDK host containment')
 records=caller['records'];groups=[];group=[]
 for r in records:
  group.append(r)
  if r['event']=='ap2_host_closed':groups.append(group);group=[]
 require(not group and len(groups)==2,'AP2 two clean module sessions')
 activations=observed['activations'];require(len(activations)==2,'AP2 two Windows activations')
 summaries=[];total=0
 for i,(activation,host) in enumerate(zip(activations,groups)):
  comparison=compare(host,reopen=i==1);total+=comparison['samples_compared']
  summary=validate_lifecycle(activation,expected_blocks=comparison['blocks'])
  transport=[r for r in activation['records'] if str(r.get('state','')).startswith(('ap1_','ap2_'))]
  traits=[r for r in transport if r['state']=='ap2_processor_traits'];require(len(traits)==1 and traits[0]['latency_samples']==traits[0]['tail_samples']==0,'AP2 actual processor latency/tail')
  mapping=[r for r in transport if r['state']=='ap1_mapping_ready'];closed=[r for r in transport if r['state']=='ap1_endpoint_closed'];ack=[r for r in transport if r['state']=='ap2_lifecycle_ack']
  require(len(mapping)==len(closed)==1 and mapping[0]['mapping_count']==mapping[0]['connection_count']==1 and mapping[0]['mapping_witness'] is True and closed[0]['mapping_unmapped'] is True,'AP2 mapping lifecycle')
  require([(r['kind'],r['next_sequence']) for r in ack]==[(9,1),(11,1),(13,comparison['blocks']+1),(15,comparison['blocks']+1)],'AP2 host lifecycle acknowledgements')
  calls={r['operation']:r['sequence'] for r in summary['lifecycle'] if r['state']=='ap0_call_completed'}
  joined=next(r['sequence'] for r in summary['lifecycle'] if r['state']=='ap0_thread_joined')
  require(calls['set_active_true']<ack[0]['sequence']<calls['set_processing_true']<ack[1]['sequence'] and calls['set_processing_false']<joined<ack[2]['sequence']<calls['set_active_false']<ack[3]['sequence'],'AP2 actual lifecycle ordering')
  blocks=[r for r in host if r['event']=='ap2_host_block'];masks=[r for r in transport if r['state']=='ap1_output_silence'];buffers=[r for r in transport if r['state']=='ap1_private_buffers_valid']
  require([r['block'] for r in buffers]==list(range(len(blocks))),'AP2 Windows private bounds')
  require([(r['block'],r['input_silence_flags'],r['output_silence_flags']) for r in masks]==[(r['block'],r['input_silence_flags'],r['output_silence_flags']) for r in blocks],'AP2 actual host/Windows masks')
  ledger=summary['call_facts']['ledger'];unload=next(r['sequence'] for r in ledger if r['operation']=='free_library' and r['event']=='call_completed')
  require(closed[0]['sequence']>unload,'AP2 mapping retired before Windows unload')
  summary.update(transport_lifecycle=transport,comparison=comparison);summaries.append(summary)
 return {'activations':summaries,'caller':caller,'comparison':{'samples_compared':total,'blocks':14,'maximum_absolute_error':0.,'comparison_rule':'independent SDK host inputs and every returned float32 sample; tolerance zero'},'cleanup':observed['cleanup']}

def validate_summary(summary):
 require(summary['stage_absent'] is True and summary['protected_before_sha256']==summary['protected_after_sha256'],'AP2 retirement/protected state')
 activations=[]
 for s in summary['activations']:
  records=sorted(s['call_facts']['ledger']+s['lifecycle']+s['transport_lifecycle'],key=lambda r:r['sequence'])+[{'state':'scanner_completed','component_session':s['component_session']}]
  activations.append({'records':records,'run_id':s['run_id'],'raw_exit':s['raw_exit'],'classification':'scanner_completed','cleanup':s['cleanup'],'inherited_shutdown':s['shutdown'],'stdout_sha256':s['stdout_sha256'],'stderr_sha256':s['stderr_sha256']})
 result=normalize({'activations':activations,'caller':summary['caller'],'cleanup':summary['cleanup']})
 require(result['comparison']==summary['comparison'],'AP2 retained comparison differs');return summary
