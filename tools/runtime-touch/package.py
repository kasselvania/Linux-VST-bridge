#!/usr/bin/env python3
"""Seal the GE X11 correction and its complete source; never modify an installed runtime."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import shutil
import tarfile
import build

GE_SHA='c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0'
GE_ROOT='GE-Proton11-7-x86_64'
TARGET='files/lib/wine/x86_64-unix/winex11.so'
PREIMAGE='5acbf619a5d13b4d38cde3adba9724e5de5427a117caf0e2336864e20281f080'
ROOT='x11-touch'
ROLES={'source':'wine-source.tar.gz','recipe':'build.py','build_record':'build-manifest.json',
       'release_patch':'0001-touch-release.patch','routing_patch':'0002-window-touch.patch',
       'ge_recipe':'apply-ge.sh','test_source':'test_touch_flags.c',
       'notices':'THIRD_PARTY_NOTICES.txt','license':'COPYING.LIB'}


def identity(path,root):
    return dict(path=path.relative_to(root).as_posix(),sha256=build.sha(path),size=path.stat().st_size)


def add_bytes(tar,name,data,mode=0o644):
    info=tarfile.TarInfo(name);info.size=len(data);info.mode=mode
    tar.addfile(info,io.BytesIO(data))


def source_archive(output,built):
    with output.open('xb') as raw,gzip.GzipFile(fileobj=raw,mode='wb',mtime=0) as gz,tarfile.open(fileobj=gz,mode='w|') as tar:
        for path in sorted((built/'wine').rglob('*')):
            if path.name.endswith(('.orig','.rej')):continue
            info=tar.gettarinfo(str(path),str(path.relative_to(built)))
            info.uid=info.gid=info.mtime=0;info.uname=info.gname='';info.pax_headers={}
            if info.isfile():
                with path.open('rb') as src:tar.addfile(info,src)
            else:tar.addfile(info)
        for name in ['build.sh','test_touch_flags.c','apply-ge.sh',*build.PATCHES]:
            add_bytes(tar,name,(built/name).read_bytes())
        # Retain the full upstream patch sequence in addition to the patched source.
        for name in ['ge.tar.gz','staging.tar.gz']:
            add_bytes(tar,'upstream/'+name,(built/'inputs'/name).read_bytes())
        add_bytes(tar,'SDK.txt',(build.SDK+'\n').encode())


def package(built,ge,output):
    if build.sha(ge)!=GE_SHA:raise ValueError('GE archive changed')
    record=json.loads((built/'build-manifest.json').read_text())
    if (record['wine']!=build.WINE or record['ge']!=build.GE or record['staging']!=build.STAGING
        or record['sdk']!=build.SDK or record['mapping_test']!='passed'
        or record['inputs']!={n:h for n,(_,h) in build.INPUTS.items()}
        or record['artifact']['path']!='obj64/dlls/winex11.drv/winex11.so'
        or set(record['touch_sources'])!={'mouse.c','touch_message_flags.h'}
        or set(record['patches'])!=set(build.PATCHES)):
        raise ValueError('Build identity changed')
    for name,(_,digest) in build.INPUTS.items():
        if build.sha(built/'inputs'/name)!=digest:raise ValueError('Upstream input changed')
    for name,digest in record['touch_sources'].items():
        if name not in ('mouse.c','touch_message_flags.h') or build.sha(built/'wine/dlls/winex11.drv'/name)!=digest:
            raise ValueError('Consumed touch source changed')
    for name,digest in {'build.py':record['recipe_sha256'],'build.sh':record['build_script_sha256'],
                        'apply-ge.sh':record['ge_script_sha256'],**record['patches']}.items():
        if build.sha(built/name)!=digest:raise ValueError('Consumed recipe changed')
    artifact=built/record['artifact']['path']
    if build.sha(artifact)!=record['artifact']['sha256']:raise ValueError('Built driver changed')
    header=artifact.read_bytes()[:20]
    if header[:6]!=b'\x7fELF\x02\x01' or int.from_bytes(header[18:20],'little')!=62:raise ValueError('Driver architecture')
    with tarfile.open(ge) as tar:
        if hashlib.sha256(tar.extractfile(GE_ROOT+'/'+TARGET).read()).hexdigest()!=PREIMAGE:
            raise ValueError('GE driver preimage changed')
    output.mkdir(mode=0o700);component=output/ROOT;component.mkdir()
    shutil.copyfile(artifact,component/'winex11.so')
    for name in ['build.py','build-manifest.json','apply-ge.sh','test_touch_flags.c',*build.PATCHES]:
        shutil.copyfile(built/name,component/name)
    shutil.copyfile(built/'wine/COPYING.LIB',component/'COPYING.LIB')
    source_archive(component/'wine-source.tar.gz',built)
    (component/'THIRD_PARTY_NOTICES.txt').write_text(
        'Wine X11 touch correction, 2026-10-18\n'
        'x86_64-unix/winex11.so and Wine source: LGPL-2.1-or-later.\n'
        'Original copyright and license notices remain in the complete source.\n'
        'Wine base: '+build.WINE+'\nGE source: '+build.GE+'\nWine-staging: '+build.STAGING+'\n'
        'The complete GE Wine patch sequence is applied, including its restoration\n'
        'of configure and config.h.in and its wineopenxr preparation. The original\n'
        'GE and staging archives and the consumed GE script are retained.\n'
        'Then 0001-touch-release.patch and 0002-window-touch.patch correct released\n'
        'contact flags and route XI2 touch through the actual X window. mouse.c and\n'
        'touch_message_flags.h carry dated modification notices. No vendor-specific\n'
        'branch is added. Only the x64 Unix X11 driver is replaced; other runtime\n'
        'components remain those of the pinned GE release plus retained corrections.\n'
        'Extract wine-source.tar.gz into a fresh /work volume, use the SDK in SDK.txt,\n'
        'and run /bin/sh /work/build.sh. Two CPUs (2,3), 4 GiB memory/no extra swap,\n'
        '256 PIDs; no network needed. build.py also reproduces source assembly from\n'
        'hash-pinned inputs. Build success and mapping tests do not prove physical\n'
        'touch behavior. No claim of byte-identical GE release reproduction is made.\n'
        'Install only into new private staging after checking the exact GE archive\n'
        'and X11 preimage, then seal the entire new runtime. Never overwrite an\n'
        'installed runtime or vendor setup. Unmodified GE/SLR keep upstream notices.\n')
    dll=identity(component/'winex11.so',component)
    manifest=dict(schema=1,base_ge_sha256=GE_SHA,
        files=[dict(path=TARGET,original_sha256=PREIMAGE,corrected_sha256=dll['sha256'],source=dll['path'],size=dll['size'])],
        artifacts={role:identity(component/name,component) for role,name in ROLES.items()})
    (component/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    archive=output/(ROOT+'.tar.gz')
    with archive.open('xb') as raw,gzip.GzipFile(fileobj=raw,mode='wb',mtime=0) as gz,tarfile.open(fileobj=gz,mode='w|') as tar:
        for path in sorted(component.iterdir()):add_bytes(tar,ROOT+'/'+path.name,path.read_bytes())
    return dict(archive=identity(archive,output),manifest=manifest)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build',type=Path,required=True);p.add_argument('--ge-archive',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();print(json.dumps(package(a.build,a.ge_archive,a.output),sort_keys=True))
