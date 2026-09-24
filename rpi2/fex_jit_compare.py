"""Summarize only source-owned A/B scalar JIT instruction bodies."""

import argparse
from collections import Counter
import json
from pathlib import Path
import re
import subprocess


FUNCTIONS = (
    'add', 'sub', 'mul', 'div',
    'dadd', 'dsub', 'dmul', 'ddiv',
)
FORMS = ('legacy', 'alias', 'live')
LINE = re.compile(r'^\s*([0-9a-f]+):\s+[0-9a-f]{8}\s+(\S+)(?:\s+(.*))?$')


def body(path):
    output = subprocess.check_output(
        ['aarch64-linux-gnu-objdump', '-D', '-b', 'binary', '-m', 'aarch64', str(path)],
        text=True,
    )
    instructions = []
    for line in output.splitlines():
        match = LINE.match(line)
        if not match:
            continue
        mnemonic, operands = match.group(2), match.group(3) or ''
        instructions.append((mnemonic, operands.split(' ;', 1)[0]))
        if mnemonic == 'ret':
            break
    if not instructions or instructions[-1][0] != 'ret':
        raise RuntimeError(f'JIT body does not terminate cleanly: {path}')
    return instructions


def counts(instructions):
    out = Counter(total_instructions=len(instructions), full_vector_moves=0,
                  scalar_result_inserts=0, vector_stack_spill_reload=0)
    for mnemonic, operands in instructions:
        if mnemonic in ('mov', 'orr') and re.search(r'v\d+\.(?:16b|2d|4s)', operands) and '[' not in operands:
            out['full_vector_moves'] += 1
        if mnemonic in ('mov', 'ins') and re.search(r'v\d+\.[sd]\[0\]', operands):
            out['scalar_result_inserts'] += 1
        if mnemonic in ('str', 'stur', 'stp', 'ldr', 'ldur', 'ldp') and re.search(r'\b[qsdev]\d+\b', operands) and '[sp' in operands:
            out['vector_stack_spill_reload'] += 1
    return dict(out)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('a', type=Path)
    parser.add_argument('b', type=Path)
    args = parser.parse_args()
    rows = []
    aggregate = {'A': Counter(), 'B': Counter()}
    for index, (operation, form) in enumerate(
        [(operation, form) for operation in FUNCTIONS for form in FORMS]
        + [('add', 'pressure')]
    ):
        rva = 0x1460 + index * 0x20
        a = body(args.a / f'{rva:04x}.bin')
        b = body(args.b / f'{rva:04x}.bin')
        ac, bc = counts(a), counts(b)
        aggregate['A'].update(ac)
        aggregate['B'].update(bc)
        rows.append({'rva': f'0x{rva:x}', 'operation': operation, 'form': form,
                     'A': ac, 'B': bc, 'instruction_body_equal': a == b})
    print(json.dumps({
        'source_fixture_blocks': len(rows),
        'aggregate': {candidate: dict(counts) for candidate, counts in aggregate.items()},
        'changed_instruction_bodies': sum(not row['instruction_body_equal'] for row in rows),
        'rows': rows,
    }, indent=2))


if __name__ == '__main__':
    main()
