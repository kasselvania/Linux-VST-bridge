"""Exercise the delivered SDK host against independent graphics reference editors.
Native Windows only: these results do not qualify Wine/Proton or hardware GPUs.
"""
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess
import sys
import time
import uuid

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(out, kind):
    host = out / 'wf0-factory-probe.exe'
    module = out / f'graphics-reference-{kind}.vst3'
    sid = uuid.uuid4().hex
    directory = pathlib.Path('C:/bridge/sessions') / sid
    directory.mkdir(parents=True)
    ready, gate = (directory / (sid + suffix) for suffix in ('.ready', '.gate'))
    source = sha(out / 'host-source-manifest.json')
    pairs = [('session', sid), ('scanner-sha256', sha(host)), ('implementation-source-manifest-sha256', source),
             ('module', str(module)), ('module-sha256', sha(module)), ('bundle-manifest-sha256', sha(module)),
             ('ready', str(ready)), ('gate', str(gate)), ('max-classes', '256'), ('stdout-cap', '1048576'),
             ('mode', 'graphics-assessment'), ('component-case', 'first-audio')]
    values = dict(pairs)
    binding = ('schema=linux-vst-bridge-wf0-handshake/v1\n' + ''.join(
        name.replace('-', '_') + '=' + values[name] + '\n' for name in (
            'session', 'scanner-sha256', 'module-sha256', 'bundle-manifest-sha256',
            'implementation-source-manifest-sha256', 'mode', 'component-case')) + 'run_ordinal=1\n').encode()
    child = subprocess.Popen([str(host), *(v for pair in pairs for v in ('--' + pair[0], pair[1]))],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        deadline = time.monotonic() + 15
        while True:
            if child.poll() is not None:
                raise RuntimeError('assessment host exited before gate')
            if time.monotonic() >= deadline:
                raise TimeoutError('assessment readiness')
            try:
                actual = ready.read_bytes()
            except (FileNotFoundError, PermissionError):
                time.sleep(.01)
                continue
            assert actual == binding
            break
        temporary = gate.with_suffix('.tmp')
        with temporary.open('xb') as stream:
            stream.write(binding)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.replace(gate)
        output, error = child.communicate(timeout=30)
        assert child.returncode == 0, (kind, child.returncode, output[-2048:], error[-1024:])
        rows = [json.loads(line) for line in output.splitlines()]
        def one(state):
            found = [r for r in rows if r.get('state') == state]
            assert len(found) == 1, state
            return found[0]
        editor = one('graphics_assessment')['editor']
        probes = {p['api']: p for p in one('graphics_runtime')['probes']}
        assert editor['closed'] is True
        assert editor['status'] == ('opened' if kind in (1, 2) else 'unavailable'), editor
        assert probes['d3d11_warp']['status'] == 'passed', probes
        assert probes['d3d11_warp']['rendering'] == 'reported_software', probes
        assert probes['opengl']['status'] == 'passed', probes
        if kind == 1:
            assert 'd3d11' in editor['during']
            assert b'reference D3D11 pixels matched' in error
        if kind == 2:
            assert 'open_gl' not in editor['before'] and 'open_gl' in editor['during'], editor
            assert b'reference OpenGL pixels matched' in error
        assert one('ap8_inspection_closed')['exit_code'] == 0
        assert one('scanner_completed')['inspection_complete'] is True
        identity = hashlib.sha256(platform.platform().encode()).hexdigest()
        context = dict(module_sha256=sha(module), class_id=one('ap11_class')['class_id'],
                       runner_fingerprint=identity, environment_fingerprint=identity, environment_revision=1,
                       host_sha256=sha(host), host_source_sha256=source, requested_graphics='Native Windows CI defaults')
        report = dict(schema=1, session=sid, gated=True, cleanup_confirmed=True, transport_retired=True,
                      error=None, records=rows, context=context,
                      provenance='Native Windows CI reference process; no Proton, commercial plug-in or hardware qualification')
        (out / f'graphics-reference-{kind}.json').write_text(json.dumps(report, sort_keys=True))
        print(json.dumps(dict(reference=kind, editor=editor, probes=probes)), flush=True)
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(10)
        child.stdout.close()
        child.stderr.close()
        shutil.rmtree(directory)

if __name__ == '__main__':
    output = pathlib.Path(sys.argv[1]).resolve()
    for fixture in (1, 2, 3, 4):
        run(output, fixture)
