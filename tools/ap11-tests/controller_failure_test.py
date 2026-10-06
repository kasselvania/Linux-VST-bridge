"""Generated peer for production Windows mapped processing, no vendor payloads."""
import ctypes
import mmap
import pathlib
import socket
import struct
import subprocess
import sys
import tempfile
import time
import uuid

kernel = ctypes.WinDLL("kernel32", use_last_error=True)
kernel.CreateEventW.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_wchar_p]
kernel.CreateEventW.restype = ctypes.c_void_p
kernel.SetEvent.argtypes = [ctypes.c_void_p]
kernel.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
kernel.CloseHandle.argtypes = [ctypes.c_void_p]
exe = pathlib.Path(sys.argv[1]).resolve()

def write(path, data):
    path.write_bytes(data)

def until(check):
    deadline = time.monotonic() + 4
    while not check():
        assert time.monotonic() < deadline, "fixture condition timed out"
        time.sleep(.001)

def scenario(mode):
    identity = uuid.uuid4().bytes
    event_name = "Local\\lvb-controller-test-" + identity.hex()
    entered = kernel.CreateEventW(None, 1, 0, event_name + "-entered")
    release = kernel.CreateEventW(None, 1, 0, event_name + "-release")
    assert entered and release
    try:
        with tempfile.TemporaryDirectory(prefix="lvb-controller-") as temp, socket.socket() as listener:
            root = pathlib.Path(temp)
            listener.bind(("127.0.0.1", 0)); listener.listen(1); listener.settimeout(8)
            write(root / "ap1.control", struct.pack("<HH", listener.getsockname()[1], 0) + identity + bytes(32))
            audio = bytearray(4192)
            struct.pack_into("<8I", audio, 0, 0x4d315041, 1, 256, 2, 4192, 64, 2128, 1032)
            struct.pack_into("<Q", audio, 56, 0x8d396b274e105ac3 ^ 1)
            for offset in [64, 1096, 2128, 3160]:
                struct.pack_into("<I", audio, offset, 0x4b123456)
                struct.pack_into("<I", audio, offset + 1028, 0x4b123456)
            struct.pack_into("<256f", audio, 68, *([.25] * 256))
            struct.pack_into("<256f", audio, 1100, *([-.5] * 256))
            write(root / "ap1.audio", audio)
            for file, magic, extent in [("ap12.status", b"LVFS", 1024), ("if1.terminal", b"LVIF", 2048)]:
                data = bytearray(extent); data[:4] = magic
                struct.pack_into("<II", data, 4, 1, extent); data[16:32] = identity
                if file == "if1.terminal":
                    # A generated stable native context, not claimed as a DAW.
                    struct.pack_into("<Q", data, 1024, 1)
                    struct.pack_into("<Q", data, 1344, 1)
                write(root / file, data)
            with (root / "output.txt").open("w+") as log:
                child = subprocess.Popen([str(exe), mode, str(root), identity.hex(), event_name], stdout=log, stderr=log)
                try:
                    with listener.accept()[0] as peer, (root / "ap1.audio").open("r+b") as af:
                        peer.settimeout(6)
                        with mmap.mmap(af.fileno(), 0) as samples:
                            def send(kind, seq=1, payload=b""):
                                peer.sendall(struct.pack("<IHHHHI16sQQQ", 0x3141504c, 1, 5, kind, 0, len(payload), identity, 1, seq, 0) + payload)
                            def read_exact(n):
                                result = b""
                                while len(result) < n:
                                    block = peer.recv(n - len(result)); assert block, "unexpected closed peer"
                                    result += block
                                return result
                            def receive(expected=None):
                                h = struct.unpack("<IHHHHI16sQQQ", read_exact(56))
                                assert h[:3] == (0x3141504c, 1, 5) and h[6] == identity
                                payload = read_exact(h[5])
                                if expected is not None: assert h[3] == expected, (h[3], expected)
                                return h[3], h[8], payload
                            receive(1); send(1, 0); receive(2)
                            send(8, payload=struct.pack("<II", 256, 0)); receive(9)
                            send(10, payload=struct.pack("<Q", 1)); receive(11)
                            def process(seq, automation=False):
                                events = struct.pack("<IIIhhd f I", 0, 2, 42, 0, 0, .25, 0., 0) if automation else b""
                                payload = struct.pack("<IIIIdIIQQII", 256, 64, 2128, 1032, 0., 0, 0, 1, (seq-1)*256, int(automation), 0) + events
                                send(3, seq, payload)
                            def done():
                                assert struct.unpack_from("<256f", samples, 2132) == (.125,) * 256
                                assert struct.unpack_from("<256f", samples, 3164) == (-.25,) * 256
                            process(1, True); receive(4); done()
                            assert kernel.WaitForSingleObject(entered, 4000) == 0
                            if mode == "slow":
                                for seq in range(2, 10): process(seq); receive(4); done()
                                assert kernel.SetEvent(release)
                                process(10); receive(4); done()
                                send(12, 11, struct.pack("<Q", 1)); receive(13)
                                send(14, 11); receive(15); send(5, 11); receive(6)
                            elif mode.endswith("state"):
                                send(16, 2)
                                def state_waiting():
                                    status = (root / "ap12.status").read_bytes()
                                    counter = struct.unpack_from("<Q", status, 384)[0]
                                    return struct.unpack_from("<Q", status, 448 + (counter & 1)*128 + 32)[0] == 7
                                until(state_waiting)
                                assert kernel.SetEvent(release); receive(7)
                            else:
                                assert kernel.SetEvent(release)
                                until(lambda: struct.unpack_from("<Q", (root / "if1.terminal").read_bytes(), 64)[0] == 2)
                                # One already admitted request may finish. No
                                # subsequent request is allowed after refusal.
                                process(2)
                                kind, _, _ = receive()
                                if kind == 4: done(); receive(7)
                                else: assert kind == 7
                    assert child.wait(timeout=8) == 0
                    terminal = (root / "if1.terminal").read_bytes()
                    assert struct.unpack_from("<Q", terminal, 64)[0] == (0 if mode == "slow" else 2)
                    if mode != "slow": assert struct.unpack_from("<Q", terminal, 384 + 15*8)[0] == 1
                    print(f"{mode}: production processing, captured samples, controller status and retirement passed")
                except Exception:
                    kernel.SetEvent(release)
                    if child.poll() is None: child.kill()
                    child.wait(timeout=5); log.seek(0); print(log.read()[-12000:])
                    raise
    finally:
        kernel.CloseHandle(entered); kernel.CloseHandle(release)

for mode in ["slow", "refuse-state", "throw-state", "refuse-audio", "throw-audio"]:
    scenario(mode)
