#!/usr/bin/env python3
"""Prepare declared inputs for an internal integrated delivery candidate.

Build-side only. Private fixture/SDK locations never enter the component manifest.
Package assembly independently verifies the source, component hashes and roster.
"""
from pathlib import Path
import argparse,datetime,hashlib,json,re,shutil,subprocess,py_compile,zipfile
parser=argparse.ArgumentParser()
parser.add_argument('--build-root',type=Path,required=True)
parser.add_argument('--source',type=Path,default=Path(__file__).resolve().parents[2])
parser.add_argument('--windows-package',type=Path,required=True)
parser.add_argument('--kit',type=Path,required=True)
parser.add_argument('--fixture',type=Path,required=True)
parser.add_argument('--reference-directory',type=Path)
parser.add_argument('--version',required=True)
parser.add_argument('--epoch',type=int,required=True)
a=parser.parse_args()
assert re.fullmatch(r'[0-9][A-Za-z0-9.]{0,79}',a.version)
b=a.build_root.resolve();s=a.source.resolve();version=a.version
p=b/('inputs-'+version);p.mkdir(mode=0o700)
sha=lambda data:hashlib.sha256(data).hexdigest()
git=lambda *a:subprocess.check_output(['git','-C',str(s),*a],text=True).strip()
head,tree=git('rev-parse','HEAD'),git('rev-parse','HEAD^{tree}')
assert not git('status','--porcelain'), 'Commit the exact candidate inputs first'
copies={s/'bridge-manager/target/x86_64-unknown-linux-gnu/release/linux-vst-bridge':'linux-vst-bridge',s/'manager-ui/target/x86_64-unknown-linux-gnu/release/linux-audio-compatibility-manager':'linux-audio-compatibility-manager',a.windows_package/'wf0-factory-probe.exe':'host.exe',a.windows_package/'host-source-manifest.json':'host-source-manifest.json',a.kit:'preparation-kit.zip',s/'compatibility/arturia-pure-lofi.json':'profile.json',s/'COPYRIGHT.md':'LVB-Proprietary.txt',a.fixture:'fixture.vst3'}
for source,name in copies.items():shutil.copyfile(source,p/name)
for name in ('session','ownership'):
 py_compile.compile(str(s/f'bridge-manager/runtime/{name}.py'),cfile=str(p/(name+'.pyc')),dfile='/usr/lib/linux-vst-bridge/supervisor/'+name+'.py',doraise=True,invalidation_mode=py_compile.PycInvalidationMode.CHECKED_HASH)
with zipfile.ZipFile(a.kit) as z:
 index=json.loads(z.read('prebuilt/index.json'));(p/'proxy.so').write_bytes(z.read(index['proxies'][0]['file']))
 for name in ('vst3sdk','base','pluginterfaces','public.sdk'):(p/(name+'.txt')).write_bytes(z.read('licenses/'+name+'.txt'))
