"""Prepared, installed-module inspection using the existing launcher and owner.

No installer invocation, discovery service or authorization handling. Inputs are
explicit local files. Proprietary files and private reports never enter Git.
"""
import dataclasses
import hashlib
import json
import pathlib
import secrets
import shutil
import time
import types
import subprocess
import re

import ap4_preview as owner
from common import canonical_json, environment_parent, sha256_file, write_atomic
from environment import ScanEnvironment

MODE = 'ap8-module-inspection'
verify_runtime = owner.profile.verify_runtime


@dataclasses.dataclass(frozen=True)
class Environment(ScanEnvironment):
    @property
    def module(self):
        return self.wf0 / 'fixture' / self.marker['module_relative']


def verify_environment(environment, *, runner_identity_sha256):
    root = environment.root
    if (root.parent != environment_parent() or root.name != '.wf0-factory-census.stage-' + environment.run_id
            or root.is_symlink() or json.loads((root / '.wf0-owner.json').read_bytes()) != environment.marker):
        raise RuntimeError('AP8 owned stage identity differs')
    if environment.marker['runner_identity_sha256'] != runner_identity_sha256:
        raise RuntimeError('AP8 runner identity differs')
    for p, expected in [(environment.scanner, environment.marker['scanner_sha256']),
                        (environment.module, environment.marker['module_sha256'])]:
        if p.is_symlink() or not p.is_file() or sha256_file(p) != expected:
            raise RuntimeError('AP8 staged executable changed')


def create(scanner, module, *, source_sha256, resources=()):
    scanner, module = pathlib.Path(scanner), pathlib.Path(module)
    for p in [scanner, module]:
        if not p.is_file() or p.is_symlink():
            raise RuntimeError('AP8 input must be an existing regular Windows file')
    run_id = secrets.token_hex(16)
    root = environment_parent() / ('.wf0-factory-census.stage-' + run_id)
    relative = module.name + '/Contents/x86_64-win/' + module.name
    marker = dict(schema='linux-vst-bridge-ap8-owned-stage/v1', run_id=run_id,
        fixture=module.stem, module_relative=relative, scanner_sha256=sha256_file(scanner),
        module_sha256=sha256_file(module), implementation_source_manifest_sha256=source_sha256,
        runner_identity_sha256=verify_runtime()['launch_critical_manifest_sha256'])
    bundle = dict(module=relative, sha256=marker['module_sha256'])
    marker['bundle_manifest'] = {'sha256': hashlib.sha256(canonical_json(bundle)).hexdigest()}
    environment = Environment(run_id, root, marker)
    root.mkdir(mode=0o700)
    try:
        for relative_dir in ('runtime-var', 'host-cache', 'host-config', 'host-data', 'host-tmp',
                             'compatdata/pfx/drive_c/wf0/bin', 'compatdata/pfx/drive_c/wf0/session'):
            (root / relative_dir).mkdir(parents=True, mode=0o700, exist_ok=True)
        environment.module.parent.mkdir(parents=True, mode=0o700)
        shutil.copy2(scanner, environment.scanner)
        shutil.copy2(module, environment.module)
        # Only explicit resource directories selected after inspection. Never copy
        # .wine, registry hives, activation databases or a vendor account directory.
        for source, relative_dir in resources:
            source, relative_dir = pathlib.Path(source), pathlib.PurePosixPath(relative_dir)
            if relative_dir.is_absolute() or '..' in relative_dir.parts or not source.is_dir():
                raise RuntimeError('AP8 resource source/destination differs')
            for p in source.rglob('*'):
                if p.is_symlink():
                    raise RuntimeError('AP8 resource symlink requires explicit resolution')
            shutil.copytree(source, environment.prefix / 'drive_c' / relative_dir, copy_function=shutil.copy2)
        write_atomic(root / '.wf0-owner.json', canonical_json(marker))
        verify_environment(environment, runner_identity_sha256=marker['runner_identity_sha256'])
    except Exception:
        # No process has been launched; this function created this exact root.
        shutil.rmtree(root)
        raise
    return environment


def command_vector(environment, session, component_case, mode):
    if not ((mode=='ap9-reference' and component_case=='exact-again') or (mode in (MODE,'ap8-commercial-preview','ap9-commercial') and (component_case=='first-audio' or re.fullmatch(r'class:[0-9A-Fa-f]{32}',component_case)))):
        raise RuntimeError('AP8 inspection command differs')
    vector = owner.runtime.command_vector(environment, session, 'exact-again', owner.runtime.PC0_MODE)
    vector[vector.index('--mode') + 1] = mode
    vector[vector.index('--component-case') + 1] = component_case
    vector[vector.index('--module') + 1] = 'C:\\wf0\\fixture\\' + environment.marker['module_relative'].replace('/', '\\')
    return vector


