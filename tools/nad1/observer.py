"""Single read-only NAD1 pass. No runtime selectors and no Windows/network execution."""
import json,os,pathlib,re,subprocess,sys
from package import verify
from common import read,decode,digest,canonical,require,publish,file_identity
from census import bundle,image,registry,metadata,proc_census,listeners,DAEMON,SERVICE
from classify import classify

def snapshot(manager):
    p=subprocess.run([manager,'operator','snapshot'],capture_output=True,timeout=150)
    require(p.returncode==0 and len(p.stdout)<16*1024*1024,'snapshot_unavailable')
    s=decode(p.stdout);a=s['system']
    require(s['schema']==6 and a['service']=='active' and a['keepers']==2 and not a['cleanup_unconfirmed'] and all(a[k]==0 for k in ['dsp','maintenance','pending_transactions','stale_transports']) and not s['capture']['armed'] and not s['capture']['active_retention'],'not_idle')
    return s

def evidence(op,raw,log,ledger):
    r=decode(raw);w=decode(ledger)
    require(r['operation']==op and w['operation']==op and r['cleanup_confirmed'] and r['owned_live']==0 and r['state']=='completed','source_not_retired')
    require(r['application_identity']=='7228a542c01b89daa5d04b9c8566af235ee7a2358e52f741b8918239c3a7d26e' and r['renderer']['launch_binding']['status']=='bound','source_application')
    # Commands remain private. Merely observe an exact request record, not success.
    facts=[]
    for line in log.splitlines():
        if b'CreateProcessInternalW' in line and b'NTKDaemon 1.32.0 Setup PC.exe' in line:
            facts.append({'class':'bundled_daemon_installer_reference_in_create_record','record_sha256':digest(line)})
    return {'operation':op,'result_sha256':digest(raw),'ledger_sha256':digest(ledger),'log_sha256':digest(log),'facts':facts,'trace_drops':r['renderer']['windows_dropped_observations'],'diagnostic_drops':r['diagnostics'],'application_identity':r['application_identity'],'requested':r['requested'],'effective':r['effective']}