(p/'START_HERE.html').write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Internal test</title><h1>Self-service delivery test</h1><p>Internal test only. Open Linux Audio Compatibility Manager, adopt the package and start its service. Setup offers Install compatibility runtime (728 MB download). Download the official supported vendor installer separately and import it. Supported prebuilt proxy metadata: Pure LoFi 1.0.0.6121 and Efx FRAGMENTS 1.0.0.2925. No compiler, VST3 SDK or Flatpak SDK is needed on this machine.</p><p>This build is not a customer release. Vendor licensing belongs to the vendor and user. Compatibility, sound and recall require the actual test.</p></html>')
(p/'THIRD_PARTY_NOTICES.txt').write_text('INTERNAL TEST ONLY\nVST3 SDK 3.8.1 MIT notices are retained as separate license files and in preparation-kit.zip. Fixed GE-Proton11-7 and Valve SLR 4.0.20260805.254769 are acquired directly from upstream through explicit setup; all upstream files and notices are preserved. No runtime, proprietary installer, commercial plug-in, license material or preset is redistributed in this package. Complete customer SBOM/notices and customer signing are still unselected. The included open test module is first-party AP10 instrumentation.\n')
(p/'COMPLIANCE_MANIFEST.json').write_text(json.dumps(dict(schema=1,classification='internal_test_only',package_version=version,source_head=head,release_authorized=False,third_party_notice_archive_complete=False,customer_signing_key_selected=False,external_proton_slr_required=False,vendor_plugins_or_licenses_included=False,physical_plugin_or_audio_claim=False),sort_keys=True))
rows=[('usr/bin/linux-vst-bridge','linux-vst-bridge','manager'),('usr/bin/linux-audio-compatibility-manager','linux-audio-compatibility-manager','frontend'),('usr/lib/linux-vst-bridge/supervisor/session.pyc','session.pyc','supervisor'),('usr/lib/linux-vst-bridge/supervisor/ownership.pyc','ownership.pyc','ownership'),('usr/lib/linux-vst-bridge/host/bridge-host.exe','host.exe','windows_host'),('usr/lib/linux-vst-bridge/host/source-manifest.json','host-source-manifest.json','host_source'),('usr/lib/linux-vst-bridge/preparation/preparation-kit.zip','preparation-kit.zip','preparation_kit'),('usr/lib/linux-vst-bridge/proxy/PureLoFi.so','proxy.so','proxy'),('usr/lib/linux-vst-bridge/self-test/ap10-return-fixture.vst3','fixture.vst3','fixture'),('usr/lib/linux-vst-bridge/profiles/arturia-pure-lofi.json','profile.json','profile')]
if a.reference_directory is not None:
 reference=json.loads((a.reference_directory/'LVB_REFERENCE_MANIFEST.json').read_text())
 expected={'LVB_Reference_Plugins_1_0_0.exe','LVB_Reference_Recovery_1_0_0.exe','LVB_Reference_Partial_1_0_0.exe','lvb-reference-instrument.vst3','lvb-reference-effect.vst3'}
 assert reference['schema']==1 and reference['classification']=='first_party_test_instrumentation' and reference['version']=='1.0.0'
 assert len(reference['files'])==5 and {r['file'] for r in reference['files']}==expected
 for row in reference['files']:
  name=row['file'];data=(a.reference_directory/name).read_bytes()
  assert data.startswith(b'MZ') and sha(data)==row['sha256']
  (p/name).write_bytes(data);rows.append(('usr/lib/linux-vst-bridge/self-test/'+name,name,'fixture'))
 (p/'LVB_REFERENCE_MANIFEST.json').write_text(json.dumps(reference,sort_keys=True))
 rows.append(('usr/lib/linux-vst-bridge/self-test/LVB_REFERENCE_MANIFEST.json','LVB_REFERENCE_MANIFEST.json','fixture_resource'))
 (p/'START_HERE.html').write_text((p/'START_HERE.html').read_text()+'<p>First-party development fixtures are included under /usr/lib/linux-vst-bridge/self-test. Import LVB_Reference_Plugins_1_0_0.exe through Setup for the stateful instrument/effect journey. The Recovery installer deliberately holds a window for Focus/Stop; Partial deliberately holds after installing the instrument. These fixtures require no vendor sign-in and do not qualify commercial software.</p>')
 (p/'THIRD_PARTY_NOTICES.txt').write_text((p/'THIRD_PARTY_NOTICES.txt').read_text()+'The additional LVB Reference instrument, effect and three installers are first-party test instrumentation. Their exact modules, including the MIT SDK boundary, are inventoried by digest.\n')
rows += [('usr/share/doc/linux-vst-bridge-beta/licenses/'+name,name,'license') for name in ('LVB-Proprietary.txt','vst3sdk.txt','base.txt','pluginterfaces.txt','public.sdk.txt')]
rows += [('usr/share/doc/linux-vst-bridge-beta/'+name,name,kind) for name,kind in [('START_HERE.html','guide'),('THIRD_PARTY_NOTICES.txt','notices'),('COMPLIANCE_MANIFEST.json','compliance')]]
sbom=dict(spdxVersion='SPDX-2.3',dataLicense='CC0-1.0',SPDXID='SPDXRef-DOCUMENT',name='Internal self-service test inventory',documentNamespace='https://github.com/kasselvania/Linux-VST-bridge/spdx/internal/'+head,creationInfo={'created':datetime.datetime.fromtimestamp(a.epoch,datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),'creators':['Tool: internal test inventory']},files=[dict(SPDXID='SPDXRef-File-'+str(n),fileName='/'+dest,checksums=[{'algorithm':'SHA256','checksumValue':sha((p/name).read_bytes())}],licenseConcluded='NOASSERTION',copyrightText='NOASSERTION') for n,(dest,name,kind) in enumerate(rows,1)])
(p/'SBOM.spdx.json').write_text(json.dumps(sbom,sort_keys=True));rows.append(('usr/share/doc/linux-vst-bridge-beta/SBOM.spdx.json','SBOM.spdx.json','sbom'))
files=[]
for dest,name,kind in rows:
 item=dict(destination=dest,source=str(p/name),sha256=sha((p/name).read_bytes()),kind=kind,component='self-service-'+kind,mode='0555' if kind in ('manager','frontend','proxy') else '0444')
 if kind in ('manager','frontend'):item.update(build_head=head,build_tree=tree)
 files.append(item)
spec=dict(schema=2,version=version,source_head=head,source_tree=tree,operator_schema=14,external_runtime={'id':'managed-ge-proton11-7-slr4-20260805-r3','manifest_sha256':sha(b'c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0\n3226d8234e7c0542ee767837832bfb1dad5e5e2dc944ec97eb221b437f6b9349\n')},files=files)
(b/('package-spec-'+version+'.json')).write_text(json.dumps(spec,sort_keys=True,indent=2))
print(json.dumps({'head':head,'tree':tree,'files':len(files),'proxies':len(index['proxies'])}))
