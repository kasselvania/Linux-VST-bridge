"""Offline comparison of recorder CPU, writes, phase delivery and completed blocks."""
import json,pathlib,sys,bisect
from governor_phase_analysis import blocks,coverage,distribution,cpu_window,interpolate
root=pathlib.Path(sys.argv[1]);raw=json.loads((root/'comparison.json').read_text());out={'conditions':[],'errors':raw['errors'],'settings_final':raw['guard_final']}
for c in raw['conditions']:
 row={k:c[k] for k in ['condition','guard_before','guard_at_measurement','guard_after','drain_checks','restored_identity','restored_master'] if k in c};out['conditions'].append(row)
 s=c.get('session',{});row['session']={k:s[k] for k in ('ready_seconds','elapsed_seconds','clean_shutdown','exit_code') if k in s};row['session']['graph_restored']=not any('lvb-' in x or x.startswith(' ') for x in s.get('jack_graph_after','').splitlines());row['session']['peak_temperature_c']=max(x['temperature_c'] for x in s.get('samples',[{'temperature_c':0}]));row['session']['throttle_flags']=sorted(set(x['throttled'] for x in s.get('samples',[])))
 measured=next((t for t in c['trials'] if t['name']=='measured'),None)
 if measured is None:continue
 row['counter_delta']={k:measured['after'][k]-measured['before'][k] for k in ['bridge_processed','missing_frames','gaps','midi_accepted','xruns','process_failures','delivered_frames']};row['capture_observation']=measured['capture_observation'];row['flush_marks_ns']=measured['flush_marks_ns']
 samples=measured['samples'];anchor=measured['graph_observed_ns'];rows,pubs,drops=blocks(c['phase_path'],anchor-2_000_000_000,anchor+20_000_000_000)
 row['phase_drops']=drops;row['windows']={}
 def delta(getter,start,end):
  a,b=interpolate(samples,start,getter),interpolate(samples,end,getter)
  return None if a is None or b is None else b-a
 for name,(a,b) in {'before_stimulus':(-1,0),'idle':(.3,1.3),'note_active':(3,13),'release':(15,17)}.items():
  cohort=[x for x in rows if anchor+a*1e9<=x['queued_ns']<anchor+b*1e9];published=[x for x in pubs if anchor+a*1e9<=x['monotonic_ns']<anchor+b*1e9]
  if not cohort:continue
  start,end=min(x['begin_ns'] for x in cohort),max(x['done_ns'] for x in cohort);frames=sum(x['frames'] for x in cohort)
  w={'coverage':coverage(published,cohort,b-a,drops),'blocks':len(cohort),'frames':frames,'vendor_ms':distribution([x['vendor_ms'] for x in cohort]),'service_ms':distribution([x['service_ms'] for x in cohort]),'queue_ms':distribution([x['queue_ms'] for x in cohort]),'slow_vendor_blocks':sum(x['vendor_ms']>x['frames']/48 for x in cohort),'slow_service_blocks':sum(x['service_ms']>x['frames']/48 for x in cohort),'cpu':cpu_window(samples,start,end,frames)}
  tids=set.intersection(*(set(x['native_threads']) for x in samples));native=[]
  for tid in tids:
   ns=delta(lambda s:s['native_threads'][tid]['cpu_ns'],start,end)
   if ns is not None:native.append({'name':samples[0]['native_threads'][tid]['name'],'cpu_seconds':ns/1e9,'one_core_percent':100*ns/(end-start)})
  w['native_threads']=sorted(native,key=lambda t:t['cpu_seconds'],reverse=True)
  w['native_io_delta']={k:delta(lambda s:int(s['native_io'][k]),start,end) for k in ['syscw','wchar','write_bytes']}
  row['windows'][name]=w
 # Pair each sampled file extent with newest complete request-observed record
 # actually contained in that prefix. Does not guess a flush-completion event.
 targets=sorted(samples,key=lambda x:x['phase_file_bytes']);i=0;offset=0;latest=0;ages=[];markers=[];drops2=[]
 with open(c['phase_path'],'rb') as source:
  for line in source:
   end=offset+len(line)
   while i<len(targets) and targets[i]['phase_file_bytes']<end:
    sample=targets[i];ages.append({'sample_ns':sample['monotonic_ns'],'file_bytes':sample['phase_file_bytes'],'newest_complete_worker_ns':latest or None,'age_ms':(sample['monotonic_ns']-latest)/1e6 if latest else None});i+=1
   r=json.loads(line)
   if r.get('phase')=='worker_request_observed':latest=max(latest,r['monotonic_ns'])
   if r.get('event')=='rpi1_phase_marker':markers.append({'at_ns':r['monotonic_ns'],'offset':offset})
   if r.get('event')=='rpi1_phase_dropped':drops2.append({'producer':r['producer'],'count':r['count']})
   offset=end
 while i<len(targets):
  s=targets[i];ages.append({'sample_ns':s['monotonic_ns'],'file_bytes':s['phase_file_bytes'],'newest_complete_worker_ns':latest or None,'age_ms':(s['monotonic_ns']-latest)/1e6 if latest else None});i+=1
 row['trace_delivery_samples']=ages;row['trace_delivery_age_ms']=distribution([x['age_ms'] for x in ages if x['age_ms'] is not None]);row['trace_bytes']=offset;row['trace_markers']=markers;row['all_phase_drops']=drops2
 print(c['condition'],'active',json.dumps(row['windows'].get('note_active',{})),flush=True)
(root/'reduced-recorder.json').write_text(json.dumps(out,indent=2)+'\n')
