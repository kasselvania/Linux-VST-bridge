"""Read-only context/preservation for generated application proof. Private output."""
import hashlib,json,pathlib,os,subprocess

def digest(path):
    with pathlib.Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def readback():
    h=pathlib.Path.home();m=h/'.local/share/linux-vst-bridge/managed'
    sw=json.loads((m/'software.json').read_text())
    for artifact in sw.values():
        if isinstance(artifact,dict) and set(artifact)=={'path','sha256'}:
            if digest(artifact['path'])!=artifact['sha256']:raise ValueError('installed_artifact_changed')
    snapshot=json.loads(subprocess.check_output([sw['manager']['path'],'operator','snapshot'],timeout=90))
    system=snapshot['system']
    if system['service']!='active' or system['keepers']!=2 or system['cleanup_unconfirmed'] or any(system[k] for k in ('dsp','maintenance','pending_transactions','stale_transports')) or snapshot['capture']['armed']:raise ValueError('fixture_requires_idle')
    historical=json.loads((h/'.cache/linux-vst-bridge/is2-integration-33202a3/before-install.json').read_text())
    retained={p:digest(p) for p in historical['retained']}
    projects={p:digest(p) for p in historical['projects']}
    registrations=json.loads((m/'registry.json').read_text())
    return {'software':sw,'registry':registrations,'retained':retained,'projects':projects,
            'products':snapshot['products'],'onboarding':snapshot['onboarding'],
            'environments':sorted(p.name for p in (m/'environments').iterdir()),
            'links':{str(p):os.readlink(p) for p in (h/'.vst3').glob('LVB_*.vst3') if p.is_symlink()},
            'system':system,'capture':snapshot['capture']}
if __name__=='__main__':print(json.dumps(readback()))
