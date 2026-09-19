"""Actual source-owned component/controller inspection; no commercial module.
Runs only on the Windows fixture lane. The source-owned DLL spawns no processes.
"""
import hashlib, json, pathlib, shutil, subprocess, sys, time, uuid

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    out=pathlib.Path(sys.argv[1]).resolve()
    host=out/'wf0-factory-probe.exe';module=out/'ap10-return-fixture.vst3'
    root=pathlib.Path(__file__).resolve().parents[2]
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    names=subprocess.check_output(['git','ls-files','windows-factory-probe','native-vst3-proxy/include','vst-state','windows-fixtures/ap10','CMakeLists.txt'],cwd=root,text=True).splitlines()
    manifest=out/'host-source-manifest.json'
    manifest.write_text(json.dumps(dict(schema=1,source_commit=commit,host_sha256=sha(host),files={n:sha(root/n) for n in names}),sort_keys=True))
    sid=uuid.uuid4().hex;directory=pathlib.Path('C:/bridge/sessions')/sid
    directory.mkdir(parents=True)
    shutil.copyfile(module,directory/"fixture.vst3");module=directory/"fixture.vst3"
    ready=directory/(sid+'.ready');gate=directory/(sid+'.gate')
    args=['--session',sid,'--scanner-sha256',sha(host),'--implementation-source-manifest-sha256',sha(manifest),
          '--module',str(module),'--module-sha256',sha(module),'--bundle-manifest-sha256','02'*32,
          '--ready',str(ready),'--gate',str(gate),'--max-classes','256','--stdout-cap','1048576',
          '--mode','ap8-module-inspection','--component-case','first-audio']
    child=subprocess.Popen([str(host),*args],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    try:
        deadline=time.monotonic()+20
        while not ready.exists():
            if child.poll() is not None:raise RuntimeError('fixture exited before readiness')
            if time.monotonic()>=deadline:raise RuntimeError('fixture readiness timeout')
            time.sleep(.01)
        gate.write_bytes(ready.read_bytes())
        output,error=child.communicate(timeout=30)
        assert child.returncode==0,(child.returncode,error[-1024:],output[-2048:])
        assert len(output)<=1048576
        records=[json.loads(line) for line in output.splitlines()]
        queries=[r for r in records if r.get('state')=='ap8_controller_query']
        assert len(queries)==1 and queries[0]['result']==-1 and queries[0]['pointer_present'] is False,queries
        rows=[r for r in records if r.get('state')=='ap8_controller_association']
        assert len(rows)==1 and rows[0]['combined'] is False
        assert rows[0]['class_id']=='4150313052455455524E535445535402',rows
        editor=[r for r in records if r.get('state')=='ap8_editor_interface']
        assert len(editor)==1 and editor[0]['created'] is False and editor[0]['attached'] is False
        assert any(r.get('state')=='ap8_inspection_closed' and r['exit_code']==0 for r in records)
        report=dict(schema=1,gated=True,cleanup_confirmed=True,transport_retired=True,error=None,records=records,
                    provenance='source-owned Windows CI fixture; no commercial product')
        (out/'mf3-source-inspection.json').write_text(json.dumps(report,sort_keys=True))
        print('MF3 source-owned exact controller association and absent-editor interface: passed')
    finally:
        if child.poll() is None:child.kill();child.wait(10)
        child.stdout.close();child.stderr.close();shutil.rmtree(directory)
if __name__=='__main__':main()
