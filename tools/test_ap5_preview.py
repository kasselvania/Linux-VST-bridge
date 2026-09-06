"""Concurrent preview ownership; no Windows process or DAW is launched."""
import contextlib
import socket
import threading
import unittest
from unittest.mock import patch
import ap4_preview as p


class IndependentOwners(unittest.TestCase):
    def test_sibling_failure_cleanup_and_capacity(self):
        started = [threading.Event() for _ in range(5)]
        finish = [threading.Event() for _ in range(5)]
        ended = [threading.Event() for _ in range(5)]
        peers = {}
        def run(peer):
            index = peers[peer.fileno()]
            started[index].set()
            self.assertTrue(finish[index].wait(5))
            ended[index].set()
            if index == 0:
                raise RuntimeError('one instance failed')
        sessions = p.Sessions(run)
        with contextlib.ExitStack() as stack:
            clients = []
            for index in range(4):
                a, b = socket.socketpair(); stack.enter_context(b)
                peers[a.fileno()] = index; clients.append(b)
                self.assertTrue(sessions.admit(a))
                self.assertTrue(started[index].wait(2))
            a, b = socket.socketpair(); stack.enter_context(b)
            self.assertFalse(sessions.admit(a)); self.assertEqual(b.recv(1), b'')
            # A failed/closing sibling does not revoke another connection.
            with patch('sys.stderr'):
                finish[0].set(); self.assertTrue(ended[0].wait(2))
                sessions.threads[0].join(2)
            self.assertEqual(clients[0].recv(1), b'')
            self.assertFalse(sessions.blocked.is_set())
            self.assertFalse(ended[1].is_set())
            a, b = socket.socketpair(); stack.enter_context(b); peers[a.fileno()] = 4
            self.assertTrue(sessions.admit(a)); self.assertTrue(started[4].wait(2))
            for event in finish: event.set()
            sessions.join()
            self.assertEqual(b.recv(1), b'')

    def test_uncertain_containment_refuses_new_work_without_stopping_sibling(self):
        release = threading.Event(); active = threading.Event()
        def run(peer):
            if peer.recv(1) == b'f':
                raise p.ContainmentError('owned cleanup failed; retained')
            active.set(); release.wait(5)
        sessions = p.Sessions(run)
        with contextlib.ExitStack() as stack:
            a, b = socket.socketpair(); stack.enter_context(b); b.sendall(b'h')
            self.assertTrue(sessions.admit(a)); self.assertTrue(active.wait(2))
            c, d = socket.socketpair(); stack.enter_context(d); d.sendall(b'f')
            with patch('sys.stderr'):
                self.assertTrue(sessions.admit(c)); sessions.threads[-1].join(2)
            self.assertTrue(sessions.blocked.is_set())
            b.setblocking(False)
            with self.assertRaises(BlockingIOError): b.recv(1)
            e, f = socket.socketpair(); stack.enter_context(f)
            self.assertFalse(sessions.admit(e)); self.assertEqual(f.recv(1), b'')
            release.set(); sessions.join()


if __name__ == '__main__': unittest.main()
