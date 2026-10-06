"""Read bounded, hash-bound capabilities for one exact packaged native proxy."""
import hashlib, json, sys, zipfile

def capability(request, name="maximum_bridge_frames"):
    with zipfile.ZipFile(request['kit']) as archive:
        names = archive.namelist()
        assert len(names) <= 256 and len(names) == len(set(names))
        assert archive.getinfo('recipe.json').file_size <= 65536
        recipe = json.loads(archive.read('recipe.json'))
        if recipe['schema'] not in (3,4):
            return None
        assert archive.getinfo('prebuilt/index.json').file_size <= 1024 * 1024
        data = archive.read('prebuilt/index.json')
        assert hashlib.sha256(data).hexdigest() == recipe['files']['prebuilt/index.json']
        index = json.loads(data)
        if recipe['schema'] == 4:
            assert index['schema'] == 3 and index['descriptor_schema'] == 1
            assert index['maximum_bridge_frames'] == 1024
            assert recipe['files'][index['engine']] == index['engine_sha256']
            if name == 'loaded_engine_admission_contract':
                if request.get('native_sha256', index['engine_sha256']) != index['engine_sha256']:
                    return None
                return 1 if type(index.get(name)) is int and index[name] == 1 else None
            if request['native_sha256'] != index['engine_sha256']:
                return None
            if name == 'audio_completion_contract':
                return 1 if type(index.get(name)) is int and index[name] == 1 and request['host_sha256'] == recipe['files']['runtime/host.exe'] else None
            return 1024
        if name != 'maximum_bridge_frames':
            return None
        assert index['schema'] in (1, 2) and 1 <= len(index['proxies']) <= 64
        matches = [row for row in index['proxies']
            if row['class_id'].upper() == request['class_id'].upper()
            and row['module_sha256'] == request['module_sha256']
            and row['native_sha256'] == request['native_sha256']]
        if not matches:
            return None
        assert len(matches) == 1
        selected = matches[0]
        assert recipe['files'][selected['file']] == selected['native_sha256']
        if index['schema'] == 1:
            return 512
        assert selected['maximum_bridge_frames'] == 1024
        return 1024

def maximum_bridge_frames(request):
    return capability(request)

if __name__ == '__main__':
    request = json.load(sys.stdin)
    name = request.get('capability', 'maximum_bridge_frames')
    assert name in ('maximum_bridge_frames', 'audio_completion_contract',
                    'loaded_engine_admission_contract')
    print(json.dumps(capability(request, name)))
