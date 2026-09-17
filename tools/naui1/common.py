"""Read-only, bounded no-follow custody; no vendor program is imported/executed."""
import hashlib
import json
import os
import pathlib
import stat
import tempfile


class Refusal(ValueError):
    pass


def require(ok, code):
    if not ok:
        raise Refusal(code)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def decode(data):
    def pairs(items):
        obj = {}
        for k, v in items:
            require(k not in obj, 'duplicate_json_key')
            obj[k] = v
        return obj
    try:
        return json.loads(data, object_pairs_hook=pairs,
                          parse_constant=lambda _: require(False, 'nonfinite_json'))
    except (UnicodeError, json.JSONDecodeError, RecursionError) as e:
        raise Refusal('malformed_json') from e


def stamp(s):
    return s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns


def directory(path):
    p = pathlib.Path(path)
    require(p.is_absolute() and '..' not in p.parts, 'absolute_owned_location_required')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for name in p.parts[1:]:
            n = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd); fd = n
        return fd
    except BaseException:
        os.close(fd); raise


def opened(path):
    p = pathlib.Path(path)
    require(p.is_absolute() and '..' not in p.parts, 'absolute_owned_location_required')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for name in p.parts[1:-1]:
            n = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = n
        n = os.open(p.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=fd)
        s = os.fstat(n)
        if not (stat.S_ISREG(s.st_mode) and s.st_uid == os.getuid()
                and not s.st_mode & 0o022 and s.st_nlink == 1):
            os.close(n)
            raise Refusal('file_owner_type_permissions_or_alias')
        return os.fdopen(n, 'rb')
    finally:
        os.close(fd)


def read(path, maximum=16*1024*1024, expected=None):
    with opened(path) as f:
        a = os.fstat(f.fileno())
        require(a.st_size <= maximum, 'file_size_bound')
        b = f.read(maximum+1)
        require(len(b) == a.st_size and stamp(a) == stamp(os.fstat(f.fileno())), 'input_changed')
    require(expected is None or digest(b) == expected, 'input_digest_mismatch')
    return b


def file_identity(path, maximum=512*1024*1024):
    with opened(path) as f:
        a = os.fstat(f.fileno())
        require(a.st_size <= maximum, 'file_size_bound')
        h = hashlib.file_digest(f, 'sha256').hexdigest()
        require(stamp(a) == stamp(os.fstat(f.fileno())), 'input_changed')
    return {'sha256': h, 'size': a.st_size}


def publish(path, value):
    """Private durable no-replace output, never inside the inspected prefix."""
    path = pathlib.Path(path)
    fd, tmp = tempfile.mkstemp(prefix='.naui1-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as f:
            f.write(canonical(value)+b'\n')
            f.flush()
            os.fsync(f.fileno())
            os.fchmod(f.fileno(), 0o400)
        os.link(tmp, path, follow_symlinks=False)
        d = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            os.fsync(d)
        finally:
            os.close(d)
    finally:
        os.unlink(tmp)
