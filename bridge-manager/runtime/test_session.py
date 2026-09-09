"""Installed supervision tests. Real child processes; no vendor qualification."""
import hashlib
import json
import os
import pathlib
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import ownership
import session


@unittest.skipUnless(sys.platform == "linux", "PID/start tracking uses Linux procfs")
class OwnershipTests(unittest.TestCase):
    def test_subtree_tracking_covers_thread_children_and_reparented_descendants(self):
        # Create a descendant from a non-main thread, then let that parent exit.
        # The observed child remains owned after the observed parent exits.
        program = """import subprocess,threading,sys,time,os
p=None
ready=threading.Event()
release=threading.Event()
def launch():
 global p
 p=subprocess.Popen([sys.executable,'-u','-c', 'import time;time.sleep(30)'])
 ready.set();release.wait()
t=threading.Thread(target=launch);t.start();ready.wait()
print(p.pid,flush=True)
sys.stdin.readline();release.set();t.join()
"""
        root=subprocess.Popen([sys.executable,'-u','-c',program],stdin=subprocess.PIPE,stdout=subprocess.PIPE,start_new_session=True)
        sibling=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
        tracker=ownership.ProcessTracker(root.pid)
        child=int(root.stdout.readline())
        try:
            observed=tracker.update()
            expected={(r['pid'],r['start_ticks']) for r in ownership.descendant_identities(root.pid)}
            self.assertTrue(expected.issubset(observed))
            self.assertTrue(any(pid==child for pid,start in observed))
            self.assertFalse(any(pid==sibling.pid for pid,start in observed))
            root.stdin.write(b'go\n');root.stdin.flush();root.wait(timeout=3)
            # Remembered identities, even after their creator has gone away.
            self.assertTrue(observed.issubset(tracker.update()))
            self.assertTrue(all(ownership.cleanup_process(root,sorted(tracker.owned)).values()))
            self.assertIsNone(sibling.poll())
        finally:
            ownership.cleanup_process(root,sorted(tracker.owned))
            root.stdin.close();root.stdout.close()
            sibling.terminate();sibling.wait(timeout=3)

    def test_cleanup_preserves_independent_sibling(self):
        first = subprocess.Popen(["/bin/sleep", "30"], start_new_session=True)
        sibling = subprocess.Popen(["/bin/sleep", "30"], start_new_session=True)
        try:
            census = ownership.process_identities()
            owned = [(p['pid'], p['start_ticks']) for p in census if p['pid'] == first.pid]
            self.assertEqual(len(owned), 1)
            self.assertTrue(all(ownership.cleanup_process(first, owned).values()))
            self.assertIsNone(sibling.poll())
            self.assertFalse(any(p['pid'] == first.pid for p in ownership.process_identities()))
        finally:
            for child in (first, sibling):
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=5)

    def test_terminal_sdk_failure_does_not_wait_for_launcher_companion(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            sid = "ab" * 16
            directory = root / "compatdata/pfx/drive_c/bridge/sessions" / sid
            directory.mkdir(parents=True, mode=0o700)
            artifact = root / "host"
            artifact.write_bytes(b"exact-test-image")
            binding = {'path': str(artifact), 'sha256': hashlib.sha256(artifact.read_bytes()).hexdigest()}
            spec = {'registration': {'host': binding, 'module': binding,
                'environment': {'root': str(root), 'runner': {'files': []}}},
                'session': sid, 'directory': str(directory), 'report': str(root / "report.json"), 'inspect': True}
            program = "import time; print('{\"event\":\"lifecycle\",\"state\":\"ap8_inspection_closed\",\"exit_code\":90}',flush=True); time.sleep(30)"
            started = time.monotonic()
            with patch.object(session, "command", return_value=([sys.executable, "-c", program], b"")), \
                 patch.object(session, "environment", return_value=os.environ.copy()):
                result = session.run(spec)
            self.assertLess(time.monotonic() - started, 4)
            self.assertEqual(result['error'], 'Windows SDK host failed: 90')
            self.assertIsNone(result['exit_before_cleanup'])
            self.assertEqual(result['raw_exit'], -signal.SIGTERM)
            self.assertTrue(result['cleanup_confirmed'])
            self.assertTrue(result['transport_retired'])
            self.assertFalse(directory.exists())
            self.assertTrue((root / "report.json").exists())

    def test_early_failure_wakes_native_and_retires_only_after_release(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            sid = "cd" * 16
            directory = root / "compatdata/pfx/drive_c/bridge/sessions" / sid
            directory.mkdir(parents=True, mode=0o700)
            (directory / "ap1.control").touch()
            artifact = root / "host"
            artifact.write_bytes(b"image")
            binding = {'path': str(artifact), 'sha256': hashlib.sha256(b"image").hexdigest()}
            spec = {'registration': {'host': binding, 'module': binding,
                'environment': {'root': str(root), 'runner': {'files': []}}},
                'session': sid, 'directory': str(directory), 'report': str(root / "report.json"),
                'inspect': False, 'binding_sent': True}
            native, owner = socket.socketpair()
            observations = []
            def consumer():
                native.settimeout(5)
                observations.append(native.recv(1))
                observations.append(directory.exists())
                native.shutdown(socket.SHUT_WR)
                observations.append(native.recv(1))
            thread = threading.Thread(target=consumer)
            thread.start()
            try:
                with patch.object(session, "command", return_value=([sys.executable, "-c", "raise SystemExit(90)"], b"")), \
                     patch.object(session, "environment", return_value=os.environ.copy()):
                    result = session.run(spec, owner)
                thread.join(timeout=6)
                self.assertFalse(thread.is_alive())
                self.assertEqual(observations, [b'F', True, b'R'])
                self.assertTrue(result['transport_retired'])
                self.assertFalse(directory.exists())
            finally:
                native.close()
                owner.close()

    def test_report_failure_does_not_skip_retirement_or_kill_sibling(self):
        sibling=subprocess.Popen(["/bin/sleep","30"],start_new_session=True)
        atomic=session.atomic
        receipts=[]
        def fail_report(path,value):
            if path.name in ('report.json','report.fault.json'):raise OSError('injected rich report failure')

            if path.name.endswith('.ownership.json'):receipts.append(value.copy())
            return atomic(path,value)
        try:
            with patch.object(session,'atomic',side_effect=fail_report):
                self.test_early_failure_wakes_native_and_retires_only_after_release()
            self.assertIsNone(sibling.poll())
            self.assertEqual(len(receipts),1)
            self.assertTrue(receipts[0]['cleanup_confirmed'])
            self.assertTrue(receipts[0]['transport_retired'])
            self.assertIn('injected rich report failure',receipts[0]['reporting_error'])
        finally:
            sibling.terminate();sibling.wait(timeout=5)


@unittest.skipUnless(sys.platform == "linux", "Cross-process atomic status requires Linux libatomic")
class FaultStatusTests(unittest.TestCase):
    def test_editor_removal_fault_survives_transport_retirement(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);sid='12'*16
            data=bytearray(1024);data[:32]=b'LVFS'+session.struct.pack('<III',1,1024,0)+bytes.fromhex(sid)
            (root/'ap12.status').write_bytes(data);(root/'ap12.status').chmod(0o600)
            observer=session.FaultStatus(root,sid)
            try:
                self.assertEqual(observer.snapshot()['editor'],{'available':False})
                extent=256+2*512*584;gui=bytearray(extent)
                gui[:36]=b'LVBU'+session.struct.pack('<III',3,extent,584)+bytes.fromhex(sid)+session.struct.pack('<I',512)
                session.struct.pack_into('<III',gui,160,212,100,0xc0000005)
                session.struct.pack_into('<Q',gui,176,0x180012345)
                (root/'ap11.ui').write_bytes(gui);(root/'ap11.ui').chmod(0o600)
                result=observer.snapshot()['editor']
                self.assertEqual(result['view_stage'],212)
                self.assertEqual(result['exception_code'],0xc0000005)
                self.assertEqual(result['exception_instruction'],0x180012345)
                (root/'ap11.ui').unlink()
                self.assertEqual(observer.snapshot()['editor'],result)
            finally:observer.close()

    def test_pending_peer_is_retained_before_containment_without_completion(self):
        for stage in (1,2,3,4,5,6):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);sid='ef'*16
                directory=root/'compatdata/pfx/drive_c/bridge/sessions'/sid
                directory.mkdir(parents=True,mode=0o700);(directory/'ap1.control').touch()
                # Fixed wire declaration, no state/audio payloads. Child uses
                # the same libatomic primitives as the Linux production reader.
                data=bytearray(1024);data[:16]=b'LVFS'+session.struct.pack('<III',1,1024,0);data[16:32]=bytes.fromhex(sid)
                status=directory/'ap12.status';status.write_bytes(data);status.chmod(0o600)
                artifact=root/'host';artifact.write_bytes(b'image')
                binding={'path':str(artifact),'sha256':hashlib.sha256(b'image').hexdigest()}
                spec={'registration':{'host':binding,'module':binding,'environment':{'root':str(root),'runner':{'files':[]}}},
                      'session':sid,'directory':str(directory),'report':str(root/'report.json'),'inspect':False,'binding_sent':True}
                native,owner=socket.socketpair();observations=[]
                program=r"""
import ctypes,mmap,os,sys,time
f=open(sys.argv[1],'r+b');m=mmap.mmap(f.fileno(),1024);a=ctypes.addressof(ctypes.c_char.from_buffer(m))
lib=ctypes.CDLL('libatomic.so.1');store=getattr(lib,'__atomic_store_8');store.argtypes=[ctypes.c_void_p,ctypes.c_uint64,ctypes.c_int]
for lane,stage in [(0,2),(1,int(sys.argv[2])),(2,22)]:
 base=64+lane*320
 row=[4,1,9,0,stage,0,123,1000,os.getpid(),os.getpid()]
 for i,v in enumerate(row):store(a+base+64+128+8*i,v,5)
 store(a+base,1,5)
 # Die later with an unfinished write in the OTHER slot. Reader must keep 9.
 store(a+base+64+2*8,999,5)
time.sleep(30)
"""
                def consumer():
                    # Ordinary peer release ends the same production supervisor.
                    time.sleep(1.5);native.shutdown(socket.SHUT_WR);native.settimeout(8)
                    observations.append(native.recv(1))
                thread=threading.Thread(target=consumer);thread.start()
                cleanup=session.cleanup_process
                def check_before_cleanup(child,owned):
                    saved=json.loads((root/'report.fault.json').read_text())
                    row=saved['before_containment']['delivery']
                    self.assertEqual((row['generation'],row['epoch'],row['request_sequence'],row['position'],row['stage']),(4,1,9,0,stage))
                    self.assertIsNone(child.poll())
                    return cleanup(child,owned)
                try:
                    with patch.object(session,'command',return_value=([sys.executable,'-c',program,str(status),str(stage)],b'')), \
                         patch.object(session,'environment',return_value=os.environ.copy()), \
                         patch.object(session,'cleanup_process',side_effect=check_before_cleanup):
                        outcome=session.run(spec,owner)
                    thread.join(timeout=9);self.assertFalse(thread.is_alive())
                    self.assertEqual(observations,[b'R'])
                    self.assertTrue(outcome['cleanup_confirmed'] and outcome['transport_retired'])
                    self.assertIsNotNone(outcome['fault_status']['early_pending'])
                    self.assertFalse(directory.exists())
                    self.assertTrue((root/'report.fault.json').exists())
                finally:native.close();owner.close()

    def test_reader_rejects_identity_and_never_uses_unstable_slot(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);p=root/'ap12.status';sid='12'*16
            data=bytearray(1024);data[:16]=b'LVFS'+session.struct.pack('<III',1,1024,0);data[16:32]=bytes.fromhex(sid)
            p.write_bytes(data);p.chmod(0o600)
            with self.assertRaisesRegex(RuntimeError,'session/version'):session.FaultStatus(root,'13'*16)
            observer=session.FaultStatus(root,sid)
            try:
                n=[0]
                def changing(offset):
                    if offset==64:n[0]+=1;return n[0]
                    return 777
                with patch.object(observer,'word',side_effect=changing):
                    self.assertEqual(observer.lane(0),{'current':False})
                    self.assertEqual(n[0],6) # exactly three bounded attempts
            finally:observer.close()


class CensusTests(unittest.TestCase):
    def test_accessibility_workaround_is_explicit_and_process_scoped(self):
        # The measured UIA removal fault is selected on an exact registration,
        # not inferred from a product name and not written to the environment.
        reg={'environment':{'root':'/tmp/ap12-test-environment'},
             'compatibility':{'disable_windows_accessibility':False}}
        before=dict(os.environ)
        with patch.object(session.subprocess,'check_output',return_value='DISPLAY=:1\nUNRELATED=private\n'):
            ordinary=session.environment(reg)
            reg['compatibility']['disable_windows_accessibility']=True
            selected=session.environment(reg)
            reg['compatibility']['disable_windows_accessibility']=False
            sibling=session.environment(reg)
        self.assertNotIn('WINEDLLOVERRIDES',ordinary)
        self.assertEqual(selected.pop('WINEDLLOVERRIDES'),'uiautomationcore=')
        self.assertEqual(selected,ordinary)
        self.assertEqual(sibling,ordinary)
        self.assertNotIn('UNRELATED',ordinary)
        self.assertEqual(dict(os.environ),before)

    def test_registered_trace_flag_reaches_only_audio_host(self):
        with tempfile.TemporaryDirectory() as tmp:
            flag=pathlib.Path(tmp)/'.local/share/linux-vst-bridge/managed/runtime/trace-enable'
            flag.parent.mkdir(parents=True)
            for contents,expected in [(b'1\n',True),(b'1\nextra',False),(b'0\n',False)]:
                flag.write_bytes(contents)
                for spec in [{'inspect':False},{'inspect':True},{'inspect':False,'vendor_access':True}]:
                    env={'HOME':tmp}
                    session.delivery_trace(spec,env)
                    self.assertEqual(env.get('LVB_AP10_TRACE')=='1',expected and not spec['inspect'] and not spec.get('vendor_access',False))

    def test_stat_only_parsing_and_descendant_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for pid, parent, start in [(100, 1, 123), (101, 100, 124), (102, 101, 125), (103, 1, 126)]:
                d = root / str(pid)
                d.mkdir()
                fields = ['S', str(parent), '100', '100'] + ['0'] * 15 + [str(start)]
                (d / 'stat').write_text(f'{pid} (name with ) parentheses) ' + ' '.join(fields))
            records = ownership.process_identities(root)
            children = ownership.descendant_identities(100, records)
            self.assertEqual([(p['pid'], p['start_ticks']) for p in children], [(101, 124), (102, 125)])
            # No command-line/name files exist; routine tracking cannot depend on them.

    def test_tracker_rejects_reused_parent_and_follows_new_reparented_children(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            def process(pid, parent, start, children=''):
                d = root / str(pid)
                d.mkdir(exist_ok=True)
                fields = ['S', str(parent), '100', '100'] + ['0'] * 15 + [str(start)]
                (d / 'stat').write_text(f'{pid} (test) ' + ' '.join(fields))
                task = d / 'task' / str(pid)
                task.mkdir(parents=True, exist_ok=True)
                (task / 'children').write_text(children)
            process(100, 1, 10, '101')
            process(101, 100, 11)
            tracker = ownership.ProcessTracker(100, root)
            # A remembered child can acquire new children after reparenting.
            process(101, 1, 11, '102')
            process(102, 101, 12)
            self.assertIn((102, 12), tracker.update())
            # Reuse of the original PID cannot grant ownership of its new tree.
            process(100, 1, 20, '103')
            process(103, 100, 21)
            self.assertNotIn((100, 20), tracker.update())
            self.assertNotIn((103, 21), tracker.owned)
            # Nor may a parent changed during traversal grant child ownership.
            process(101, 1, 11, '104')
            process(104, 101, 22)
            identity = tracker.identity
            seen = 0
            def reused_during_scan(pid):
                nonlocal seen
                result = identity(pid)
                if pid == 101:
                    seen += 1
                    if seen == 2:
                        return (99, 1)
                return result
            with patch.object(tracker, 'identity', side_effect=reused_during_scan):
                self.assertNotIn((104, 22), tracker.update())

    def test_changed_artifact_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / 'image'
            p.write_bytes(b'actual')
            with self.assertRaisesRegex(RuntimeError, 'changed'):
                session.verify({'path': str(p), 'sha256': '0' * 64})


if __name__ == '__main__':
    unittest.main()
