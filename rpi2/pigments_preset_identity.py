"""Read-only identity observation for the pinned Pigments 7.0.1 fixture.

This is an experiment parser, not a vendor preset API. Never rewrite state.
Unknown framing, disagreeing component/controller names, or damaged envelopes
stop the sweep. Names are labels, not durable plug-in/content identities.
"""
import hashlib
import re
import struct

MODULE = bytes.fromhex('bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07')
CLASS = bytes.fromhex('41727475415649534b61743150726f63')
PREFIX = b'22 serialization::archive 10 0 7 0 7 '


def metadata(chunk):
    if not chunk.startswith(PREFIX):
        raise ValueError('Unknown Pigments state framing')
    cursor = len(PREFIX)
    labels = []
    for _ in range(2):  # length-prefixed preset name and bank
        match = re.match(rb'([0-9]{1,3}) ', chunk[cursor:cursor + 5])
        if not match:
            raise ValueError('Missing label length')
        count = int(match[1])
        cursor += match.end()
        if not 1 <= count <= 255 or cursor + count >= len(chunk):
            raise ValueError('Label extent')
        label = chunk[cursor:cursor + count].decode('utf-8')
        if any(ord(c) < 32 for c in label) or chunk[cursor + count:cursor + count + 1] != b' ':
            raise ValueError('Label framing')
        labels.append(label)
        cursor += count + 1
    return dict(name=labels[0], bank=labels[1])


def identify(blob):
    if not 120 <= len(blob) <= 104 + (1 << 20):
        raise ValueError('State size')
    if blob[:8] != b'LVBSTATE' or struct.unpack_from('<II', blob, 8) != (3, 104):
        raise ValueError('State envelope')
    if blob[16:32] != CLASS or blob[32:64] != MODULE:
        raise ValueError('Different plug-in identity')
    if struct.unpack_from('<II', blob, 64) != (len(blob) - 104, 0):
        raise ValueError('State extent')
    payload = blob[104:]
    if hashlib.sha256(payload).digest() != blob[72:104]:
        raise ValueError('State checksum')
    a, b, n, flags = struct.unpack_from('<IIII', payload)
    if flags != 3 or n != 4446 or 16 + a + b + n * 16 != len(payload):
        raise ValueError('Commercial state extent')
    component = metadata(payload[16:16 + a])
    controller = metadata(payload[16 + a:16 + a + b])
    if component != controller:
        raise ValueError('Component/controller preset disagreement')
    return component
