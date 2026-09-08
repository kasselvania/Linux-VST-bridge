"""Prepare one native descriptor from actual SDK inspection (not a scanner)."""
import argparse,hashlib,json,pathlib,uuid

def generate(records,class_id,module_sha256):
    buses=[r for r in records if r.get('state')=='ap8_bus']
    # Preserve SDK indices and arrangements. Only stereo main audio is active;
    # auxiliary audio and event outputs remain represented but inactive.
    for media in (0,1):
        for direction in (0,1):
            group=[r for r in buses if r['media']==media and r['direction']==direction]
            if len(group)>8 or [r['index'] for r in group]!=list(range(len(group))):
                raise ValueError('bus count/index bound')
    for r in buses:
        if r['media'] not in (0,1) or r['direction'] not in (0,1) or r['type'] not in (0,1):
            raise ValueError('unsupported bus metadata')
        if r['media']==0 and (r['channels']!=2 or r.get('arrangement')!=3):
            raise ValueError('only SDK-declared stereo audio arrangements supported')
        if r['media']==1 and not 0<=r['channels']<=16:
            raise ValueError('event channels outside bound')
    for direction in (0,1):
        group=[r for r in buses if r['media']==0 and r['direction']==direction]
        mains=[r for r in group if r['type']==0]
        if len(mains)!=(1 if direction or group else 0) or (mains and mains[0]['index']!=0):
            raise ValueError('one index-zero main audio bus required')
        if direction and len(group)!=1:raise ValueError('one audio output supported')
    notes=[r for r in buses if r['media']==1 and r['direction']==0]
    if len(notes)>1 or (notes and notes[0]['index']!=0):raise ValueError('one event input supported')
    metadata=next(r for r in records if r.get('state')=='ap8_inspected')
    vendor=next(r for r in records if r.get('state')=='ap11_class' and r['class_id'].upper()==class_id.upper())
    name=vendor['name']
    if not name or '\0' in name or len(name.encode('utf-8'))>63:
        raise ValueError('vendor class display name outside SDK bound')
    # Octal UTF-8 bytes keep the SDK class label identical on every compiler.
    label='"'+''.join('\\%03o'%b for b in name.encode('utf-8'))+'"'
    if metadata['float32_result']!=0:raise ValueError('float32 processing unsupported')
    effect=any(r['media']==0 and r['direction']==0 for r in buses)
    params=[p for r in records if r.get('state')=='ap8_parameters' for p in r['parameters']]
    if len(params)!=next(r['count'] for r in records if r.get('state')=='ap8_parameter_count') or len({p[0] for p in params})!=len(params):
        raise ValueError('incomplete/duplicate parameter metadata')
    # Immutable UUIDv5 namespace and logical vendor CID. No path/build/session in native class IDs.
    namespace=uuid.UUID('9389480f-b4b0-5e02-a1d7-687a57b54b3f')
    ids=[uuid.uuid5(namespace,class_id.upper()+suffix).hex for suffix in (':processor',':controller')]
    words=lambda s:','.join('0x'+s[i:i+8] for i in range(0,32,8))
    quote=lambda s:'u'+json.dumps(s,ensure_ascii=True)
    identity=bytes.fromhex(class_id+module_sha256)
    return '''#pragma once
#include <cstdint>
namespace AP8 {
struct Parameter {uint32_t id;const char16_t* title;const char16_t* units;int32_t steps,flags;double initial;};
struct Bus {uint32_t media,direction,index,channels,type,flags;uint64_t arrangement;const char16_t* name;};
inline constexpr char class_name[]='''+label+''';
inline constexpr bool effect='''+('true' if effect else 'false')+''';
inline constexpr Bus buses[]={
'''+',\n'.join('{'+','.join([str(r[k]) for k in ('media','direction','index','channels','type','flags')]+[str(r.get('arrangement',0)),quote(r['name'])])+'}' for r in buses)+'''
};
inline constexpr uint8_t identity[48]={'''+','.join(str(x) for x in identity)+'''};
#define AP8_PROCESSOR_UID '''+words(ids[0])+'''
#define AP8_CONTROLLER_UID '''+words(ids[1])+'''
inline constexpr Parameter parameters[]={
'''+',\n'.join('{'+','.join([str(p[0]),quote(p[1]),quote(p[2]),str(p[3]),str(p[4]),repr(p[6])])+'}' for p in params)+'''\n};
}
'''

def main():
    p=argparse.ArgumentParser();p.add_argument('inspection',type=pathlib.Path);p.add_argument('--class-id',required=True);p.add_argument('--module-sha256',required=True);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
    a.output.write_text(generate(json.loads(a.inspection.read_text())['records'],a.class_id,a.module_sha256))
if __name__=='__main__':main()
