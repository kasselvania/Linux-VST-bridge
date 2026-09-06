"""Focused owner lifetime/startup/cleanup regressions; no Windows launches."""
import contextlib
import json
import pathlib
import socket
import tempfile
import threading
import time
import types
import unittest
from unittest.mock import MagicMock, patch
import ap4_preview as p

CLEAN = {'owned_descendants_zero': True, 'process_group_empty': True}

class PreviewTests(unittest.TestCase):
    def test_idle_lease_and_disconnect_or_explicit_stop(self):
        a, b = socket.socketpair()
        with a, b:
            stopped = False
            connection = p.Connection(a, lambda: stopped)
            with patch.object(p.time, 'monotonic', return_value=500):
                self.assertFalse(connection.stopped())
            b.close()
            with patch.object(p.time, 'monotonic', return_value=600):
                self.assertFalse(connection.stopped())
            with patch.object(p.time, 'monotonic', return_value=603):
                self.assertTrue(connection.stopped())
        a, b = socket.socketpair()
        with a, b:
            connection = p.Connection(a, lambda: True)
            self.assertTrue(connection.stopped())

    def test_normal_connection_reuses_supervisor_without_daw_or_expiry(self):
        self.connection_case(False)

    def test_failed_containment_retains_stage_and_closes_lease(self):
        self.connection_case(True)

    def connection_case(self, cleanup_failure):
        with tempfile.TemporaryDirectory() as temp:
            root = pathlib.Path(temp); session = root/'session'; session.mkdir()
            output = root/'results'; output.mkdir()
            environment = types.SimpleNamespace(session=session, run_id='a'*32)
            a, b = socket.socketpair()
            native_errors = []
            def native():
                try:
                    with b:
                        b.sendall(b'AP4\n'); size = int.from_bytes(b.recv(2), 'little')
                        data = b''
                        while len(data)<size: data += b.recv(size-len(data))
                        token, directory = data.decode().split('\n')
                        self.assertEqual(len(token),32); self.assertEqual(pathlib.Path(directory),session)
                        (session/'ap1.control').write_bytes(b'ready')
                        if not cleanup_failure: self.assertEqual(b.recv(1), b'R')
                        self.assertEqual(b.recv(1), b'')
                except Exception as error: native_errors.append(error)
            thread = threading.Thread(target=native); thread.start()
            retire = MagicMock()
            def supervise(env, **kw):
                self.assertIsNone(kw['post_gate_seconds']); self.assertFalse(kw['stop_requested']())
                self.assertIs(env,environment); self.assertNotIn('caller_command',kw)
                cleanup = dict(CLEAN, process_group_empty=not cleanup_failure)
                observed = {'cleanup': cleanup, 'raw_exit': 0, 'records': []}
                (session/'ap3-gui-report.jsonl').write_text('{"gain":0.25}\n')
                kw['checkpoint']('supervisor_containment',observed)
                return observed
            with patch.object(p.runtime,'supervise',side_effect=supervise):
                if cleanup_failure:
                    with self.assertRaisesRegex(RuntimeError,'containment incomplete'):
                        p.serve(a,lambda:environment,retire,output,lambda:False)
                else: p.serve(a,lambda:environment,retire,output,lambda:False)
            thread.join(3); self.assertFalse(thread.is_alive()); self.assertFalse(native_errors,native_errors)
            self.assertEqual(retire.call_count,0 if cleanup_failure else 1)
            result=json.loads((output/('a'*32+'.json')).read_text())
            self.assertEqual(result['retired'],not cleanup_failure)
            self.assertIn('0.25',result['native_report'])

    def test_startup_timeout_and_bad_greeting_never_launch(self):
        with tempfile.TemporaryDirectory() as temp:
            env=types.SimpleNamespace(session=pathlib.Path(temp))
            connection=types.SimpleNamespace(stopped=lambda:False)
            with self.assertRaises(TimeoutError): p.wait_control(env,connection,seconds=0)
            a,b=socket.socketpair()
            with b:
                b.sendall(b'bad!')
                create=MagicMock()
                with self.assertRaisesRegex(RuntimeError,'greeting differs'):
                    p.serve(a,create,MagicMock(),pathlib.Path(temp),lambda:False)
                create.assert_not_called(); self.assertEqual(b.recv(1),b'')

    def test_actual_supervisor_survives_180_seconds_then_uses_owned_cleanup(self):
        d=p.runtime
        with tempfile.TemporaryDirectory() as temp, contextlib.ExitStack() as stack:
            session=pathlib.Path(temp)
            env=types.SimpleNamespace(session=session,run_id='a'*32,marker={'fixture':'again'})
            root=MagicMock(pid=123,returncode=0);root.poll.return_value=None
            selector=MagicMock(); selector.get_map.return_value={}
            now=0
            def pump(selector,streams,timeout):
                nonlocal now
                if not streams.records:
                    (session/('b'*32+'.ready')).write_bytes(b'ready')
                    streams.records.append({'event':'lifecycle','state':'readiness_announced'})
                now+=200
            for name,value in {'verify_environment':None,'verify_diagnostic_runner':{'launch_critical_manifest_sha256':'c'*64},
                'handshake':b'ready','command_vector':['inert-test-command'],'controlled_environment':{},
                'process_identity':{'pid':123,'start_ticks':1},'descendants':[],'topology':{},'protected_snapshot':{}}.items():
                stack.enter_context(patch.object(d,name,return_value=value))
            cleanup=stack.enter_context(patch.object(d,'cleanup_process',return_value=CLEAN))
            stack.enter_context(patch.object(d.subprocess,'Popen',return_value=root))
            stack.enter_context(patch.object(d.selectors,'DefaultSelector',return_value=selector))
            stack.enter_context(patch.object(d,'pump',side_effect=pump))
            stack.enter_context(patch.object(d.time,'monotonic',side_effect=lambda:now))
            result=d.supervise(env,mode=d.PC0_MODE,session_override='b'*32,
                post_gate_seconds=None,stop_requested=lambda:now>=800)
            self.assertEqual(now,800)
            self.assertNotEqual(result['classification'],'stage_timeout')
            cleanup.assert_called_once_with(root,[(123,1)])
            self.assertEqual(result['cleanup'],CLEAN)

if __name__=='__main__': unittest.main()
