"""Original bounded PE/ASAR metadata reader. No archive extraction or JS execution."""
import os
import pathlib
import re
import stat
import struct
import time
from common import Refusal, require, opened, stamp, file_identity, digest, decode, directory

OPTIONAL = ('chrome_100_percent.pak', 'chrome_200_percent.pak', 'resources.pak',
            'icudtl.dat', 'snapshot_blob.bin', 'v8_context_snapshot.bin',
            'libEGL.dll', 'libGLESv2.dll')


def at(f, offset, size, maximum=32*1024*1024):
    require(0 <= offset and 0 <= size <= maximum and offset+size <= os.fstat(f.fileno()).st_size, 'metadata_range')
    f.seek(offset)
    data = f.read(size)
    require(len(data) == size, 'metadata_truncated')
    return data


def pe(f):
    d = at(f, 0, 64)
    require(d[:2] == b'MZ', 'not_pe')
    pos = struct.unpack_from('<I', d, 60)[0]
    h = at(f, pos, 24)
    require(h[:4] == b'PE\0\0', 'not_pe')
    machine, count = struct.unpack_from('<HH', h, 4)
    optsize, flags = struct.unpack_from('<HH', h, 20)
    require(machine in (0x14c, 0x8664) and 1 <= count <= 96 and 96 <= optsize <= 512
            and flags & 2 and not flags & 0x2000, 'unsupported_pe')
    opt = at(f, pos+24, optsize)
    magic = struct.unpack_from('<H', opt)[0]
    require((machine, magic) in ((0x14c, 0x10b), (0x8664, 0x20b)), 'pe_architecture')
    directory = 112 if magic == 0x20b else 96
    sections = at(f, pos+24+optsize, count*40)
    def rva(v, n):
        matches = []
        for i in range(count):
            vs, va, raw, off = struct.unpack_from('<IIII', sections, i*40+8)
            if va <= v and v+n <= va+min(vs, raw):
                matches.append(off+v-va)
        require(len(matches) == 1, 'pe_resource_mapping')
        return at(f, matches[0], n, 4*1024*1024)
    require(len(opt) >= directory+24, 'pe_resource_directory')
    rr, rs = struct.unpack_from('<II', opt, directory+16)
    versions = []
    if rr and rs:
        resource = rva(rr, rs)
        def entries(offset):
            require(offset+16 <= len(resource), 'pe_resource_header')
            names, ids = struct.unpack_from('<HH', resource, offset+12)
            require(names+ids <= 512 and offset+16+8*(names+ids) <= len(resource), 'pe_resource_count')
            return [struct.unpack_from('<II', resource, offset+16+i*8) for i in range(names+ids)]
        for kind, target in entries(0):
            if kind != 16:
                continue
            require(target & 0x80000000, 'pe_version_type')
            for _, sub in entries(target & 0x7fffffff):
                require(sub & 0x80000000, 'pe_version_name')
                for _, leaf in entries(sub & 0x7fffffff):
                    require(not leaf & 0x80000000 and leaf+16 <= len(resource), 'pe_version_leaf')
                    v, n = struct.unpack_from('<II', resource, leaf)
                    versions.append(version_strings(rva(v, n)))
    require(len(versions) <= 16, 'pe_version_count')
    facts = {}
    for row in versions:
        for k, v in row.items():
            require(k not in facts or facts[k] == v, 'pe_version_conflict')
            facts[k] = v
    return {'architecture': 'x64' if machine == 0x8664 else 'x86', 'version': facts}


def version_strings(data):
    facts = {}
    visited = [0]
    def node(start, limit, depth):
        visited[0] += 1
        require(depth <= 5 and visited[0] <= 512 and start+6 <= limit, 'version_bounds')
        length, value_len, typ = struct.unpack_from('<HHH', data, start)
        end = start+length
        require(length >= 8 and end <= limit, 'version_extent')
        p = start+6
        q = p
        while q+2 <= end and data[q:q+2] != b'\0\0':
            q += 2
        require(q+2 <= end and q-p <= 256, 'version_key')
        key = data[p:q].decode('utf-16le')
        p = (q+2+3) & ~3
        n = value_len*(2 if typ == 1 else 1)
        require(p+n <= end, 'version_value')
        if key in ('ProductName', 'FileDescription', 'OriginalFilename', 'FileVersion', 'ProductVersion'):
            require(typ == 1 and n <= 512, 'version_string_type')
            value = data[p:p+n].decode('utf-16le').rstrip('\0')
            require(key not in facts or facts[key] == value, 'version_duplicate')
            facts[key] = value
        p = (p+n+3) & ~3
        while p+6 <= end:
            p = (node(p, end, depth+1)+3) & ~3
        return end
    node(0, len(data), 0)
    return facts


