#!/usr/bin/env python3
"""One source-owned A/B/control session through the installed installer owner."""
import argparse,hashlib,json,os,pathlib,shutil,subprocess,time
from report import summarize
from environment import validate
from identity import verify_package,select_runner,installed_identity,canonical,envelope,atomic_new,read_json

def digest(p):
    with pathlib.Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def run(package, output, mode, seal, source):
    package=pathlib.Path(package);manifest=verify_package(package,seal,source);payload=package/"payload.exe"
    os.umask(0o077)
    h=pathlib.Path.home();m=h/'.local/share/linux-vst-bridge/managed'
    cli=h/'.local/bin/linux-vst-bridge'
    activity=json.loads(subprocess.check_output([str(cli),'operator','activity'],timeout=90))
    system=activity['system']
    assert system['service']=='active' and not system['cleanup_unconfirmed']
    assert all(system[k]==0 for k in ('dsp','maintenance','pending_transactions','stale_transports'))
    assert not activity['capture']['armed'] and not activity['capture']['active_retention']
    software=json.loads((m/'software.json').read_text())
    if canonical(installed_identity(software))!=canonical(manifest['installed']):raise ValueError('installed_generation_drift')
    runner=select_runner(read_json(m/'registry.json'),manifest['runner'])
    assert shutil.disk_usage(output.parent).free>2*1024**3
    output.mkdir(mode=0o700)
    root=output/'environment';root.mkdir(mode=0o700)
    for name in ('home','compatdata','runtime-var','host-cache','host-config','host-data','host-tmp','client'):(root/name).mkdir(mode=0o700)
    (root/'operation.lock').touch(mode=0o600)
    exe=output/'fixture.exe';shutil.copyfile(payload,exe);exe.chmod(0o400)
    operation=os.urandom(16).hex();report=output/'result.json';spec=output/'spec.json'
    spec.write_text(json.dumps({'schema':2,'operation':operation,'environment':{'id':operation,'root':str(root),'revision':1,'runner':runner},'installer':{'path':str(exe),'sha256':digest(exe)},'format':'pe_executable','installer_launch':software['installer_launch'],'report':str(report)}))
    unit='linux-vst-bridge-installer-'+operation+'.service'
    began=time.monotonic()
    subprocess.run(['systemd-run','--user','--collect','--property=UMask=0077','--property=KillMode=control-group','--property=TimeoutStopSec=20','--property=StandardOutput=null','--property=StandardError=null','--unit='+unit,'/usr/bin/python3','-B',str(pathlib.Path(__file__).with_name('supervise.py')),str(spec),mode,seal,source['head'],source['tree']],check=True,timeout=15)
    try:
        while time.monotonic()-began<120:
            if report.exists() and json.loads(report.read_text()).get('cleanup_confirmed'):break
            time.sleep(.1)
    finally:
        if subprocess.run(['systemctl','--user','is-active','--quiet',unit]).returncode==0:
            subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=35)
    result=json.loads(report.read_text())
    assert result['cleanup_confirmed'] and result['owned_live']==0
    private_path=output/(operation+'-transaction-private.json');private=json.loads(private_path.read_text())
    for row in private['ledger']['processes']:
        try: assert int((pathlib.Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(')',1)[1].split()[19])!=row['start_ticks']
        except FileNotFoundError:pass
    log=output/(operation+'-private.log');assert log.stat().st_size<=8*1024**2
    capture_path=output/(operation+'-is3-observation.private.json');capture=json.loads(capture_path.read_text())
    lines=[l for l in capture['oracle_rows'] if l.startswith('IS3_CAP_V1')]
    images={}
    for arch in ('system32','syswow64'):
        p=root/'compatdata/pfx/drive_c/windows'/arch/'WindowsPowerShell/v1.0/powershell.exe'
        if p.exists():images[arch]={'sha256':digest(p),'size':p.stat().st_size}
    env_lines=[l for l in capture['oracle_rows'] if l.startswith('IS3_ENV_V1')]
    unix_path=output/(operation+'-is3-unix.private.json')
    proof={'schema':2,'session_mode':mode,'observation_private_sha256':digest(capture_path),'oracle_record_count':len(capture['oracle_rows']),'loader_record_count':len(capture['loader_rows_private']),'observation_drops':capture['dropped_records'],'windows_environment_lines':env_lines,'unix_environment':json.loads(unix_path.read_text()),'outer_exit':result['raw_exit'],'cleanup_confirmed':True,'owned_survivors':0,'payload_sha256':digest(exe),'source_sha256':{n:digest(pathlib.Path(__file__).with_name(n)) for n in ('run.py','report.py','capability.cpp')},'installed_artifacts':{k:software[k]['sha256'] for k in ('manager','supervisor','ownership','installer_launch')},'powershell_images':images,'private_transaction_sha256':digest(private_path),'private_log_sha256':digest(log),'binding':result['transaction'].get('launch_binding'),'duration_seconds':time.monotonic()-began,'runner_id':runner['id'],'oracle_lines':lines}
    # Retire our scratch prefix before any classification; retain exact raw proof
    # even on a later identity/oracle refusal. Final proof itself is immutable.
    assert root.resolve()==root;shutil.rmtree(root);proof['scratch_prefix_removed']=True
    identity=envelope(manifest,seal)
    identity['powershell_images']=images
    identity['installed']=installed_identity(read_json(m/'software.json'))
    verify_package(package,seal,source)
    select_runner(read_json(m/'registry.json'),manifest['runner'])
    proof['identity']=identity
    proof['comparison']=summarize(lines)
    proof['windows_environment']=validate(env_lines)
    proof['loader_observation']='bounded private Wine diagnostics corroborate behavior only'
    proof['staged_runtime_sha256']=manifest['files']['session.py']
    atomic_new(output/'proof-private.json',proof)
    if canonical(identity)!=canonical(envelope(manifest,seal)):raise ValueError('observed_identity_drift')
    if result['raw_exit']!=0 or proof['binding']['status']!='bound':raise ValueError('session_not_complete')
    return proof
