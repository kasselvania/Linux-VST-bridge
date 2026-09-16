#!/usr/bin/env python3
"""Qualify production IS4 selection and launch, using sealed source-owned inputs."""
import pathlib,subprocess,sys
sys.dont_write_bytecode=True
from identity import verify_package,read_json,digest,installed_identity,canonical,runner_identity,atomic_new

def main():
    if len(sys.argv)!=6:raise ValueError('closed_supervisor_arguments')
    spec_path=pathlib.Path(sys.argv[1]);mode,seal,head,tree=sys.argv[2:]
    if mode not in ('baseline','unix_override','restored'):raise ValueError('fixture_mode')
    root=pathlib.Path(__file__).parent
    manifest=verify_package(root,seal,{'head':head,'tree':tree})
    spec=read_json(spec_path)
    if spec['installer']['sha256']!=manifest['payload']['sha256'] or digest(spec['installer']['path'])!=manifest['payload']['sha256']:raise ValueError('not_exact_source_owned_payload')
    if canonical(runner_identity(spec['environment']['runner']))!=canonical(manifest['runner']):raise ValueError('runner_generation')
    managed=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
    software=read_json(managed/'software.json')
    if canonical(installed_identity(software))!=canonical(manifest['installed']):raise ValueError('installed_generation')
    if spec['installer_launch']!=software['installer_launch']:raise ValueError('adapter_identity')
    # A generated owner generation, not a replacement of installed software.
    for name,file in [('manager','policy-owner'),('supervisor','session.py'),('ownership','ownership.py')]:
        software[name]={'path':str(root/file),'sha256':manifest['files'][file]}
    selected='intentionally_unavailable' if mode=='unix_override' else 'inherited'
    owner_path=spec_path.with_name('generated-software.private.json')
    bound_path=spec_path.with_name('bound-spec.private.json')
    atomic_new(owner_path,software)
    subprocess.run([str(root/'policy-owner'),str(spec_path),str(owner_path),selected,str(bound_path)],check=True,timeout=15)
    bound=read_json(bound_path)
    # Construction is the same Rust bind() called by manager launch().
    if bound['installer_capability']['windows_scripting']!={'powershell':selected}:raise ValueError('policy_selection')
    import session
    # Read-only diagnostic capture; never select IS3's unix_override seam.
    session.managed_install(bound,source_owned_is3={'schema':1,'mode':'baseline','operation':bound['operation'],'artifact':bound['installer']})
if __name__=='__main__':main()
