"""One headless preference/restore mechanism check. Never rewrites vendor state."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from pigments_preset_identity import identify


def preference(blob):
    rows=[n for n in ET.fromstring(blob).iter('param') if n.get('name')=='Multicore2']
    if len(rows)!=1 or float(rows[0].get('value')) not in (0,1):
        raise ValueError('Unknown multicore preference')
    return int(float(rows[0].get('value')))


def disabled_preference(blob):
    if preference(blob)!=1:raise ValueError('Expected original Multicore ON')
    pattern=rb'(<param\s+name="Multicore2"\s+value=")([^"\n]+)("\s*/>)'
    matches=list(re.finditer(pattern,blob))
    if len(matches)!=1:raise ValueError('Ambiguous preference bytes')
    m=matches[0]
    changed=blob[:m.start(2)]+b'0.000000'+blob[m.end(2):]
    if preference(changed)!=0:raise ValueError('Preference edit readback failed')
    return changed


def state_readback(blob):
    identity=identify(blob)
    payload=blob[104:];a,b,_,_=struct.unpack_from('<IIII',payload)
    result={'identity':identity,'state_sha256':hashlib.sha256(blob).hexdigest()}
    for name,chunk in [('component',payload[16:16+a]),('controller',payload[16+a:16+a+b])]:
        values=re.findall(rb'(?<!\S)10 Multicore2 ([^ ]+) ',chunk)
        if len(values)!=1 or float(values[0]) not in (0,1):raise ValueError('Unknown state multicore field')
        result[name+'_multicore']=int(float(values[0]))
    result['agrees']=result['component_multicore']==result['controller_multicore']
    return result


def main():
    root=Path(sys.argv[1]);env=root/'pigments-arm';output=env/'logs/multicore-mechanism-01'
    output.mkdir(mode=0o700)
    pref=env/'compatdata/pfx/drive_c/ProgramData/Arturia/Pigments/tmp/plugin.pref.xml'
    original=env/'logs/preset-buttons-01/physical-navigation-final.state'
    selected=env/'logs/preset-sweep-01/005-trial-start.state'
    binary=root/'staging/phase-trace-01/trace-host'
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    if sha(binary)!='85780f4856f2bac9f605628ca7decd80568cca0ad36fa92db70960bf74dd1a62':raise ValueError('Candidate identity changed')
    def closed():
        units=subprocess.check_output(['systemctl','--user','list-units','--state=running','--plain','--no-legend','lvb-rpi*.service'],text=True)
        if units.strip():raise RuntimeError('Owned runtime still running')
        for p in Path('/proc').glob('[0-9]*/cmdline'):
            try:cmd=p.read_bytes()
            except (FileNotFoundError,PermissionError,ProcessLookupError):continue
            if b'wf0-factory-probe-rpi1-e232.exe' in cmd or b'Pigments.exe' in cmd:
                raise RuntimeError('Plugin process still running')
    closed()
    info=pref.stat();before=pref.read_bytes();backup=output/'plugin.pref.original.xml'
    shutil.copy2(pref,backup)
    report={'preference_before':preference(before),'preference_original_sha256':sha(pref),
        'preference_metadata_before':{'mode':stat.S_IMODE(info.st_mode),'uid':info.st_uid,'gid':info.st_gid,'mtime_ns':info.st_mtime_ns},
        'operator_state':state_readback(original.read_bytes()),'selected_state':state_readback(selected.read_bytes()),
        'candidate_sha256':sha(binary),'events':[],'errors':[]}
    session=None;ready=False;run=env/'logs/multicore-readback-01';fifo=run/'commands.fifo';log=run/'outer.log'
    def event(name,**fields):report['events'].append({'event':name,'monotonic_ns':time.monotonic_ns(),**fields})
    def request(command,marker):
        offset=log.stat().st_size
        fd=os.open(fifo,os.O_WRONLY|os.O_NONBLOCK)
        try:os.write(fd,(command+'\n').encode())
        finally:os.close(fd)
        deadline=time.monotonic()+15
        while time.monotonic()<deadline:
            rows=log.read_text()[offset:].splitlines()
            if any(marker in line for line in rows):return [s for s in rows if s.startswith('RPI1_')]
            if session.poll() is not None:raise RuntimeError('Session ended during readback')
            time.sleep(.1)
        raise RuntimeError('Readback timeout')
    def capture(name):
        dest=run/(name+'.state');request('save '+str(dest),'RPI1_STATE_SAVED ')
        return state_readback(dest.read_bytes())
    try:
        pref.write_bytes(disabled_preference(before))
        event('preference_changed_while_closed',value=preference(pref.read_bytes()),sha256=sha(pref))
        with (output/'session.log').open('x') as sink:
            session=subprocess.Popen(['flock','--nonblock',str(root.parent/'rpi1-private/operation.lock'),
                '/usr/bin/python3',str(env/'trace_session.py'),'multicore-readback-01','--config',str(env/'preset-navigation.conf'),
                '--binary',str(binary),'--phase-trace','off'],stdout=sink,stderr=subprocess.STDOUT)
            deadline=time.monotonic()+100
            while not log.exists() or 'RPI1_READY ' not in log.read_text():
                if session.poll() is not None or time.monotonic()>deadline:raise RuntimeError('Startup failed')
                time.sleep(.5)
            ready=True
            if 'RPI1_PHASE_TRACE mode=off recorder=false file=false' not in log.read_text():raise RuntimeError('Native phase mode mismatch')
            event('startup_state',**capture('startup'))
            request('restore '+str(selected),'RPI1_STATE_RESTORED ')
            event('after_24am_restore',**capture('after-24am'))
            report['post_restore_master']=request('parameter 0','RPI1_PARAMETER_READBACK id=0 ')
    except Exception as exc:
        report['errors'].append(str(exc))
    finally:
        try:
            if session is not None and session.poll() is None:
                if ready:
                    request('restore '+str(original),'RPI1_STATE_RESTORED ')
                    report['restored_master']=request('parameter 0','RPI1_PARAMETER_READBACK id=0 ')
                    report['restored_state']=capture('restored')
                request('quit','RPI1_CLEAN_SHUTDOWN ')
                session.wait(timeout=35)
            closed()
        except Exception as exc:
            report['errors'].append('cleanup: '+str(exc))
            if session is not None and session.poll() is None:
                subprocess.run(['systemctl','--user','stop','lvb-rpi2-multicore-readback-01.service'],timeout=25,check=False)
                session.wait(timeout=35)
        finally:
            # Only restore while the test's plugin process is confirmed closed.
            closed()
            shutil.copy2(backup,pref)
            report['preference_restored_exact']=pref.read_bytes()==before
            now=pref.stat()
            report['preference_metadata_restored']=(stat.S_IMODE(now.st_mode),now.st_uid,now.st_gid,now.st_mtime_ns)==(stat.S_IMODE(info.st_mode),info.st_uid,info.st_gid,info.st_mtime_ns)
            report['preference_after']=preference(pref.read_bytes())
            if (run/'run.json').exists():report['session']=json.loads((run/'run.json').read_text())
            report['generic_binary_sha256']=sha(root/'source-bridge/rpi0/standalone/target/release/lvb-arm-plugin-standalone')
            report['helper_default_sha256']=sha(root/'source-bridge/rpi0/standalone/target/release/lvb-arm-pigments-standalone')
            (output/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:report.get(k) for k in ['events','errors','preference_restored_exact','preference_metadata_restored']}),flush=True)


if __name__=='__main__':main()
