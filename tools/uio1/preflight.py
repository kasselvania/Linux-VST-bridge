"""Pinned-runner helper fixture and non-interactive exact editor census."""
import json
import pathlib
import sys
from launch import Context,private_json

if __name__=='__main__':
    # Only development inputs: the Rust admission binary and its exact class.
    c=Context(pathlib.Path(sys.argv[1]),sys.argv[2],pathlib.Path(__file__).parent/'package')
    out=c.package/'preflight'
    out.mkdir(mode=0o700)
    fixture=c.helper('uio1-tests.exe',[],out/'fixture.log',15).finish()
    if fixture['exit']!=0:raise RuntimeError('pinned-runner fixture failed; inspect private log')
    process,rows,snapshot=c.census(out/'census.log')
    private_json(out/'identity.json',dict(process=process,rows=rows,snapshot=snapshot))
    c.fault.close()
    print(json.dumps(dict(fixture=fixture,window_count=len([r for r in rows if r['type']=='window']),
        renderer_kinds=[r['kind'] for r in rows if r['type']=='renderer_module'],
        cgroup_cleanup='systemd-owned diagnostic unit',product_unchanged=True)))
