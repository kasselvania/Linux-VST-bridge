"""Prepare one native descriptor from actual SDK inspection (not a scanner)."""
import argparse,hashlib,json,pathlib,uuid

def generate(records,class_id,module_sha256):
    buses=[r for r in records if r.get('state')=='ap8_bus']
    audio=[r for r in buses if r['media']==0]
    if [(r['direction'],r['index'],r['channels'],r['type']) for r in audio] != [(1,0,2,0)]:
        raise ValueError('prepared instrument requires no audio input and one stereo main output')
    notes=[r for r in buses if r['media']==1 and r['direction']==0]
    if len(notes)!=1 or notes[0]['index']!=0 or notes[0]['channels']!=16:
        raise ValueError('prepared note input differs')
    metadata=next(r for r in records if r.get('state')=='ap8_inspected')
    if metadata['latency_samples']!=0 or metadata['float32_result']!=0:
        raise ValueError('prepared audio contract differs')
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
