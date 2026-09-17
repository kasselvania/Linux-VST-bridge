"""NAD1 read-only Linux-generation census. Names admit leads, never ownership.

No Windows PID joins, signalling, service requests, socket probes, or environment
publication. Callers must keep `private` outside public receipts.
"""
import collections
import hashlib
import os
import pathlib
import re
import stat
import time

NAME = 'NTKDaemon.exe'
LIMITS = {'stat': (4096, 1), 'comm': (256, 1), 'cmdline': (65536, 4096),
          'maps': (2 * 1024 * 1024, 32768), 'environ': (262144, 8192)}
CATEGORIES = frozenset({'stat_unavailable', 'command_unavailable', 'maps_unavailable',
    'environment_unavailable', 'prefix_relation_unavailable', 'image_unavailable',
    'process_reused', 'extent_exhausted', 'record_invalid'})

class Extent(ValueError):
    pass


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def bounded(path, kind):
    maximum, records = LIMITS[kind]
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    chunks = []; total = 0; count = 0
    separator = b'\0' if kind in ('cmdline', 'environ') else b'\n'
    try:
        while True:
            part = os.read(fd, min(8192, maximum + 1 - total))
            if not part:
                break
            chunks.append(part); total += len(part); count += part.count(separator)
            if total > maximum or count > records:
                raise Extent(kind)
        return b''.join(chunks)
    finally:
        os.close(fd)


def generation(raw, pid):
    match = re.fullmatch(rb'([0-9]+) \((.*)\) (.*)\n?', raw, re.DOTALL)
    if not match or int(match[1]) != pid:
        raise ValueError('stat_identity')
    fields = match[3].split()
    if len(fields) < 20 or not fields[19].isdigit():
        raise ValueError('stat_extent')
    return int(fields[19]), match[2]


def token_lead(raw):
    # Only complete NUL-delimited argv entries, never free-form substring/prose.
    if raw and not raw.endswith(b'\0'):
        raise ValueError('command_termination')
    for token in raw.split(b'\0'):
        if token == NAME.encode() or re.fullmatch(
                rb'(?:/[^\0\r\n]+/|[A-Za-z]:\\[^\0\r\n]+\\)NTKDaemon\.exe', token):
            return True
    return False


def mapped_images(raw):
    found = set()
    for line in raw.splitlines():
        fields = line.split(None, 5)
        if len(fields) == 6:
            location = fields[5]
            deleted = location.endswith(b' (deleted)')
            if deleted:
                location = location[:-10]
            if location.startswith(b'/') and location.rsplit(b'/', 1)[-1] == NAME.encode():
                found.add((os.fsdecode(location), deleted))
    return found


