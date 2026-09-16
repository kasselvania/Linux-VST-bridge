"""Adapter contract on Windows; source-owned bytes only, no desktop input."""
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(adapter, image, directory, expected=None, wrapper=None, depth=0):
    operation, token = os.urandom(16).hex(), os.urandom(32).hex()
    digest = expected or sha(image)
    request = directory / 'request.private'
    request.write_bytes(('\n'.join(('IS2_LAUNCH_V1', operation, token, '2', digest,
                                  str(image.stat().st_size), str(image), ''))).encode('utf-16le'))
    argv = [str(wrapper), "--adapter", str(depth), str(adapter), str(request)] if wrapper else [str(adapter), str(request)]
    result = subprocess.run(argv, cwd=directory,
                            capture_output=True, timeout=30)
    frames = [v.split() for v in result.stderr.decode().splitlines() if v.startswith('IS2_ROOT_V1 ')]
    if frames:
        assert len(frames) == 1
        assert frames[0][1:6] == [operation, token, '2', digest, str(image.stat().st_size)]
        assert len(frames[0]) == 10 and int(frames[0][6]) != int(frames[0][8])
    request.unlink()
    return result, frames


def main(directory):
    adapter = (directory / 'is2-launch.exe').resolve()
    source = (directory / 'is2-fixture.exe').resolve()
    with tempfile.TemporaryDirectory(prefix='is2-contract-') as name:
        root = Path(name)
        target = root / 'fixture.exe'
        target.write_bytes(source.read_bytes())
        case = Path(str(target) + '.case')
        case.write_text('contract\n')
        direct = subprocess.run([str(target)], cwd=root, capture_output=True, timeout=20)
        assert direct.returncode == 0
        contract = Path(str(target) + '.contract')
        before = contract.read_bytes()
        bound, frames = run(adapter, target, root)
        assert bound.returncode == 0 and len(frames) == 1
        assert contract.read_bytes() == before, 'adapter changed cwd/environment/elevation'
        for test, code in (('exit23', 23), ('wrapper1', 0), ('wrapper3', 0)):
            case.write_text(test + '\n')
            result, frames = run(adapter, target, root)
            assert result.returncode == code and len(frames) == 1, (test, result.returncode, result.stdout, result.stderr)
        case.write_text('exit23\n')
        for depth in (1, 3):
            result, frames = run(adapter, target, root, wrapper=source, depth=depth)
            assert result.returncode == 23 and len(frames) == 1
        alias = root / 'alias.exe'
        os.link(target, alias)
        Path(str(alias) + '.case').write_text('exit23\n')
        result, frames = run(adapter, alias, root)
        assert result.returncode == 23 and len(frames) == 1
        other = root / 'other'
        other.mkdir()
        lookalike = other / 'fixture.exe'
        lookalike.write_bytes(source.read_bytes() + b'UNRELATED')
        result, frames = run(adapter, lookalike, root, sha(source))
        assert result.returncode == 126 and not frames
        request = root / 'truncated.private'
        request.write_bytes(b'I\x00S\x002\x00')
        refused = subprocess.run([str(adapter), str(request)], capture_output=True, timeout=10)
        assert refused.returncode == 123 and b'IS2_ROOT_V1' not in refused.stderr
    print('IS2 Windows adapter: exact copy, alias, mismatch, contract, wrappers, exit and truncation passed')

if __name__ == '__main__':
    main(Path(sys.argv[1]))
