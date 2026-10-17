"""Adapter contract on Windows; source-owned bytes only, no desktop input."""
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(adapter, image, directory, expected=None, wrapper=None, depth=0,version=1):
    operation, token = os.urandom(16).hex(), os.urandom(32).hex()
    digest = expected or sha(image)
    request = directory / 'request.private'
    request.write_bytes(('\n'.join((f'IS2_LAUNCH_V{version}', operation, token, '2', digest,
                                  str(image.stat().st_size), str(image), ''))).encode('utf-16le'))
    argv = [str(wrapper), "--adapter", str(depth), str(adapter), str(request)] if wrapper else [str(adapter), str(request)]
    result = subprocess.run(argv, cwd=directory,
                            capture_output=True, timeout=30)
    frames = [v.split() for v in result.stderr.decode().splitlines() if v.startswith('IS2_ROOT_V1 ')]
    if frames:
        assert len(frames) == 1
        assert frames[0][1:6] == [operation, token, '2', digest, str(image.stat().st_size)]
        assert len(frames[0]) == 10 and int(frames[0][6]) != int(frames[0][8])
    exits=[v.split() for v in result.stderr.decode().splitlines() if v.startswith('IS2_EXIT_V1 ')]
    if version==2 and frames:
        assert len(exits)==1 and exits[0][1:10]==frames[0][1:10],result.stderr
        began,ended,status=map(int,exits[0][10:])
        assert 0<=began<=ended<2**32 and ended-began<=3600000
        assert status==result.returncode
    else:assert not exits
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
        for test, code in (('exit23', 23), ('wrapper1', 0), ('wrapper3', 0),('external_zero',0),('exit259',259)):
            case.write_text(test + '\n')
            result, frames = run(adapter, target, root)
            assert result.returncode == code and len(frames) == 1, (test, result.returncode, len(frames))
            result,frames=run(adapter,target,root,version=2)
            assert result.returncode==code and len(frames)==1,(test,result.returncode,len(frames))
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
        # The outer Proton/Python launcher can start well before Wine opens the
        # chosen PE. Replacing A with B in that gap must refuse before B runs.
        target.write_bytes(source.read_bytes() + b'BUILD-A')
        contract.unlink(missing_ok=True)
        operation, token = os.urandom(16).hex(), os.urandom(32).hex()
        request = root / 'delayed.private'
        request.write_bytes(('\n'.join(('NAUI2_LAUNCH_V1', operation, token, '2', sha(target),
                              str(target.stat().st_size), str(target), 'inherited', ''))).encode('utf-16le'))
        ready, release = root / 'outer-started', root / 'open-image'
        wrapper = "import pathlib,subprocess,sys,time;ready,release=map(pathlib.Path,sys.argv[1:3]);ready.touch();\nwhile not release.exists(): time.sleep(.01)\nsys.exit(subprocess.call(sys.argv[3:]))"
        outer = subprocess.Popen([sys.executable, '-c', wrapper, str(ready), str(release), str(adapter), str(request)],
                                 cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 5
            while not ready.exists():
                assert time.monotonic() < deadline, 'delayed launcher did not start'
                time.sleep(.01)
            replacement = root / 'replacement.exe'; replacement.write_bytes(source.read_bytes() + b'BUILD-B')
            replacement.replace(target); release.touch()
            out, err = outer.communicate(timeout=30)
            assert outer.returncode == 126 and b'IS2_ROOT_V1' not in err, (outer.returncode, out, err)
            assert not contract.exists(), 'Unadopted replacement executed user code'
        finally:
            if outer.poll() is None: outer.kill(); outer.wait(timeout=5)
        request = root / 'truncated.private'
        request.write_bytes(b'I\x00S\x002\x00')
        refused = subprocess.run([str(adapter), str(request)], capture_output=True, timeout=10)
        assert refused.returncode == 123 and b'IS2_ROOT_V1' not in refused.stderr
    print('IS2 Windows adapter: exact copy, alias, mismatch, contract, wrappers, exit and truncation passed')

if __name__ == '__main__':
    main(Path(sys.argv[1]))
