"""Small trusted loader. Generator and builder authority belongs to the exact kit."""
import hashlib, json, pathlib, sys, zipfile
BUILDER='tools/mf3/native_builder.py'
GENERATOR='tools/ap8_descriptor.py'
def load_generators(request):
    archive=pathlib.Path(request['kit'])
    assert hashlib.sha256(archive.read_bytes()).hexdigest()==request['kit_sha256']
    with zipfile.ZipFile(archive) as z:
        assert len(z.namelist())==len(set(z.namelist())) and len(z.namelist())<=512
        assert z.getinfo('recipe.json').file_size<=65536
        recipe=json.loads(z.read('recipe.json'))
        assert recipe['schema']==2, 'build_recipe_generator_identity_missing'
        loaded=[]
        for name in (BUILDER,GENERATOR):
            info=z.getinfo(name);assert not info.is_dir() and info.file_size<=1024*1024
            data=z.read(name)
            assert hashlib.sha256(data).hexdigest()==recipe['files'][name], 'build_recipe_generator_changed'
            loaded.append(data)
    return recipe,loaded

def main():
    request=json.load(sys.stdin)
    try:
        recipe,(builder,generator)=load_generators(request)
        b={'__name__':'mf3_kit_builder'};g={'__name__':'mf3_kit_descriptor'}
        exec(compile(builder,BUILDER,'exec'),b)
        exec(compile(generator,GENERATOR,'exec'),g)
        result=b['build'](request,g['generate'])
        result.update(kit_sha256=request['kit_sha256'],builder_sha256=recipe['files'][BUILDER],generator_sha256=recipe['files'][GENERATOR])
        print(json.dumps(result))
    except Exception as e:
        result={'error':(str(e) or type(e).__name__)[:256]}
        if pathlib.Path(request['directory']).is_dir():
            (pathlib.Path(request['directory'])/'failure.json').write_text(json.dumps(result))
        print(json.dumps(result));raise SystemExit(1)
if __name__=='__main__':main()