class StreamState(owner.runtime.StreamState):
    def __init__(self):
        super().__init__()
        self.vendor = bytearray()
        self.vendor_bytes = 0
        self._incoming = bytearray()

    def feed(self, name, data):
        if name != 'stdout':
            return super().feed(name, data)
        self._incoming.extend(data)
        while b'\n' in self._incoming:
            line, _, rest = self._incoming.partition(b'\n')
            self._incoming = bytearray(rest)
            if line.startswith(b'{"event":'):
                # Host records retain mandatory JSON, sequence and call pairing
                # checks. Ordinary vendor stdout is not a protocol frame.
                super().feed(name, line + b'\n')
            else:
                self.vendor_bytes += len(line) + 1
                self.vendor.extend((line + b'\n')[:max(0, 65536-len(self.vendor))])
        if len(self._incoming) > 65536:
            if self._incoming.startswith(b'{"event":'):
                raise RuntimeError('AP8 host record length bound')
            self.vendor_bytes += len(self._incoming)
            self.vendor.extend(self._incoming[:max(0, 65536-len(self.vendor))])
            self._incoming.clear()

    def accept(self, record):
        super().accept(record)
        if record.get('state') == 'ap8_call':
            self.in_flight_at = time.monotonic()
        elif record.get('state') in ('ap8_result', 'ap8_failure', 'ap8_inspection_closed'):
            self.in_flight_at = None


class ReferenceStreamState(StreamState, owner.profile.StreamState):
    """Retain AP0 activation attribution for the real reference effect."""
    pass

def inspect(environment, output, *, graphical=False, class_id=None):
    output = pathlib.Path(output)
    owner.private_directory(output)
    reporting_errors = []
    streams = StreamState()
    def checkpoint(stage, available=None, error=None):
        # Reporting cannot interrupt process cleanup.
        try:
            write_atomic(output / (environment.run_id + '-checkpoint.json'),
                canonical_json(dict(stage=stage, observation=available, error=str(error) if error else None,
                    vendor_stdout=streams.vendor.decode('utf-8', 'replace'), vendor_stdout_bytes=streams.vendor_bytes,
                    runtime_stderr=streams.stderr[-16384:].decode('utf-8', 'replace'))))
        except Exception as failure:
            reporting_errors.append(dict(stage=stage, error=str(failure)))
    profile = types.SimpleNamespace(verify_runtime=verify_runtime, verify_environment=verify_environment,
        command_vector=command_vector, StreamState=lambda: streams)
    if graphical:
        desktop = {}
        for line in subprocess.check_output(['systemctl', '--user', 'show-environment'], text=True).splitlines():
            name, separator, value = line.partition('=')
            if separator and name in ('DISPLAY', 'XAUTHORITY', 'WAYLAND_DISPLAY'):
                desktop[name] = value
        if not desktop.get('DISPLAY'):
            raise RuntimeError('existing desktop display unavailable')
        profile.controlled_environment = lambda env: {**owner.runtime.controlled_environment(env), **desktop}
    result = owner.runtime.supervise(environment, mode=MODE, component_case='class:' + class_id if class_id else 'first-audio',
        profile=profile, checkpoint=checkpoint, post_gate_seconds=45)
    result['reporting_errors'] = reporting_errors
    result['vendor_stdout'] = streams.vendor.decode('utf-8', 'replace')
    result['vendor_stdout_bytes'] = streams.vendor_bytes
    result['runtime_stderr'] = streams.stderr[-16384:].decode('utf-8', 'replace')
    try:
        write_atomic(output / (environment.run_id + '.json'), canonical_json(result))
    except Exception as failure:
        reporting_errors.append(dict(stage='final_report', error=str(failure)))
        record = dict(observation=result, error=None, reporting_errors=reporting_errors,
            containment_confirmed=all(result.get('cleanup', {}).get(k) is True
                for k in ('owned_descendants_zero', 'process_group_empty')),
            retired=False, retirement_error=None)
        raise owner.SessionError('AP8 inspection report failed', record) from failure
    # Keep the owned environment for targeted repair/readback. Retirement is an
    # explicit later operation, conditional on confirmed physical containment.
    return result
