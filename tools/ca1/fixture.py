#!/usr/bin/env python3
"""Pinned-runner CA1 proof using the production session.run collector/finalizer.

Fixture command/registration substitution is the only test boundary. Actual
Popen, pipes, tracker, collector, cleanup and incident finalization are unchanged.
No vendor module, account UI, debugger or product publication is involved.
"""
import argparse,fcntl,hashlib,json,os,pathlib,subprocess,sys
from unittest.mock import patch


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--runtime',type=pathlib.Path,required=True)
    p.add_argument('--fixture',type=pathlib.Path,required=True)
    p.add_argument('--sha256',required=True)
    p.add_argument('--output',type=pathlib.Path,required=True)
    p.add_argument('--mode',choices=['handled','fatal','fatal-late','normal','exit'],required=True)
    a=p.parse_args();os.umask(0o077);a.output.mkdir(mode=0o700)
    sys.path.insert(0,str(a.runtime));import session
    manager=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
    cli=pathlib.Path.home()/'.local/bin/linux-vst-bridge'
    cap=json.loads(subprocess.check_output([str(cli),'capacity']))['capacity']
    assert cap['dsp']==0 and not cap['cleanup_unconfirmed'] and cap['maintenance']==0
    db=json.loads((manager/'registry.json').read_text())
    original=db['classes']['41727475415649534B61743150726F63']['registration']
    env=original['environment'];runner=env['runner'];artifact=dict(path=str(a.fixture.resolve(strict=True)),sha256=a.sha256)
    session.verify(artifact)
    for x in runner['files']:session.verify(x)
    # Shared lock coexists with the existing keeper, but excludes installers.
    with open(pathlib.Path(env['root'])/'operation.lock','a+b') as lock:
        fcntl.flock(lock,fcntl.LOCK_SH|fcntl.LOCK_NB)
        sid=os.urandom(16).hex();directory=pathlib.Path(env['root'])/'compatdata/pfx/drive_c/bridge/sessions'/sid
        directory.mkdir(mode=0o700)
        incident=a.output/sid;incident.mkdir(mode=0o700)
        reg=dict(metadata={'class_id':original['metadata']['class_id']},environment=env,host=artifact,module=artifact,
                 host_source_sha256=a.sha256,compatibility={'disable_windows_accessibility':False})
        spec=dict(registration=reg,session=sid,directory=str(directory),report=str(a.output/'owner-result.json'),inspect=True,
                  crash_capture=dict(schema=1,id=sid,directory=str(incident),session=sid,fixture=dict(artifact=artifact,mode=a.mode)))
        argv=[runner['entry_point'],'--verb=run','--',runner['proton'],'runinprefix','Z:'+artifact['path'].replace('/','\\')]
        if a.mode!='handled':argv.append('--'+a.mode)
        (a.output/'spec.json').write_text(json.dumps(spec))
        with patch.object(session,'command',return_value=(argv,b'')):
            result=session.run(spec)
        assert result['cleanup_confirmed'] and result['transport_retired'],result.get('error')
        assert not result['incident']['reporting_error'],result['incident']
        report=json.loads((incident/'incident.json').read_text())
        matching=[x for x in report['exceptions'] if x['belongs_to_selected_windows_host']]
        if a.mode in ('handled','fatal','fatal-late'):
            exception=next(x for x in matching if x['code']==0xc0000005)
            assert exception['fault']['module_sha256']==a.sha256,exception['fault']
            assert exception['fault']['symbol']['name']=='ap18_fault_site',exception['fault']
            assert any(x.get('symbol') and x['symbol']['name']=='ap18_fault_caller' for x in exception['stack']),exception['stack']
            expected='exception_followed_by_normal_exit' if a.mode=='handled' else 'exception_matching_self_exit_status'
            assert exception['classification']==expected,exception['classification']
        else:assert not matching,matching
        if a.mode=='fatal-late':assert report['counts']['received_bytes']>65536 and result['discarded_diagnostic_bytes']['stderr']>0
        share=json.loads((incident/'share.json').read_text())
        share.update(fixture_mode=a.mode,fixture_sha256=a.sha256,production_run=True,cleanup_confirmed=True,transport_retired=True)
        (a.output/'proof.json').write_text(json.dumps(share,indent=2)+'\n')
        print(json.dumps(share))

if __name__=='__main__':main()
