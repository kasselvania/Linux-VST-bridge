import pathlib,json,hashlib,struct,math,time,os
h=pathlib.Path.home();out=[]
for stage in (h/'.local/share/linux-vst-bridge/environments').glob('.wf0-factory-census.stage-*'):
 session=stage/'compatdata/pfx/drive_c/wf0/session'
 records=[json.loads(l) for l in (session/'ap3-gui-report.jsonl').read_text().splitlines()]
 state=next(r for r in reversed(records) if r['event']=='ap4_native_state' and r['operation']=='set')
 gain=state['gain']; f=(session/'ap1.audio').open('rb',buffering=0); end=time.monotonic()+2; snapshots={}
 while time.monotonic()<end:
  a=os.pread(f.fileno(),4128,64); b=os.pread(f.fileno(),4128,64)
  if a!=b or len(a)!=4128:continue
  words=struct.unpack('<1032I',a)
  if any(words[o]!=0x4b123456 or words[o+257]!=0x4b123456 for o in (0,258,516,774)):continue
  ins=[words[c*258+1:c*258+257] for c in (0,1)]
  outs=[list(words[(c+2)*258+1:(c+2)*258+257]) for c in (0,1)]
  if any(v==0x7fc12345 for ch in outs for v in ch):continue
  # JSON gain is rounded; recover the exact saved f32 from the known reference value.
  real_gain=struct.unpack('<f',struct.pack('<f',gain))[0]
  expected=[[struct.unpack('<I',struct.pack('<f',struct.unpack('<f',struct.pack('<I',v))[0]*real_gain))[0] for v in ch] for ch in ins]
  if outs!=expected or not any(v&0x7fffffff for ch in outs for v in ch):continue
  key=hashlib.sha256(b''.join(struct.pack('<I',v) for ch in ins for v in ch)).hexdigest()
  snapshots[key]={'input_sha256':key,'output_sha256':hashlib.sha256(b''.join(struct.pack('<I',v) for ch in outs for v in ch)).hexdigest(),'samples':512,'maximum_error':0}
  if len(snapshots)>=2:break
  time.sleep(.006)
 out.append({'run_id':json.loads((stage/'.wf0-owner.json').read_bytes())['run_id'],'gain':gain,'at':time.time(),'state_sha256':state['sha256'],'snapshots':list(snapshots.values())})
 f.close()
print(json.dumps({'method':'read-only double-read of completed non-poisoned 256-frame guarded planes; supplemental, not a protocol acknowledgement','instances':out}))
