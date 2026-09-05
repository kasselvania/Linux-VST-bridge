#!/usr/bin/env python3
"""Build the small static caller locally; publish exact bytes and compiler inputs."""
import argparse,json,os,pathlib,shutil,subprocess,sys
from ap1_client_artifact import PATHS,canonical,digest,elf,verify_client
ROOT=pathlib.Path(__file__).resolve().parents[1]
def run(argv):return subprocess.check_output(argv,cwd=ROOT,text=True).strip()
def input_records(source):
    return [{'path':p,'git_blob':run(['git','rev-parse',source+':'+p])} for p in PATHS]
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--source',required=True);parser.add_argument('--output',type=pathlib.Path,required=True);args=parser.parse_args()
    args.source=run(["git","rev-parse",args.source+"^{commit}"])
    records=input_records(args.source)
    for r in records:
        b=(ROOT/r['path']).read_bytes()
        import hashlib
        if hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()!=r['git_blob']:raise RuntimeError('native source differs')
    compiler=pathlib.Path(run(['rustup','which','--toolchain','stable','rustc']))
    version=run([str(compiler),'-vV']);host=next(line.split(': ',1)[1] for line in version.splitlines() if line.startswith('host: '))
    linker=compiler.parent.parent/'lib/rustlib'/host/'bin/rust-lld'
    env=os.environ.copy();env['RUSTC']=str(compiler);env['CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER']=str(linker)
    env['RUSTFLAGS']='-C linker-flavor=ld.lld -C target-feature=+crt-static --remap-path-prefix='+str(ROOT)+'=.'
    command=['rustup','run','stable','cargo','build','--manifest-path','native-audio-client/Cargo.toml','--release','--locked','--offline','--target','x86_64-unknown-linux-musl']
    subprocess.run(command,cwd=ROOT,env=env,check=True)
    binary=ROOT/'native-audio-client/target/x86_64-unknown-linux-musl/release/ap1-native-client';raw=binary.read_bytes();elf(raw)
    record={'schema':'ap1-native-build/v1','source_commit':args.source,'records':records,'input_sha256':digest(canonical(records)),
            'binary_sha256':digest(raw),'target':'x86_64-unknown-linux-musl','rustc':version,'linker_sha256':digest(linker.read_bytes()),
            'flags':['release','locked','offline','ld.lld','crt-static','repository paths remapped to .']}
    manifest=digest(canonical(record));root=args.output/manifest;root.mkdir(parents=True,exist_ok=False,mode=0o700)
    (root/'AP1_CLIENT_BUILD.json').write_bytes(canonical(record));shutil.copyfile(binary,root/'ap1-native-client');(root/'ap1-native-client').chmod(0o700)
    binding={k:record[k] for k in ('source_commit','input_sha256','binary_sha256')};binding['manifest_sha256']=manifest;verify_client(root,binding)
    (ROOT/'docs/campaigns/AP1_CLIENT.json').write_bytes(canonical(binding));print(json.dumps(binding))
if __name__=='__main__':main()
