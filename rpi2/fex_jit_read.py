"""Read only source-owned scalar JIT blocks from one live FEX fixture process."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re


RVAS = tuple(range(0x1460, 0x1761, 0x20))
MAX_BLOCK = 1024


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--candidate', choices=('A', 'B'), required=True)
    parser.add_argument('--map', type=Path, required=True)
    parser.add_argument('--module', type=Path, required=True)
    parser.add_argument('--expected-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()

    proc = Path('/proc') / str(args.pid)
    stat = (proc / 'stat').read_text()
    start_ticks = int(stat[stat.rfind(')') + 2:].split()[19])
    maps = (proc / 'maps').read_text().splitlines()
    module_rows = [line for line in maps if str(args.module) in line]
    if not module_rows or hashlib.sha256(args.module.read_bytes()).hexdigest() != args.expected_sha256:
        raise RuntimeError('mapped FEX module identity mismatch')
    module_inode = args.module.stat().st_ino
    if any(int(row.split()[4]) != module_inode for row in module_rows):
        raise RuntimeError('mapped FEX module inode mismatch')

    executable = []
    for row in maps:
        fields = row.split(maxsplit=5)
        if len(fields) == 5 and fields[1] == 'rwxp':
            begin, end = (int(value, 16) for value in fields[0].split('-'))
            executable.append((begin, end))
    if not executable:
        raise RuntimeError('anonymous executable JIT mapping unavailable')

    labels = args.map.read_text().splitlines()
    args.output.mkdir(exist_ok=False)
    rows = []
    fd = os.open(proc / 'mem', os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        for rva in RVAS:
            suffix = f'fex_scalar_fixture.exe+0x{rva:x} '
            matching = [line for line in labels if suffix in line]
            if len(matching) != 1:
                raise RuntimeError(f'guest block identity ambiguous at 0x{rva:x}')
            address_text, size_text, *_ = matching[0].split()
            address, size = int(address_text, 16), int(size_text, 16)
            if not 0 < size <= MAX_BLOCK or not any(
                begin <= address and address + size <= end for begin, end in executable
            ):
                raise RuntimeError(f'JIT block range invalid at 0x{rva:x}')
            data = os.pread(fd, size, address)
            if len(data) != size:
                raise RuntimeError(f'short JIT read at 0x{rva:x}')
            (args.output / f'{rva:04x}.bin').write_bytes(data)
            rows.append({'rva': f'0x{rva:x}', 'size': size,
                         'sha256': hashlib.sha256(data).hexdigest()})
    finally:
        os.close(fd)

    after = (proc / 'stat').read_text()
    if int(after[after.rfind(')') + 2:].split()[19]) != start_ticks:
        raise RuntimeError('process identity changed during JIT read')
    (args.output / 'manifest.json').write_text(json.dumps({
        'candidate': args.candidate,
        'module_sha256': args.expected_sha256,
        'pid': args.pid,
        'process_start_ticks': start_ticks,
        'blocks': rows,
    }, indent=2) + '\n')
    print(f'JIT_{args.candidate}_CAPTURED {len(rows)} blocks')


if __name__ == '__main__':
    main()
