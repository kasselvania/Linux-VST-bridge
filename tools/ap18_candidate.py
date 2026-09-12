#!/usr/bin/env python3
"""Engineering build input generation from the actual retained Pigments census.
These inputs are not runtime authority: publication accepts only the resulting
compiled finite profile/artifact roster. Never reads vendor state or licenses.
"""
import argparse, hashlib, json, pathlib, subprocess, zipfile
ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = 'c8660ba8765dfd42ae80faabd4f670d195a779ad'
BUILD = '9d359c30131361989d83707b9cced2c73a080669'
TREE = 'f8a5d6aaad1c3e8a70e7134d616cd07f2ee59237'
ZIP = '86ffa9c0dd229cf75262b9bca9d45ad56ed13815c090d4851e56f366aa31d201'
def sha(b): return hashlib.sha256(b).hexdigest()
def git(*a): return subprocess.check_output(['git',*a],cwd=ROOT,text=True).strip()
def generate(census, native, archive, output):
    assert git('rev-parse',SOURCE+'^{tree}') == git('rev-parse',BUILD+'^{tree}') == TREE
    assert sha(archive.read_bytes()) == ZIP
    with zipfile.ZipFile(archive) as z:
        assert z.read('SOURCE_COMMIT.txt').decode('utf-8-sig').strip() == BUILD
        host = z.read('wf0-factory-probe.exe')
        binaries = {n:sha(z.read(n)) for n in z.namelist() if n.endswith('.exe')}
    paths=['CMakeLists.txt','cmake','windows-factory-probe','vst-state','windows-fixtures/ap10',
           'native-vst3-proxy/include','native-vst3-proxy/source/output_results.h',
           '.github/workflows/ap8-windows.yml','tools/wf0-factory-census','tools/ap10-tests',
           'tools/ap11-tests','tools/ap12-tests','tools/ap13-tests','tools/ap14-tests','tools/ap18-tests',
           'tools/test_delivery_trace.py','tools/test_delivery_mailbox.py','tools/test_ap4_socket.py']
    inputs=[]
    for row in git('ls-tree','-r',BUILD,'--',*paths).splitlines():
        mode,kind,blob,path=row.split(None,3);assert kind=='blob'
        inputs.append(dict(path=path,git_mode=mode,git_blob=blob))
    manifest=dict(schema='linux-vst-bridge-ap18-host-source/v1',source_head=SOURCE,commit=BUILD,tree=TREE,
        ci_run=34664586766,artifact_id=10288860435,artifact_zip_sha256=ZIP,binaries=binaries,
        sdk_commit='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96',sdk_lock='cmake/HP0Vst3SdkLock.cmake',
        toolchain=dict(generator='Visual Studio 17 2022',architecture='x64',toolset='v143',windows_sdk='10.0.19041.0',observed_msvc='19.44.35228.0'),inputs=inputs)
    source=(json.dumps(manifest,sort_keys=True,separators=(',',':'))+'\n').encode()
    c=census['census'];r=census['report'];m=c['selected'];env=c['environment']['environment'];runner=env['runner']
    assert r['cleanup_confirmed'] and r['transport_retired'] and r['error'] is None
    assert m['name']=='Pigments' and m['vendor']==c['factory_vendor']=='Arturia' and 'Instrument' in m['subcategories'].split('|')
    assert c['float32'] and not c['float64'] and native['returncode']==0 and len(native['source_commit'])==40
    filehash=lambda path: next(a['sha256'] for a in runner['files'] if a['path']==path)
    p=dict(schema=1,id='arturia-pigments',revision=8,claim='review_candidate',module_sha256=c['module']['sha256'],factory_vendor=c['factory_vendor'],
        **{'class':m},role='instrument',requirements=dict(runner=dict(id=runner['id'],version=runner['version'],
        proton_sha256=filehash(runner['proton']),entry_point_sha256=filehash(runner['entry_point']),file_sha256=sorted(a['sha256'] for a in runner['files'])),
        environment_family=c['environment']['family'],environment_revision=env['revision'],host_sha256=sha(host),host_source_sha256=sha(source),
        native_sha256=native['sha256'],native_source_commit=native['source_commit'],descriptor_sha256=native['descriptor_sha256']),
        capabilities=dict(vendor_retirement='process_scoped_vendor_retirement',editor_lifetime='retain_editor_view_until_instance_retirement',event_output='reported_zero_event_channels_unspecified',accessibility='windows_default',editor='detached_direct_vendor_lifecycle',state='concurrent_read_only_capture_v12',precision='float32_only',performance='frames512_recommended256_unqualified'),
        limitations=['short_delivery_gaps','unqualified256','detached_focus_refusal','exact_operator_artifact_only','pigments_under_qualification','sole_stereo_auxiliary_input_only','returned_result_diagnosis'],
        evidence=['docs/AP18.md','evidence/ap18/pigments/installed.json','evidence/ap18/pigments/census.json','evidence/ap18/pigments/native-activation.json','evidence/ap18/pigments/event-output-operational-census.json','evidence/ap18/pigments/event-output-correction.json','evidence/ap18/pigments/retained-editor.json','evidence/ap18/pigments/process-retirement.json'])
    output.mkdir(mode=0o700)
    for name,data in [('host.exe',host),('host-source-manifest.json',source),('profile.json',(json.dumps(p,indent=2)+'\n').encode())]:
        (output/name).write_bytes(data);(output/name).chmod(0o400)
    return p
if __name__=='__main__':
    a=argparse.ArgumentParser();a.add_argument('--census',type=pathlib.Path,required=True);a.add_argument('--native-receipt',type=pathlib.Path,required=True);a.add_argument('--windows-zip',type=pathlib.Path,required=True);a.add_argument('--output',type=pathlib.Path,required=True);v=a.parse_args()
    p=generate(json.loads(v.census.read_text()),json.loads(v.native_receipt.read_text()),v.windows_zip,v.output)
    print(json.dumps({'id':p['id'],'revision':p['revision'],'host_sha256':p['requirements']['host_sha256'],'manifest_sha256':p['requirements']['host_source_sha256']}))
