"""A queued keydown must not arrive after shutdown's final releases."""
import socket
import struct
import threading
import time
import unittest

from gateway import ForwardingTarget, modifier_releases


class DelayedSocket:
    def __init__(self, connection):
        self.connection = connection
        self.entered = threading.Event()
        self.proceed = threading.Event()

    def __getattr__(self, name):
        return getattr(self.connection, name)

    def send(self, data):
        self.entered.set()
        if not self.proceed.wait(2):
            raise RuntimeError("Test write did not resume")
        return self.connection.send(data)


class ShutdownTest(unittest.TestCase):
    def test_inflight_keydown_precedes_releases_and_later_write_is_refused(self):
        sending, receiving = socket.socketpair()
        self.addCleanup(sending.close)
        self.addCleanup(receiving.close)
        receiving.settimeout(2)
        delayed = DelayedSocket(sending)
        target = ForwardingTarget(delayed)
        down = struct.pack(">BBHI", 4, 1, 0, 0xffe1)
        worker = threading.Thread(target=target.send, args=(down,))
        worker.start()
        self.assertTrue(delayed.entered.wait(2))
        closing = threading.Thread(target=target.finish)
        closing.start()
        delayed.proceed.set()
        worker.join(2)
        closing.join(2)
        self.assertFalse(worker.is_alive())
        self.assertFalse(closing.is_alive())
        with self.assertRaises(OSError):
            target.send(down)
        received = bytearray()
        while block := receiving.recv(4096):
            received.extend(block)
        self.assertEqual(received, down + modifier_releases)

    def test_backpressure_does_not_leave_shutdown_waiting_for_writer(self):
        sending, receiving = socket.socketpair()
        self.addCleanup(sending.close)
        self.addCleanup(receiving.close)
        sending.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4096)
        sending.setblocking(False)
        try:
            while True:
                sending.send(b"x" * 65536)
        except BlockingIOError:
            pass
        sending.setblocking(True)
        delayed = DelayedSocket(sending)
        delayed.proceed.set()
        target = ForwardingTarget(delayed)
        errors = []
        def write():
            try:
                target.send(b"keydown")
            except OSError as error:
                errors.append(error)
        worker = threading.Thread(target=write, daemon=True)
        worker.start()
        self.assertTrue(delayed.entered.wait(2))
        closing = threading.Thread(target=target.finish, daemon=True)
        started = time.monotonic()
        closing.start()
        closing.join(2.8)
        worker.join(.1)
        self.assertFalse(worker.is_alive())
        self.assertFalse(closing.is_alive())
        self.assertLess(time.monotonic() - started, 2.8)
        self.assertTrue(errors)
        self.assertTrue(target.stopping)


if __name__ == "__main__":
    unittest.main()
