"""Exercise the frozen native engine using identities generated after its build."""
import argparse, json, os, pathlib, shutil, subprocess, tempfile, uuid
from test_reusable_engine import kit, prepare, sha
p=argparse.ArgumentParser();p.add_argument('engine',type=pathlib.Path);p.add_argument('consumer',type=pathlib.Path);a=p.parse_args()
engine=a.engine.resolve();consumer=a.consumer.resolve()
with tempfile.TemporaryDirectory(prefix='lvb-unseen-') as tmp:
 root=pathlib.Path(tmp);archive=kit(root,engine.read_bytes());before=sha(archive.read_bytes())
 classes=[uuid.uuid4().hex.upper() for _ in range(2)]
 ids=[uuid.uuid5(uuid.UUID('9389480f-b4b0-5e02-a1d7-687a57b54b3f'),cid+':processor').hex for cid in classes]
 paths=[]
 for number,(cid,module,effect,value) in enumerate([(classes[0],'ab'*32,False,0.25),(classes[1],'cd'*32,True,0.6),(classes[0],'ef'*32,False,0.75)]):
  work=root/str(number);work.mkdir();prepare(work,archive,cid,module,effect,value,loader=True)
  paths.append(work/'prepared/native.so');assert sha(paths[-1].read_bytes())==sha(engine.read_bytes())
 for i,value in [(0,0.25),(2,0.75)]:
  subprocess.run([str(consumer),str(paths[i]),ids[0],str(paths[1]),ids[1],str(value)],check=True)
 bundles=[]
 for number,path in enumerate(paths):
  leaf=root/f'bundle-{number}.vst3'/'Contents'/'x86_64-linux';leaf.mkdir(parents=True)
  shutil.copy2(path,leaf/'native.so');shutil.copy2(path.with_name('plugin-descriptor.json'),leaf/'plugin-descriptor.json')
  bundles.append(leaf.parents[1])
 publication=root/'publication.vst3';publication.symlink_to(bundles[0],target_is_directory=True)
 published_engine=publication/'Contents'/'x86_64-linux'/'native.so'
 # An unchanged publication link must still load normally through the real bundle shape.
 subprocess.run([str(consumer),str(published_engine),ids[0],str(paths[1]),ids[1],'0.25'],check=True)
 subprocess.run([str(consumer),'--descriptor-race-refuse',str(published_engine),
                 str(publication),str(bundles[2]),str(bundles[0])],check=True)
 assert pathlib.Path(os.readlink(publication))==bundles[0]
 for kind in ['missing','bad-engine','bad-schema','oversize']:
  d=root/kind;d.mkdir();shutil.copyfile(engine,d/'native.so')
  data=json.loads(paths[0].with_name('plugin-descriptor.json').read_text())
  if kind=='bad-engine':data['engine_sha256']='00'*32
  if kind=='bad-schema':data['schema']=999
  if kind!='missing':(d/'plugin-descriptor.json').write_text(' '*(8*1024*1024+1) if kind=='oversize' else json.dumps(data))
  subprocess.run([str(consumer),str(d/'native.so')],check=True)
 assert sha(archive.read_bytes())==before
 print(json.dumps(dict(engine_sha256=sha(engine.read_bytes()),classes_chosen_after_build=2,module_update=True,
                      compiler_invocations_during_preparation=0,negative_cases=5,audio_processing_tested=False,
                      dlopen_descriptor_race_refused=True)))
