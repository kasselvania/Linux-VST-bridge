"""AP4 reference-state and actual returned-sample admission, independent of DSP."""
import hashlib,struct,math
import ap3_contract as previous
from ap0_contract import require
MODE='ap4-plugin-state-recall'
CLEAN=previous.CLEAN
SDK=('state_capture','state_gain','state_mute')
GUI=('bitwig_first','bitwig_gain','bitwig_mute')
MODULE=bytes.fromhex('60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f')
CLASS=bytes.fromhex('84e8de5f92554f5396fae4133c935a18')
def decode(hexadecimal):
 b=bytes.fromhex(hexadecimal);require(len(b)==116 and b[:8]==b'LVBSTATE' and struct.unpack_from('<II',b,8)==(1,104),'AP4 envelope header')
 require(b[16:32]==CLASS and b[32:64]==MODULE and struct.unpack_from('<II',b,64)==(12,0) and hashlib.sha256(b[104:]).digest()==b[72:104],'AP4 state identity/extent/integrity')
 values=struct.unpack('<ffi',b[104:]);require(all(math.isfinite(v) and 0<=v<=1 for v in values[:2]) and values[2] in (0,1),'AP4 reference values')
 return b[104:],values

def reconstruct(record):
 n=record['active_frames'];require(type(n)is int and 0<n<=1440000,'AP4 stream bound')
 factor=record['initial_factor'];require(factor in (0.,.25,.375,.5,1.),'AP4 reference factor')
 mute=record['mute_position'];require(mute==(1<<64)-1 or type(mute)is int and 0<=mute<=n and mute%256==0,'AP4 parameter-only ordering')
 x=record['seed'];require(type(x)is int and 0<=x<=previous.MASK,'AP4 fresh seed')
 position=0;b=0;ih=oh=14695981039346656037;reference=[[0.]*4096 for _ in range(2)]
 while position<n+1024:
  length=min(256,n+1024-position)
  if position>=mute:factor=0.
  flags=3 if position>=n or b%17==8 else 1 if b%13==4 else 0
  for i in range(length):
   for ch in range(2):
    x=(x^(x<<13))&previous.MASK;x^=x>>7;x=(x^(x<<17))&previous.MASK
    sample=0. if flags&(1<<ch) else (x%65-32)/256.
    reference[ch][(position+i)%4096]=sample*factor;ih=previous.fold(ih,sample)
  for ch in range(2):
   for i in range(length):oh=previous.fold(oh,0. if position+i<1024 else reference[ch][(position+i-1024)%4096])
  position+=length;b+=1
 require(record['input_fnv1a64']==ih and record['output_fnv1a64']==oh,'AP4 independently reconstructed sample digests differ')
 require(record['samples']==2*(n+1024) and record['max_error']==0 and record['callback_effects']==0 and record['callback_overruns']==0 and record['latency_samples']==1024,'AP4 actual samples/callback/latency')
 require(0<=record['callback_p99_ns']<=record['callback_max_ns']<=256*1000000000//48000,'AP4 callback timing')
 return record['samples']

def compare_sdk(records,label):
 reports=[r for r in records if r['event']=='ap4_host_compared'];states=[r for r in records if r['event']=='ap4_host_state'];q=[r for r in records if r['event']=='ap3_proxy_stats'];closed=[r for r in records if r['event']=='ap3_host_closed']
 require(len(q)==len(closed)==1 and q[0]['fault']==0 and all(1<=q[0][k]<=2048 for k in ('request_high','result_high')),'AP4 queue failure')
 require(closed[0]['terminate_result']==0 and closed[0]['module_unloaded'] is True and closed[0]['references_released'] is True,'AP4 native unload')
 require(len(reports)==(3 if label=='state_capture' else 2),'AP4 restored audio cases')
 decoded=[]
 for r in states:
  payload,values=decode(r['envelope_hex']);require(r['controller_gain']==values[0],'AP4 controller differs from processor');decoded.append(dict(case=r['case'],payload_hex=payload.hex(),gain=values[0],reduction=values[1],bypass=values[2],sha256=hashlib.sha256(payload).hexdigest()))
 require(decoded and decoded[0]['case']=='fresh_default' and (decoded[0]['gain'],decoded[0]['reduction'],decoded[0]['bypass'])==(1.,0.,0),'AP4 new processor default')
 if label=='state_capture':
  require(reports[0]['active_frames']==1440000 and reports[0]['initial_gain_sent'] is True and reports[0]['initial_factor']==.25 and reports[0]['parameter_only_mute'] is True,'AP4 paced save/flush case')
  for case,gain in [('overlapping_save',.25),('parameter_only_mute',0.)]:require(any(r['case']==case and r['gain']==gain and r['reduction']==0 and r['bypass']==0 for r in decoded),'AP4 actual capture missing')
  require([r['initial_factor'] for r in reports[1:]]==[.375,1.] and all(r['initial_gain_sent'] is False for r in reports[1:]),'AP4 non-gain state processing')
  require(any(r['reduction']==.125 and r['bypass']==0 for r in decoded) and any(r['reduction']==.125 and r['bypass']==1 for r in decoded),'AP4 full state fields')
 else:
  require(reports[0]['initial_gain_sent'] is False and reports[0]['initial_factor']==(.25 if label=='state_gain' else 0.),'AP4 restored audio cannot resend gain')
  require(reports[1]['initial_gain_sent'] is True and reports[1]['initial_factor']==.5,'AP4 deliberate later edit')
 return dict(samples_compared=sum(reconstruct(r) for r in reports),maximum_absolute_error=0.,streams=reports,states=decoded,queue=q[0],intervals=len(reports))

def compare_gui(records,label):
 ui=[r for r in records if r['event']=='ap4_bitwig_ui'];life=[r for r in records if r['event']=='ap3_proxy_lifecycle' and r.get('blocks',0)>0];q=[r for r in records if r['event']=='ap3_proxy_stats'];w=[r for r in records if r['event']=='ap4_sample_comparison'];project=[r for r in records if r['event']=='ap4_project']
 require(len(ui)==len(life)==len(q)==len(w)==len(project)==1,'AP4 one DAW session/result')
 u,l,q,w,project=ui[0],life[0],q[0],w[0],project[0]
 require(u['case']==label and all(u.get(k)is True for k in ('playback','saved','quit','responsive','moonlight_control')),'AP4 direct DAW observations')
 require(l['clean'] is True and l['callback_rejections']==0 and l['requested_rate']==48000 and 1<=l['requested_maximum']<=256 and l['frames']>=48000 and q['fault']==0 and q['processed']>=l['blocks'],'AP4 DAW processing/cleanup')
 require(w['samples']==2*l['frames'] and w['maximum_error']==0,'AP4 actual DAW samples differ')
 native=[r for r in records if r['event']=='ap4_native_state']
 wanted=.25 if label=='bitwig_first' else 0.
 require(any(r['operation']=='get' and r['gain']==wanted for r in native),'AP4 saved processor value')
 if label!='bitwig_first':
  restored=.25 if label=='bitwig_gain' else 0.
  require(all(u.get(k)is True for k in ('restored_control','restored_playback','observed_before_edit')),'AP4 reopen observation before edit')
  require(w['restores']>0 and w['restored_gain']==restored and w['before_edit_samples']>=48000 and w['restored_samples']>=w['before_edit_samples'],'AP4 restored processing before edits')
  require(any(r['operation']=='set' and r['gain']==restored for r in native),'AP4 actual processor restore')
  require(project['project_before_sha256']==project['expected_before_sha256'],'AP4 saved project changed before reopen')
 if label=='bitwig_mute':require(u.get('later_edit')is True and w['nonzero_samples']>0 and project['project_sha256']==project['project_before_sha256'],'AP4 deliberate edit after mute, unchanged saved carrier')
 return dict(queue=q,processing=l,gui=u,project=project,comparison=w,intervals=q['epoch'],samples_compared=w['samples'],maximum_absolute_error=w['maximum_error'])

def normalize_session(observed,label):
 caller=observed['caller'];require(caller['raw_exit']==0 and caller['cleanup']==CLEAN,'AP4 native containment')
 comparison=compare_sdk(caller['records'],label) if label in SDK else compare_gui(caller['records'],label)
 result=previous.validate_windows_session(observed,comparison,label,ap4=True)
 records=observed['records'];calls=[r for r in records if r.get('state') in {'ap4_state_started','ap4_state_result'}]
 require(calls and len(calls)%2==0,'AP4 actual state call pairs')
 for a,b in zip(calls[::2],calls[1::2]):require(a['state']=='ap4_state_started' and b['state']=='ap4_state_result' and a['operation']==b['operation'] and a['owner_thread'] is True and b['result']==0,'AP4 state call result/owner')
 readback=[r for r in records if r.get('state')=='ap4_state_readback'];require(readback,'AP4 Windows state readback missing')
 for r in readback:require(r['payload_bytes']==12 and len(bytes.fromhex(r['payload_hex']))==12,'AP4 Windows state extent')
 if label in SDK:
  require(all(any(r['payload_hex']==v['payload_hex'] for r in readback) for v in comparison['states']),'AP4 host state not corroborated by Windows')
 result['comparison']['windows_state']=readback
 return result

def normalize(observed):
 require(set(observed['segments']) in (set(SDK),set(SDK+GUI)) and observed['cleanup']==CLEAN,'AP4 complete bounded batch')
 sessions={k:normalize_session(v,k) for k,v in observed['segments'].items()}
 captured={r['case']:r['payload_hex'] for r in sessions['state_capture']['comparison']['states']}
 for label,case in [('state_gain','overlapping_save'),('state_mute','parameter_only_mute')]:
  require(any(r['case']=='restore_readback' and r['payload_hex']==captured[case] for r in sessions[label]['comparison']['states']),'AP4 fresh-session saved bytes differ')
 if set(GUI)<=set(sessions):
  first,second,third=[sessions[k]['comparison']['project'] for k in GUI]
  require(first['project_sha256']==second['project_before_sha256'] and second['project_sha256']==third['project_before_sha256'],'AP4 same saved project across fresh processes')
 return dict(raw=dict(segments={k:v['raw'] for k,v in sessions.items()},cleanup=CLEAN),comparison=dict(samples_compared=sum(v['comparison']['samples_compared'] for v in sessions.values()),maximum_absolute_error=0.,latency_samples=1024),sessions={k:v['comparison'] for k,v in sessions.items()},cleanup=CLEAN)

def validate_summary(summary):
 require(summary['stage_absent'] is True and summary['protected_before_sha256']==summary['protected_after_sha256'],'AP4 stage/protected state')
 expected=normalize(summary['raw']);require(all(summary[k]==v for k,v in expected.items() if k!='raw'),'AP4 retained admission differs');return summary