def main():
    require(len(sys.argv)==1,'no_runtime_arguments');os.umask(0o077)
    source=pathlib.Path(__file__).resolve().parent;seal,seal_hash=verify(source);m=decode(read(source/'input.json'))
    require(m['schema']==1 and m['environment']=='627d2cba97edbecf113c22504eb4c81b' and m['failure_operation']=='354af73fea5773244ac3ebd21425e4ed','input_identity')
    home=pathlib.Path.home();managed=home/'.local/share/linux-vst-bridge/managed';env=managed/'environments'/m['environment'];appdir=managed/'vendor-applications/native-access';out=home/'.cache/linux-vst-bridge/nad1-observation-continuation-1'
    sw=decode(read(managed/'software.json',expected=m['software_sha256']))
    for k,h in m['installed_artifacts'].items():require(sw[k]['sha256']==h and file_identity(sw[k]['path'])['sha256']==h,'installed_changed')
    before=snapshot(sw['manager']['path']);require(not (managed/'operator/resume.json').exists(),'resume_pending')
    # One explicitly authorized corrected continuation; the original marker is retained.
    out.mkdir(mode=0o700,exist_ok=True);publish(out/'started.json',{'seal':seal_hash,'source':seal['source_head']})
    tree_before=metadata(env)
    publish(out/'before.private.json', {'environment_metadata':tree_before,'snapshot':before})
    appraw=read(appdir/'application.json',expected=m['application_record_sha256']);app=decode(appraw)
    require(digest(canonical(app))==m['application_identity'] and app['environment']['root']==str(env),'application_binding')
    eraw=read(env/'environment.json');require(decode(eraw)==app['environment'],'environment_changed')
    for f in app['files'].values():require(file_identity(f['artifact']['path'])=={'sha256':f['artifact']['sha256'],'size':f['size']},'application_resource_changed')
    prior=decode(read(home/'.cache/linux-vst-bridge/naui2-install-f12953c/before-install.json'))
    def preserved():
        for group in ['retained','projects','predecessor_files']:
            for path,h in prior[group].items():require(file_identity(path,2**31)['sha256']==h,'protected_changed')
    preserved();operation_names=sorted(p.name for p in (appdir/'operations').iterdir() if p.is_dir())
    expected_ops=sorted([x['operation'] for x in m['controls']]+[m['failure_operation']]);require(operation_names==expected_ops,'operation_set_changed')
    bound=[];input_hashes={};input_paths={}
    def retained(path,expected=None,bound=32*1024*1024):
        raw=read(path,bound,expected);input_paths[str(path)]=(digest(raw),bound);input_hashes[digest(str(path.relative_to(home)).encode())]=digest(raw);return raw
    for control in m['controls']:
        d=appdir/'operations'/control['operation']
        retained(d/'presentation.json',control['presentation_sha256'])
        raw=retained(d/'result.json',control['result_sha256'])
        log=retained(d/(control['operation']+'-stderr.private.log'))
        ledger=retained(d/(control['operation']+'-ledger.private.json'))
        bound.append(evidence(control['operation'],raw,log,ledger))
    op=m['failure_operation'];d=appdir/'operations'/op;cache=home/'.cache/linux-vst-bridge'/m['private_snapshot'];private={}
    for rel,h in m['private_sources'].items():private[rel]=retained(cache/rel,h)
    raw=retained(d/'result.json',m['private_sources']['terminal/renderer-result.private.json'])
    ledger=retained(d/(op+'-ledger.private.json'),m['private_sources']['terminal/process-ledger.private.json'])
    log=retained(d/(op+'-stderr.private.log'),m['private_sources']['terminal/runner-stderr.private.log'])
    bound.append(evidence(op,raw,log,ledger))
    sources=[x['operation'] for x in bound if x['facts']]
    # A reference may also be an argument to a query/helper. It is not launch proof,
    # and multiple operations may legitimately reference the same bundled image.
    # The retained private snapshot binds this later session independently of B.
    snapshot_manifest=decode(retained(cache/'manifest.json'))
    require(snapshot_manifest['operation']==op and snapshot_manifest['files']['application-log.private']['sha256']==digest(private['application-log.private']),'private_snapshot_source')
    require(decode(read(appdir/'current.json'))['operation']==op,'dependency_operation_changed')
    vendor=private['application-log.private'];diagnostics=[]
    for line in vendor.splitlines():
        for category,pattern in [('daemon_start_timeout',rb'timed out'),('dependency_permission',rb'[Pp]ermission'),('missing_object',rb'ENOENT')]:
            if re.search(pattern,line):diagnostics.append({'category':category,'record_sha256':digest(line),'authority':'retained_vendor_log_not_operation_attribution'})
    drive=env/'compatdata/pfx/drive_c';bundled=bundle(drive);daemon_path=drive/DAEMON;daemon=image(daemon_path) if daemon_path.exists() else None
    regraw=retained(env/'compatdata/pfx/system.reg');regs=registry(regraw)
    process=proc_census(env/'compatdata/pfx');ports=listeners()
    # No query launches Wine. Runtime SCM status is unavailable; never infer running
    # from registry alone. Absence is supported by complete exact file/key census.
    state={'schema':1,'complete':process['unavailable']==0,'installer':'exact' if len(bundled)==1 else 'absent','daemon':'exact' if daemon else 'absent','registration':'absent' if not regs else 'unavailable','service':'absent' if not regs else 'unavailable','process':'absent' if not process['candidates'] else 'unavailable','readiness':'unavailable','foreign':'unavailable' if process['unavailable'] else 'exact' if any(x['exact'] and x['prefix_relation'] in ('foreign','deleted') for x in process['candidates']) else 'absent'}
    # Bound vendor sources only; session stores/cookies are never read.
    vendor_logs=[]
    for path in (drive/'users').glob('*/Documents/Native Instruments/Logs/Native Access/native-access.log'):
        data=retained(path,bound=512*1024);vendor_logs.append({'sha256':digest(data),'size':len(data)})
    require(len(vendor_logs)<=4,'vendor_log_count')
    unit='linux-vst-bridge-renderer-'+op+'.service'
    p=subprocess.run(['systemctl','--user','show',unit,'--property=LoadState,ActiveState,MainPID,ControlPID,ControlGroup'],capture_output=True,timeout=15)
    fields=dict(line.split('=',1) for line in p.stdout.decode().splitlines() if '=' in line)
    require(fields=={'LoadState':'not-found','ActiveState':'inactive','MainPID':'0','ControlPID':'0','ControlGroup':''},'application_not_retired')
    tree_after=metadata(env);require(tree_before==tree_after,'prefix_changed');preserved()
    for path,(h,n) in input_paths.items():read(path,n,h)
    require(read(appdir/'application.json')==appraw and read(env/'environment.json')==eraw,'records_changed')
    read(managed/'software.json',expected=m['software_sha256']);after=snapshot(sw['manager']['path'])
    require(before['products']==after['products'] and before['onboarding']==after['onboarding'],'canonical_changed')
    require(sorted(p.name for p in (appdir/'operations').iterdir() if p.is_dir())==operation_names,'new_operation')
    require({str(p):os.readlink(p) for p in (home/'.vst3').glob('LVB_*.vst3') if p.is_symlink()}==prior['links'],'publication_changed')
    require(verify(source)[1]==seal_hash,'source_changed')
    # Retain raw registry/records privately for bounded offline analysis without a second pass.
    publish(out/'private.json',{'registry':regraw.decode(),'application':app,'software':sw,'processes':process,'source_files':{k:v.decode('utf8','replace') for k,v in private.items()}})
    public={'schema':1,'source_head':seal['source_head'],'source_tree':seal['source_tree'],'seal_sha256':seal_hash,'input_manifest_sha256':digest(read(source/'input.json')),'disposition':classify(state),'state':state,'bundle':bundled,'installed_daemon':daemon,'registry_sha256':digest(regraw),'service_registration_count':len(regs),'service_state_authority':'registry_presence_only_runtime_scm_unavailable_no_wine_launch','process_census':process,'passive_listeners':ports,'readiness_contract':'unavailable_no_daemon_identity_or_protocol_admitted','operation_sources':bound,'dependency_failure_snapshot_operation':op,'installer_reference_operations':sources,'failure_attribution':'snapshot operation exact; vendor-log error-to-process relation unproved','vendor_diagnostics':diagnostics,'vendor_log_sources':vendor_logs,'private_input_hashes':input_hashes,'private_result_sha256':file_identity(out/'private.json')['sha256'],'preservation':{'system':after['system'],'capture':after['capture'],'software_unchanged':True,'application_unchanged':True,'environment_entries_unchanged':len(tree_before),'protected_counts':{k:len(prior[k]) for k in ['retained','projects','predecessor_files']},'products_publications_onboarding_unchanged':True,'operation_set_unchanged':True,'A_B_C_unchanged':True,'application_closed':True,'real_daemon_mutations':0,'windows_launches':0}}
    publish(out/'result.json',public);print(canonical({'disposition':public['disposition'],'result_sha256':file_identity(out/'result.json')['sha256'],'source_seal':seal_hash}).decode())
if __name__=='__main__':main()
