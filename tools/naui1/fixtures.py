"""Generated source-owned bytes only; never vendor resources."""
import json
import pathlib
import struct
from common import digest, canonical

OP = 'e4143128adc87de3fdbbdfb44f186ee5'
ROOT_DIGEST = 'a'*64
APP_DIGEST = 'b'*64


def align(data):
    return data+b'\0'*(-len(data) % 4)


def version_node(key, value=b'', children=b'', text=False):
    key_bytes = (key+'\0').encode('utf-16le')
    content = align(struct.pack('<HHH', 0, len(value)//2 if text else len(value), int(text))+key_bytes)+value
    content = align(content)+children
    return struct.pack('<H', len(content))+content[2:]


def executable(product='Native Access'):
    strings = b''.join(version_node(k, (v+'\0').encode('utf-16le'), text=True) for k, v in
                       [('ProductName', product), ('ProductVersion', '3.22.0'), ('FileVersion', '3.22.0')])
    version = version_node('VS_VERSION_INFO', b'\0'*52,
                           version_node('StringFileInfo', children=version_node('040904b0', children=strings)))
    resource = bytearray(88)
    for offset in (0, 24, 48):
        struct.pack_into('<HH', resource, offset+12, 0, 1)
    struct.pack_into('<II', resource, 16, 16, 0x80000018)
    struct.pack_into('<II', resource, 40, 1, 0x80000030)
    struct.pack_into('<II', resource, 64, 1033, 72)
    struct.pack_into('<IIII', resource, 72, 0x1000+88, len(version), 0, 0)
    resource.extend(version)
    out = bytearray(512); out[:2] = b'MZ'; struct.pack_into('<I', out, 60, 64)
    out[64:68] = b'PE\0\0'; struct.pack_into('<HH', out, 68, 0x8664, 1)
    struct.pack_into('<HH', out, 84, 240, 2); struct.pack_into('<H', out, 88, 0x20b)
    struct.pack_into('<II', out, 88+112+16, 0x1000, len(resource))
    struct.pack_into('<IIII', out, 88+240+8, len(resource), 0x1000, len(resource), 512)
    return bytes(out)+bytes(resource)


def archive(package=None, main=b'const {app} = require("electron");'):
    package = package or {'name': 'native-access', 'version': '3.22.0', 'main': 'main.js'}
    pb = canonical(package)
    tree = {'files': {'package.json': {'size': len(pb), 'offset': '0'},
                      'main.js': {'size': len(main), 'offset': str(len(pb))}}}
    text = canonical(tree); hp = align(struct.pack('<I', len(text))+text)
    header = struct.pack('<I', len(hp))+hp
    return struct.pack('<II', 4, len(header))+header+pb+main


def application(base, product='Native Access', package=None, main=None):
    root = pathlib.Path(base)/'Program Files/Native Instruments/Native Access'
    (root/'resources').mkdir(parents=True)
    (root/'Native Access.exe').write_bytes(executable(product))
    (root/'resources/app.asar').write_bytes(archive(package, main) if main is not None else archive(package))
    for name in ('icudtl.dat', 'resources.pak', 'v8_context_snapshot.bin'):
        (root/name).write_bytes(('source-owned '+name).encode())
    return root


def source_records():
    binding = {'schema': 1, 'operation': OP, 'epoch': 2, 'token_sha256': 'c'*64,
               'artifact_sha256': ROOT_DIGEST, 'size': 100, 'status': 'bound', 'reason': None, 'root_ordinal': 1}
    root = {'epoch': 2, 'creation_ordinal': 1, 'windows_pid': 100, 'windows_tid': 1,
            'created_timestamp': None, 'parent_ordinal': None, 'creator_windows_pid': 50, 'creator_windows_tid': None,
            'target_root': True, 'target_tree': True, 'root_authority': 'verified_launch_adapter_exact_handle_image_and_creation_time',
            'image_identity': {'sha256': ROOT_DIGEST, 'size': 100}, 'image_request': None, 'self_exit': None}
    app = {'epoch': 2, 'creation_ordinal': 2, 'windows_pid': 200, 'windows_tid': 2,
           'created_timestamp': '10.200', 'parent_ordinal': 1, 'creator_windows_pid': 100, 'creator_windows_tid': 1,
           'target_root': False, 'target_tree': True, 'image_request': 'C:\\Program Files\\Native Access\\Native Access.exe',
           'image_identity': {'sha256': APP_DIGEST, 'authority': 'same_open_file_at_launch_request_not_mapped'}, 'self_exit': None}
    tx = {'schema': 1, 'operation': OP, 'windows_trace': {'launch_binding': binding, 'processes': [root, app], 'dropped_observations': 0},
          'diagnostics': {'runner': {'dropped_records': 0, 'dropped_bytes': 0}}, 'ledger': {'processes': []}}
    result = {'operation': OP, 'transaction': {'launch_binding': binding}}
    identity = {'application_identity': 'established', 'renderer_family': 'electron_chromium',
                'files': {'Native Access.exe': {'sha256': APP_DIGEST}}}
    return tx, result, identity, app['image_request']


def log(line, pid=200, source='gpu_process_host.cc', severity='ERROR'):
    return f'[{pid}:2:0916/123456.123:{severity}:{source}(100)] {line}\n'.encode()


def request(role='gpu-process'):
    return (f'10.100:0064:0001:trace:process:CreateProcessInternalW app L"app" cmdline L"app --type={role}", inherit 1\n'
            '10.200:0064:0001:trace:process:CreateProcessInternalW started process pid 00c8 tid 0002\n').encode()


def exited(status):
    return {'domain': 'wine_self_exit_observation', 'status': status, 'timestamp': '11.000',
            'source': 'Wine NtTerminateProcess self pseudo-handle', 'process_exiting': 1}
