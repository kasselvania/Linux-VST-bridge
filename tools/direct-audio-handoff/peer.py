#!/usr/bin/env python3
"""Linux counterpart for the standalone Wine shared-futex probe; no audio/SDK."""
import ctypes
import mmap
import os
from pathlib import Path
import sys
import time

class Timespec(ctypes.Structure):
    _fields_ = [('sec', ctypes.c_long), ('nsec', ctypes.c_long)]

def run(path):
    if sys.platform != 'linux' or os.uname().machine != 'x86_64':
        raise RuntimeError('Linux x86-64 peer required')
    libc = ctypes.CDLL(None, use_errno=True)
    libc.syscall.restype = ctypes.c_long
    with Path(path).open('r+b') as source, mmap.mmap(source.fileno(), 4096) as mapping:
        words = (ctypes.c_uint32 * 4).from_buffer(mapping)
        base = ctypes.addressof(words)
        def futex(index, operation, value, deadline_ns=None):
            deadline = Timespec(divmod(deadline_ns, 1000000000)[0], divmod(deadline_ns, 1000000000)[1]) if deadline_ns else None
            pointer = ctypes.byref(deadline) if deadline else ctypes.c_void_p()
            result = libc.syscall(ctypes.c_long(202), ctypes.c_void_p(base + index * 4),
                                  ctypes.c_int(operation), ctypes.c_uint32(value), pointer,
                                  ctypes.c_void_p(), ctypes.c_uint32(0xffffffff))
            return result if result >= 0 else -ctypes.get_errno()
        wine_wakes = 0
        try:
            startup_deadline = time.monotonic_ns() + 60000000000
            while words[2] != 1:
                if time.monotonic_ns() >= startup_deadline:
                    raise RuntimeError('Wine probe readiness expired')
                time.sleep(0.001)
            time.sleep(0.01) # first Wine wait must actually be sleeping
            for ticket in range(1, 1001):
                words[0] = ticket
                woke = futex(0, 1, 1)
                if woke < 0 or (ticket == 1 and woke != 1):
                    raise RuntimeError('native-to-Wine sleeping-wait proof failed')
                wine_wakes += woke
                deadline = time.monotonic_ns() + 3000000000
                while True:
                    observed = words[1]
                    if observed == ticket:
                        break
                    if observed != ticket - 1 or time.monotonic_ns() >= deadline:
                        raise RuntimeError('native reply ticket/deadline refusal')
                    result = futex(1, 9, ticket - 1, deadline)
                    if result not in (0, -11, -4):
                        raise RuntimeError('native wait refused: ' + str(result))
            print('native_peer_ok exchanges=1000 native_woke_wine=' + str(wine_wakes), flush=True)
            return wine_wakes
        except BaseException:
            words[3] = 1
            futex(0, 1, 1)
            futex(1, 1, 1)
            raise
        finally:
            del words

if __name__ == '__main__':
    run(sys.argv[1])
