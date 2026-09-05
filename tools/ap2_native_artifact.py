"""Exact native SDK host/bundle verification; no execution or publication API."""
import pathlib,struct
from ap1_client_artifact import canonical,digest
NAMES=('ap2-offline-host','AGainOfflineBridge.vst3/Contents/x86_64-linux/AGainOfflineBridge.so')
PATHS=tuple(sorted(('CMakeLists.txt','cmake/HP0Vst3SdkLock.cmake','cmake/HP0ModernGcc.cmake',
 'native-audio-client/Cargo.toml','native-audio-client/Cargo.lock','native-audio-client/src/lib.rs','native-audio-client/src/main.rs','native-audio-client/src/mapping.rs','native-audio-client/src/endpoint.rs',
 'native-vst3-proxy/CMakeLists.txt','native-vst3-proxy/backend/Cargo.toml','native-vst3-proxy/backend/Cargo.lock','native-vst3-proxy/backend/src/lib.rs',
 'native-vst3-proxy/include/ap2_backend.h','native-vst3-proxy/source/processor.h','native-vst3-proxy/source/processor.cpp','native-vst3-proxy/source/factory.cpp','native-vst3-proxy/host/main.cpp','tools/ap2_build_native.py')))
def verify_native(root,binding):
 import json
 root=pathlib.Path(root)
 if root.is_symlink():raise RuntimeError('AP2 native root symlink')
 manifest=root/'AP2_NATIVE_BUILD.json'
 raw=manifest.read_bytes();record=json.loads(raw)
 if manifest.is_symlink() or canonical(record)!=raw or digest(raw)!=binding['manifest_sha256']:raise RuntimeError('AP2 native manifest differs')
 if record['source_commit']!=binding['source_commit'] or record['input_sha256']!=binding['input_sha256']:raise RuntimeError('AP2 native source binding differs')
 if [r['path'] for r in record['binaries']]!=list(NAMES):raise RuntimeError('AP2 native binary roster differs')
 license=root/'VST3_SDK_LICENSE.txt'
 if license.is_symlink() or digest(license.read_bytes())!=record['license_sha256']:raise RuntimeError('AP2 SDK license differs')
 if sorted(p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file())!=sorted([*NAMES,'VST3_SDK_LICENSE.txt','AP2_NATIVE_BUILD.json']):raise RuntimeError('AP2 native store roster differs')
 for item in record['binaries']:
  path=root/item['path']
  if path.is_symlink() or any(p.is_symlink() for p in path.parents if p!=root.parent):raise RuntimeError('AP2 native symlink')
  raw=path.read_bytes()
  if len(raw)>32*1024*1024 or digest(raw)!=item['sha256']:raise RuntimeError('AP2 native binary differs')
  if len(raw)<64 or raw[:6]!=b'\x7fELF\x02\x01' or struct.unpack_from('<H',raw,18)[0]!=62 or struct.unpack_from('<H',raw,16)[0] not in {2,3}:raise RuntimeError('AP2 requires native Linux x86-64 ELF')
 return record
