#!/usr/bin/env python3
"""Sealed three-session campaign; source-owned only, no commercial entry point."""
import sys
sys.dont_write_bytecode=True
import argparse,pathlib,re,os
from identity import verify_package,read_json,canonical,digest,atomic_new,envelope

MODES=('baseline','unix_override','restored')
def loader_projection(directory,proof):
    op=proof['binding']['operation']
    if not re.fullmatch('[a-f0-9]{32}',op):raise ValueError('operation')
    path=directory/(op+'-is3-observation.private.json')
    if digest(path)!=proof['observation_private_sha256']:raise ValueError('private_observation_drift')
    capture=read_json(path);rows=capture['loader_rows_private']
    if len(rows)>64 or any(not isinstance(x,str) or len(x.encode())>4096 for x in rows):raise ValueError('loader_bound')
    target=[x for x in rows if 'powershell.exe' in x.lower()]
    return {'private_sha256':digest(path),'retained_rows':len(rows),'dropped_records':capture['dropped_records'],
        'empty_environment_load_order_observed':any(':module:get_load_order_value got environment  for L"powershell.exe"' in x for x in target),
        'builtin_load_observed':any(':loaddll:build_module Loaded' in x and x.endswith(': builtin') for x in target),
        'authority':'corroboration_only; behavior and exact comparison identity select outcome'}

def campaign(package,output,seal,source,session_runner=None):
    package=pathlib.Path(package);output=pathlib.Path(output)
    manifest=verify_package(package,seal,source)
    if session_runner is None:
        from run import run
        session_runner=run
    from comparison import compare
    output.mkdir(mode=0o700)
    proofs=[]
    for mode in MODES:
        verify_package(package,seal,source)
        directory=output/mode
        session_runner(package,directory,mode,seal,source)
        proof=read_json(directory/'proof-private.json')
        if proof['session_mode']!=mode or canonical(proof['identity'])!=canonical(envelope(manifest,seal)):raise ValueError('campaign_session_identity')
        proofs.append(proof)
    result=compare(proofs)
    loader=[loader_projection(output/mode,proof) for mode,proof in zip(MODES,proofs)]
    verify_package(package,seal,source)
    public={'schema':2,'candidate_source':source,'fixture_manifest_sha256':seal,'fixture_manifest':manifest,
        'comparison_identity':envelope(manifest,seal),'result':result,'sessions':proofs,'loader_evidence':loader,
        'private_proof_sha256':{mode:digest(output/mode/'proof-private.json') for mode in MODES},
        'commercial_session':False,'runtime_installation':False,'registry_mutation':False,
        'prior_observation':'evidence/is3/loader-authority.json retained unchanged; superseded comparison custody only'}
    atomic_new(output/'loader-authority.json',public)
    return result

if __name__=='__main__':
    import json
    p=argparse.ArgumentParser();p.add_argument('--package',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--manifest-sha256',required=True);p.add_argument('--source-head',required=True);p.add_argument('--source-tree',required=True);a=p.parse_args();os.umask(0o077)
    print(json.dumps(campaign(a.package,a.output,a.manifest_sha256,{'head':a.source_head,'tree':a.source_tree})))
