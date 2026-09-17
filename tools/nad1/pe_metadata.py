"""PE identity without treating optional vendor version strings as execution authority."""
import struct
from common import Refusal, require
from identity import at, pe

# A version field may use a representation outside the strict NAUI1 reader's
# admitted text contract. Preserve exact bytes through the image digest; do not
# reinterpret binary fields as strings or change the accepted NAUI1 reader.
OPTIONAL_REFUSALS = frozenset({'version_string_type'})

def metadata(stream):
    dos = at(stream, 0, 64)
    require(dos[:2] == b'MZ', 'not_pe')
    offset = struct.unpack_from('<I', dos, 60)[0]
    header = at(stream, offset, 24)
    require(header[:4] == b'PE\0\0', 'not_pe')
    machine, count = struct.unpack_from('<HH', header, 4)
    length, flags = struct.unpack_from('<HH', header, 20)
    require(machine in (0x14c, 0x8664) and 1 <= count <= 96
            and 96 <= length <= 512 and flags & 2 and not flags & 0x2000,
            'unsupported_pe')
    optional = at(stream, offset + 24, length)
    require((machine, struct.unpack_from('<H', optional)[0]) in
            ((0x14c, 0x10b), (0x8664, 0x20b)), 'pe_architecture')
    architecture = 'x64' if machine == 0x8664 else 'x86'
    try:
        value = pe(stream)
    except Refusal as error:
        if str(error) not in OPTIONAL_REFUSALS:
            raise
        return {'architecture': architecture, 'version': {},
                'version_status': 'unavailable', 'version_reason': str(error)}
    require(value['architecture'] == architecture, 'pe_architecture_changed')
    return dict(value, version_status='observed' if value['version'] else 'absent',
                version_reason=None)
