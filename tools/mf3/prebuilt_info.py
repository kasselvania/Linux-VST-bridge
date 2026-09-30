"""Read bounded, hash-bound capabilities for one exact packaged native proxy."""
import hashlib, json, sys, zipfile

def maximum_bridge_frames(request):
    with zipfile.ZipFile(request['kit']) as archive:
        names = archive.namelist()
        assert len(names) <= 256 and len(names) == len(set(names))
        assert archive.getinfo('recipe.json').file_size <= 65536
        recipe = json.loads(archive.read('recipe.json'))
        if recipe['schema'] != 3:
            return None
        assert archive.getinfo('prebuilt/index.json').file_size <= 1024 * 1024
        data = archive.read('prebuilt/index.json')
        assert hashlib.sha256(data).hexdigest() == recipe['files']['prebuilt/index.json']
        index = json.loads(data)
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

if __name__ == '__main__':
    print(json.dumps(maximum_bridge_frames(json.load(sys.stdin))))
