"""Actual source-owned component/controller inspection; no commercial module.
Runs only on the Windows fixture lane. The source-owned DLL spawns no processes.
"""
import hashlib, json, os, pathlib, shutil, subprocess, sys, time, uuid

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def handshake(sid,scanner,module,manifest,mode):
    return ''.join((
        'schema=linux-vst-bridge-wf0-handshake/v1\n',f'session={sid}\n',
        f'scanner_sha256={scanner}\n',f'module_sha256={module}\n',
        f'bundle_manifest_sha256={"02"*32}\n',
        f'implementation_source_manifest_sha256={manifest}\n',
        f'mode={mode}\n','component_case=first-audio\n','run_ordinal=1\n')).encode()
def publish_gate(path,binding):
    temporary=path.with_name(path.name+'.tmp')
    if path.exists():raise RuntimeError('fixture gate already exists')
    with temporary.open('xb') as stream:
        stream.write(binding);stream.flush();os.fsync(stream.fileno())
    os.replace(temporary,path)
def main():
    out=pathlib.Path(sys.argv[1]).resolve()
    host=out/'wf0-factory-probe.exe';module=out/'ap10-return-fixture.vst3'
    root=pathlib.Path(__file__).resolve().parents[2]
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    names=subprocess.check_output(['git','ls-files','windows-factory-probe','native-vst3-proxy/include','vst-state','windows-fixtures/ap10','CMakeLists.txt'],cwd=root,text=True).splitlines()
    manifest=out/'host-source-manifest.json'
    manifest.write_text(json.dumps(dict(schema=1,source_commit=commit,host_sha256=sha(host),files={n:sha(root/n) for n in names}),sort_keys=True))
    def inspect(case,expected,present,accepted,mode='ap8-module-inspection'):
        sid=uuid.uuid4().hex;directory=pathlib.Path('C:/bridge/sessions')/sid
        directory.mkdir(parents=True);fixture=directory/'fixture.vst3';shutil.copyfile(module,fixture)
        ready=directory/(sid+'.ready');gate=directory/(sid+'.gate')
        scanner_sha=sha(host);module_sha=sha(fixture);manifest_sha=sha(manifest)
        binding=handshake(sid,scanner_sha,module_sha,manifest_sha,mode)
        args=['--session',sid,'--scanner-sha256',scanner_sha,'--implementation-source-manifest-sha256',manifest_sha,
              '--module',str(fixture),'--module-sha256',module_sha,'--bundle-manifest-sha256','02'*32,
              '--ready',str(ready),'--gate',str(gate),'--max-classes','256','--stdout-cap','1048576',
              '--mode',mode,'--component-case','first-audio']
        environment=dict(os.environ)
        environment.pop('LVB_AP8_CONTROLLER_QUERY_CASE',None)
        if case:environment['LVB_AP8_CONTROLLER_QUERY_CASE']=case
        child=subprocess.Popen([str(host),*args],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=environment)
        try:
            deadline=time.monotonic()+20
            while True:
                if child.poll() is not None:raise RuntimeError('fixture exited before readiness')
                if time.monotonic()>=deadline:raise RuntimeError('fixture readiness timeout')
                try:actual=ready.read_bytes()
                except FileNotFoundError:
                    time.sleep(.01);continue
                if actual!=binding:raise RuntimeError('fixture readiness binding differs')
                break
            publish_gate(gate,binding);output,error=child.communicate(timeout=30)
            assert child.returncode==(0 if accepted else 90),(case,child.returncode,error[-1024:],output[-2048:])
            assert len(output)<=1048576
            records=[json.loads(line) for line in output.splitlines()]
            queries=[r for r in records if r.get('state')=='ap8_controller_query']
            assert len(queries)==1 and queries[0]['result']==expected and queries[0]['pointer_present'] is present,queries
            associations=[r for r in records if r.get('state')=='ap8_controller_association']
            if accepted:
                assert len(associations)==1 and associations[0]['combined'] is False,associations
                assert associations[0]['class_id']=='4150313052455455524E535445535402',associations
            else:
                assert not associations,associations
                assert any(r.get('state')=='ap8_failure' and r.get('reason')=='controller query tuple' for r in records),records[-8:]
            assert any(r.get('state')=='ap8_inspection_closed' and r['exit_code']==(0 if accepted else 90) for r in records)
            return records
        finally:
            if child.poll() is None:child.kill();child.wait(10)
            child.stdout.close();child.stderr.close();shutil.rmtree(directory)
    # Windows COM-compatible kNoInterface is signed E_NOINTERFACE.
    records=inspect(None,-2147467262,False,True)
    inspect('false-null',1,False,True)
    inspect('success-null',0,False,False)
    inspect('error-nonnull',-2147024809,True,False)
    stereo=inspect(None,-2147467262,False,True,'ap18-stereo-negotiation')
    snapshots=[r for r in stereo if r.get('state')=='ap18_stereo_bus_snapshot']
    assert [r['stage'] for r in snapshots]==['before','after'],snapshots
    assert all(r['input_count']==1 and r['output_count']==1 for r in snapshots),snapshots
    requests=[r for r in stereo if r.get('state')=='ap18_stereo_request']
    assert len(requests)==1 and requests[0]['result']==0 and requests[0]['readback_stereo'] is True and requests[0]['accepted'] is True and requests[0]['layout_changed'] is False,requests
    restores=[r for r in stereo if r.get('state')=='ap18_stereo_restore']
    assert len(restores)==1 and restores[0]['attempted'] is False and restores[0]['verified'] is True,restores
    assert any(r.get('state')=='scanner_completed' and r.get('inspection_complete') is True for r in stereo),stereo[-8:]
    editor=[r for r in records if r.get('state')=='ap8_editor_interface']
    assert len(editor)==1 and editor[0]['created'] is False and editor[0]['attached'] is False
    report=dict(schema=1,gated=True,cleanup_confirmed=True,transport_retired=True,error=None,records=records,
                provenance='source-owned Windows CI fixture; no commercial product')
    (out/'mf3-source-inspection.json').write_text(json.dumps(report,sort_keys=True))
    print('MF3 source-owned exact controller association and query tuple policy: passed')
if __name__=='__main__':main()