def asar(f):
    size, header_size = struct.unpack('<II', at(f, 0, 8))
    require(size == 4 and 8 <= header_size <= 8*1024*1024 and header_size % 4 == 0, 'asar_header_size')
    header = at(f, 8, header_size)
    payload_size, text_size = struct.unpack_from('<II', header)
    require(payload_size == header_size-4 and text_size <= payload_size-4
            and 8+text_size+((-text_size) % 4) == header_size, 'asar_pickle')
    require(not any(header[8+text_size:]), 'asar_padding')
    tree = decode(header[8:8+text_size])
    require(isinstance(tree, dict) and set(tree) == {'files'}, 'asar_root')
    table = {}; count = [0]
    def walk(n, prefix='', depth=0):
        require(isinstance(n, dict) and depth <= 32, 'asar_tree')
        files = n.get('files')
        require(isinstance(files, dict), 'asar_directory')
        for name, entry in files.items():
            count[0] += 1
            require(count[0] <= 100000 and isinstance(entry, dict) and name not in ('', '.', '..')
                    and not any(c in name for c in '/\\\0'), 'asar_entry')
            path = prefix+name
            if 'files' in entry:
                walk(entry, path+'/', depth+1)
            else:
                table[path] = entry
    walk(tree)
    def member(name, bound):
        require(isinstance(name, str) and not name.startswith('/') and '..' not in name.split('/'), 'asar_member_path')
        e = table.get(name)
        require(isinstance(e, dict) and not e.get('unpacked') and 'link' not in e, 'asar_selected_member_unavailable')
        n = e.get('size'); off = e.get('offset')
        require(type(n) is int and isinstance(off, str) and re.fullmatch(r'0|[1-9][0-9]{0,12}', off), 'asar_member_extent')
        return at(f, 8+header_size+int(off), n, bound)
    package_bytes = member('package.json', 256*1024)
    package = decode(package_bytes)
    require(isinstance(package, dict), 'asar_package')
    main = package.get('main', 'index.js')
    require(isinstance(main, str) and len(main) <= 512, 'asar_main')
    if main.startswith('./'):
        main = main[2:]
    main_bytes = member(main, 32*1024*1024)
    # Syntax evidence only. No source text, dependency metadata or code is emitted.
    electron_import = bool(re.search(rb'''(?:require\s*\(\s*|from\s*)["']electron(?:/main)?["']''', main_bytes))
    name = package.get('productName', package.get('name', ''))
    require(isinstance(name, str), 'package_name')
    exact_name = re.sub('[-_ ]', '', name).lower() in ('nativeaccess', 'nativeaccess2')
    version = package.get('version')
    if not isinstance(version, str) or not re.fullmatch(r'\d{1,5}(?:\.\d{1,5}){1,3}(?:-[A-Za-z0-9.-]{1,32})?', version):
        version = None
    return {'package_sha256': digest(package_bytes), 'main_sha256': digest(main_bytes),
            'main_location_sha256': digest(main.encode()), 'main_size': len(main_bytes),
            'application_name_matches': exact_name, 'application_version': version,
            'electron_import_observed': electron_import, 'header_sha256': digest(header), 'entry_count': count[0]}


def discover(drive):
    """Enumerate names only; never follow prefix symlinks or read arbitrary files."""
    pending = [pathlib.Path(drive)]; candidates = []; count = 0
    deadline = time.monotonic()+45
    while pending:
        d = pending.pop()
        require(d.resolve() == d, 'aliased_census_root')
        fd = directory(d)
        try:
            with os.scandir(fd) as it:
                for e in it:
                    count += 1
                    require(count <= 100000 and time.monotonic() < deadline, 'census_bound')
                    if e.name.casefold() == 'native access.exe':
                        require(e.is_file(follow_symlinks=False), 'application_alias_or_type')
                        candidates.append((d/e.name))
                    if e.is_dir(follow_symlinks=False):
                        pending.append((d/e.name))
        finally:
            os.close(fd)
    require(len(candidates) == 1, 'application_root_not_unique')
    exe = candidates[0]; root = exe.parent
    files = {}; private = {}; budget = 1024*1024*1024
    for label in ('Native Access.exe', 'resources/app.asar')+OPTIONAL:
        p = exe if label == 'Native Access.exe' else root/label
        if label in OPTIONAL and not p.exists() and not p.is_symlink():
            continue
        ident = file_identity(p)
        require(ident['size'] > 0 and ident['size'] <= budget, 'application_hash_budget')
        budget -= ident['size']; files[label] = ident; private[label] = str(p)
    with opened(exe) as f:
        before = stamp(os.fstat(f.fileno())); metadata = pe(f)
        require(before == stamp(os.fstat(f.fileno())), 'pe_changed')
    with opened(root/'resources/app.asar') as f:
        before = stamp(os.fstat(f.fileno())); archive = asar(f)
        require(before == stamp(os.fstat(f.fileno())), 'asar_changed')
    # Names alone are not identity: PE resource product + package name + selected
    # entry + Electron import + collocated ICU/Chromium and V8 resource families.
    product = metadata['version'].get('ProductName', '')
    require(product.casefold() == 'native access' and archive['application_name_matches'], 'application_metadata_mismatch')
    electron = (archive['electron_import_observed'] and 'icudtl.dat' in files
                and any(k in files for k in ('resources.pak', 'chrome_100_percent.pak', 'chrome_200_percent.pak'))
                and any(k in files for k in ('snapshot_blob.bin', 'v8_context_snapshot.bin')))
    # Recheck every identified byte after parsing; catches stable replacement
    # between hashing and parsing too (not only mutation of an open descriptor).
    require(all(file_identity(private[k]) == v for k, v in files.items()), 'application_generation_changed')
    versions = {k: v for k, v in metadata['version'].items() if k in ('FileVersion', 'ProductVersion') and re.fullmatch(r'[0-9., ]{1,64}', v)}
    public = {'application_identity': 'established', 'renderer_family': 'electron_chromium' if electron else 'unresolved',
              'application_version': archive['application_version'], 'renderer_version': None,
              'architecture': metadata['architecture'], 'pe_versions': versions, 'asar': archive,
              'files': files, 'root_location_sha256': digest(str(root.relative_to(drive)).encode()), 'census_entries': count}
    return public, {'files': private, 'windows_image': 'C:\\'+str(exe.relative_to(drive)).replace('/', '\\')}