def image_identity(path):
    p = pathlib.Path(path)
    if not p.is_absolute() or p.resolve() != p:
        raise ValueError('image_alias')
    fd = os.open(p, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or not 64 <= before.st_size <= 256*1024*1024:
            raise ValueError('image_extent_or_alias')
        header = os.read(fd, 64)
        if header[:2] != b'MZ':
            raise ValueError('not_pe')
        offset = int.from_bytes(header[60:64], 'little')
        if offset + 26 > before.st_size:
            raise ValueError('pe_extent')
        os.lseek(fd, offset, os.SEEK_SET); pe = os.read(fd, 26)
        if len(pe) != 26 or pe[:4] != b'PE\0\0' or pe[4:6] not in (b'\x4c\x01', b'\x64\x86'):
            raise ValueError('pe_identity')
        os.lseek(fd, 0, os.SEEK_SET); h = hashlib.sha256(); left = before.st_size
        while left:
            data = os.read(fd, min(left, 65536))
            if not data: raise ValueError('image_short')
            left -= len(data); h.update(data)
        def stamp(s): return s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns
        if stamp(before) != stamp(os.fstat(fd)) or stamp(before) != stamp(p.lstat()):
            raise ValueError('image_changed')
        return {'sha256': h.hexdigest(), 'size': before.st_size,
                'architecture': 'x86' if pe[4:6] == b'\x4c\x01' else 'x64'}
    finally:
        os.close(fd)


def prefix_relation(raw, prefix):
    if raw and not raw.endswith(b'\0'):
        raise ValueError('environment_termination')
    values = [v[11:] for v in raw.split(b'\0') if v.startswith(b'WINEPREFIX=')]
    if len(values) != 1 or not values[0]:
        raise ValueError('prefix_unavailable')
    p = pathlib.Path(os.fsdecode(values[0]))
    if not p.is_absolute() or '..' in p.parts:
        raise ValueError('prefix_unavailable')
    if not p.exists(): return 'deleted'
    if p.resolve() != p: raise ValueError('prefix_alias')
    return 'same' if p == pathlib.Path(prefix).resolve() else 'foreign'


def census(prefix, admitted=None, proc=pathlib.Path('/proc'), *, reader=bounded):
    """`admitted` is an internal exact artifact, never an operator selector.

    A full no-lead scan establishes absence under the generated-proven lead law.
    Unreadable unled tasks are ignored; exhausting a readable lead-search extent
    is explicit incompleteness because a lead could lie beyond the bound.
    """
    candidates=[]; failures=[]; private=[]; unled_unreadable=0; visited=0
    deadline=time.monotonic()+30; search_exhausted=0
    with os.scandir(proc) as entries:
        for entry in entries:
            if not entry.name.isdigit(): continue
            visited+=1
            if visited>32768 or time.monotonic()>deadline:
                search_exhausted+=1; break
            p=pathlib.Path(entry.path); pid=int(entry.name); values={}; errors={}; leads=[]
            # Linux comm/stat, argv, and mapped semantic image are independent
            # bounded lead sources. Failure without a positive lead has no role.
            for kind in ('stat','comm','cmdline','maps'):
                try: values[kind]=reader(p/kind,kind)
                except Extent: errors[kind]='extent_exhausted'
                except (OSError,ValueError): errors[kind]='unavailable'
            before=None
            try:
                if 'stat' in values:
                    before,comm=generation(values['stat'],pid)
                    if comm==NAME.encode(): leads.append('linux_stat_comm_exact')
            except ValueError: errors['stat']='unavailable'
            if values.get('comm',b'').rstrip(b'\n')==NAME.encode(): leads.append('linux_comm_exact')
            try:
                if token_lead(values.get('cmdline',b'')): leads.append('nul_command_token_exact')
            except ValueError: errors['cmdline']='unavailable'
            images=mapped_images(values.get('maps',b''))
            if images: leads.append('mapped_pe_basename_exact')
            if not leads:
                if 'extent_exhausted' in errors.values(): search_exhausted+=1
                elif errors: unled_unreadable+=1
                continue
            generation_hash=sha((str(pid)+':'+str(before)).encode())
            failure=None; stage=None; relation='unavailable'; identity=None; exact=False
            names={'stat':'stat_unavailable','cmdline':'command_unavailable','maps':'maps_unavailable'}
            for kind in ('stat','cmdline','maps'):
                if kind in errors:
                    stage=kind;failure='extent_exhausted' if errors[kind]=='extent_exhausted' else names[kind];break
            if before is None and failure is None:stage='stat';failure='stat_unavailable'
            if failure is None:
                stage='environment'
                try: environment=reader(p/'environ','environ')
                except Extent: failure='extent_exhausted'
                except (OSError,ValueError): failure='environment_unavailable'
            if failure is None:
                stage='prefix_relation'
                try: relation=prefix_relation(environment,prefix)
                except (OSError,ValueError): failure='prefix_relation_unavailable'
            if failure is None:
                stage='image'
                try:
                    if len(images)!=1: raise ValueError('image_count')
                    path,deleted=next(iter(images))
                    if deleted: raise ValueError('image_deleted')
                    identity=image_identity(path)
                    if admitted is None or identity!=admitted: raise ValueError('image_not_admitted')
                    exact=True
                except (OSError,ValueError): failure='image_unavailable'
            # Even failed detailed reads must not attach to a recycled generation.
            try:
                after,_=generation(reader(p/'stat','stat'),pid)
                if before is None or after!=before: failure='process_reused';stage='generation_recheck';exact=False
            except Extent: failure='extent_exhausted';stage='generation_recheck';exact=False
            except (OSError,ValueError): failure='stat_unavailable';stage='generation_recheck';exact=False
            row={'ordinal':len(candidates)+1,'generation_sha256':generation_hash,
                 'lead_authorities':sorted(leads),'prefix_relation':relation,
                 'exact':exact,'image':identity}
            candidates.append(row)
            if failure:
                assert failure in CATEGORIES
                item={'generation_sha256':generation_hash,'lead_authorities':sorted(leads),
                      'failure_stage':stage,'category':failure}
                failures.append(item)
                private.append(dict(item,linux_pid=pid,start_ticks=before,
                                    input_sha256={k:sha(v) for k,v in values.items()}))
    counts=dict(sorted(collections.Counter(x['category'] for x in failures).items()))
    return {'candidates':candidates,'unavailable':len(failures)+search_exhausted,
            'visited':visited,'unled_unreadable':unled_unreadable,
            'search_extent_exhausted':search_exhausted,'failure_counts':counts,
            'failure_hashes':[sha(__import__('json').dumps(x,sort_keys=True).encode()) for x in failures],
            'private':private}
