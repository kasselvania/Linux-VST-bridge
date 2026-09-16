#!/usr/bin/env python3
"""Sealed source-owned invocation; never an installed/operator capability."""
import pathlib,sys
sys.dont_write_bytecode=True
from identity import verify_package,read_json,digest,select_runner,installed_identity,canonical

def main():
    if len(sys.argv)!=6:raise ValueError('closed_supervisor_arguments')
    spec_path=pathlib.Path(sys.argv[1]);mode,seal,head,tree=sys.argv[2:]
    if mode not in ('baseline','unix_override','restored'):raise ValueError('fixture_mode')
    root=pathlib.Path(__file__).parent
    manifest=verify_package(root,seal,{'head':head,'tree':tree})
    spec=read_json(spec_path)
    if spec['installer']['sha256']!=manifest['payload']['sha256'] or digest(spec['installer']['path'])!=manifest['payload']['sha256']:raise ValueError('not_exact_source_owned_payload')
    from identity import runner_identity
    if canonical(runner_identity(spec['environment']['runner']))!=canonical(manifest['runner']):raise ValueError('runner_generation')
    managed=pathlib.Path.home()/'.local/share/linux-vst-bridge/managed'
    software=read_json(managed/'software.json')
    if canonical(installed_identity(software))!=canonical(manifest['installed']):raise ValueError('installed_generation')
    if spec['installer_launch']!=software['installer_launch']:raise ValueError('adapter_identity')
    # Import the production owner only after the exact closed package verifies.
    import session
    session.managed_install(spec,source_owned_is3={'schema':1,'mode':mode,'operation':spec['operation'],'artifact':spec['installer']})
if __name__=='__main__':main()
