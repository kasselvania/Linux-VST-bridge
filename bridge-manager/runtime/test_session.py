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


class VendorOperationTests(unittest.TestCase):
    def test_ASC_launch_uses_vendor_shortcut_working_directory(self):
        root=pathlib.Path('/fixture/environment')
        exe=root/'compatdata/pfx/drive_c/Program Files (x86)/Arturia/Arturia Software Center/Arturia Software Center.exe'
        spec={'application':{'environment':{'root':str(root),'runner':{'entry_point':'/runner/entry','proton':'/runner/proton'}},'executable':{'path':str(exe)}}}
        argv,cwd=session.vendor_launch(spec)
        self.assertEqual(cwd,exe.parent)
        self.assertEqual(argv,['/runner/entry','--verb=run','--','/runner/proton','runinprefix',r'C:\Program Files (x86)\Arturia\Arturia Software Center\Arturia Software Center.exe'])

    def test_launcher_exit_does_not_complete_or_cancel_owned_helpers(self):
        self.assertEqual(session.vendor_operation_state(None, 2), 'running')
        self.assertEqual(session.vendor_operation_state(0, 1), 'unknown')
        self.assertEqual(session.vendor_operation_state(1, 1), 'unknown')
        self.assertEqual(session.vendor_operation_state(0, 0), 'completed')
        self.assertEqual(session.vendor_operation_state(1, 0), 'failed')


class BusCensusCommandTests(unittest.TestCase):
    def test_event_policy_is_registered_not_ambient(self):
        reg={'environment':{'root':'/fixture'},'compatibility':{'disable_windows_accessibility':False}}
        with patch.object(session.subprocess,'check_output',return_value='DISPLAY=:0\nLVB_EVENT_OUTPUT_POLICY=reported_zero_event_channels_unspecified\n'):
            self.assertNotIn('LVB_EVENT_OUTPUT_POLICY',session.environment(reg))
            reg['compatibility']['event_output']='reported_zero_event_channels_unspecified'
            self.assertEqual(session.environment(reg)['LVB_EVENT_OUTPUT_POLICY'],reg['compatibility']['event_output'])
            self.assertNotIn('LVB_EDITOR_LIFETIME',session.environment(reg))
            reg['compatibility']['editor_lifetime']='retain_editor_view_until_instance_retirement'
            self.assertEqual(session.environment(reg)['LVB_EDITOR_LIFETIME'],reg['compatibility']['editor_lifetime'])
            self.assertNotIn('LVB_VENDOR_RETIREMENT',session.environment(reg))
            reg['compatibility']['vendor_retirement']='process_scoped_vendor_retirement'
            self.assertEqual(session.environment(reg)['LVB_VENDOR_RETIREMENT'],'process_scoped_vendor_retirement')
            reg['compatibility']['vendor_retirement']='all_vendor_processes'
            with self.assertRaisesRegex(RuntimeError,'unsupported vendor retirement'):session.environment(reg)
            del reg['compatibility']['vendor_retirement']
            reg['compatibility']['editor_lifetime']='all_editors'
            with self.assertRaisesRegex(RuntimeError,'unsupported editor lifetime'):session.environment(reg)
            del reg['compatibility']['editor_lifetime']
            reg['compatibility']['event_output']='all_zero_buses'
            with self.assertRaisesRegex(RuntimeError,'unsupported event output policy'):session.environment(reg)

    def test_probe_is_inspection_only_and_handshake_bound(self):
        reg={'environment':{'root':'/fixture','runner':{'entry_point':'/entry','proton':'/proton'}},
             'metadata':{'class_id':'A'*32},'host':{'path':'/fixture/compatdata/pfx/drive_c/host.exe','sha256':'1'*64},
             'host_source_sha256':'2'*64,'module':{'path':'/fixture/compatdata/pfx/drive_c/plugin.vst3','sha256':'3'*64}}
        spec={'registration':reg,'session':'4'*32,'inspect':True,'bus_lifecycle_probe':True}
        argv,binding=session.command(spec)
        self.assertEqual(argv[argv.index('--mode')+1],'ap18-bus-lifecycle')
        self.assertIn(b'mode=ap18-bus-lifecycle\n',binding)
        for key,value in [('inspect',False),('keeper',True),('vendor_access',True)]:
            with self.assertRaisesRegex(RuntimeError,'isolated inspection'):
                session.command(dict(spec,**{key:value}))
        del spec['bus_lifecycle_probe']
        argv,_=session.command(spec)
        self.assertEqual(argv[argv.index('--mode')+1],'ap8-module-inspection')


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

    def test_gui_diagnostic_versions_are_exact_and_preserve_retained_layout(self):
        for version,header,message in [(3,256,584),(4,320,608),(5,320,608),(6,320,608),(4,256,584)]:
            with self.subTest(version=version,header=header), tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);sid='34'*16
                data=bytearray(1024);data[:32]=b'LVFS'+session.struct.pack('<III',1,1024,0)+bytes.fromhex(sid)
                (root/'ap12.status').write_bytes(data);(root/'ap12.status').chmod(0o600)
                extent=header+2*512*message;gui=bytearray(extent)
                gui[:36]=b'LVBU'+session.struct.pack('<III',version,extent,message)+bytes.fromhex(sid)+session.struct.pack('<I',512)
                session.struct.pack_into('<I',gui,116,1)
                (root/'ap11.ui').write_bytes(gui);(root/'ap11.ui').chmod(0o600)
                observer=session.FaultStatus(root,sid)
                try:
                    if (version,header) in [(3,256),(4,320),(5,320)]:
                        self.assertEqual(observer.snapshot()['editor']['open'],1)
                    else:
                        with self.assertRaisesRegex(RuntimeError,'GUI identity/version'):observer.snapshot()
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
    def test_only_coordinated_inspection_shares_the_keeper_lock(self):
        import fcntl
        self.assertEqual(session.operation_lock_mode({'inspect': True}), fcntl.LOCK_EX)
        self.assertEqual(session.operation_lock_mode({'inspect': False}), fcntl.LOCK_SH)
        self.assertEqual(session.operation_lock_mode({'inspect': True, 'shared_inspection': True}), fcntl.LOCK_SH)
        for flags in ({'inspect': False}, {'inspect': True, 'keeper': True}, {'inspect': True, 'vendor_access': True}):
            with self.assertRaises(RuntimeError):
                session.operation_lock_mode(dict(flags, shared_inspection=True))

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



@unittest.skipUnless(sys.platform == 'linux', 'Memory transport requires Linux tmpfs')
class MemoryTransportTests(unittest.TestCase):
    def fixture(self, disk, memory):
        durable=pathlib.Path(disk)/'compatdata/pfx/drive_c/bridge/sessions'/('ab'*16)
        durable.mkdir(parents=True,mode=0o700)
        root=pathlib.Path(memory);(root/'storage-v1').write_bytes(b'linux-vst-bridge volatile transport v1\n');(root/'storage-v1').chmod(0o600)
        directory=root/('ab'*16);directory.mkdir(mode=0o700);m=directory.stat()
        host=pathlib.Path(disk)/'host';host.write_bytes(b'exact-image')
        artifact={'path':str(host),'sha256':hashlib.sha256(host.read_bytes()).hexdigest()}
        reg={'host':artifact,'module':artifact,'host_source_sha256':'a'*64,'metadata':{'class_id':'01'*16},
             'environment':{'root':disk,'runner':{'files':[],'entry_point':'/exact/entry','proton':'/exact/proton'}}}
        spec={'registration':reg,'session':'ab'*16,'directory':str(directory),'report':str(pathlib.Path(disk)/'report.json'),
              'inspect':False,'binding_sent':True,'shared_runtime':True,'transport':{'schema':1,'device':m.st_dev,'inode':m.st_ino}}
        return spec,durable,directory

    def test_production_run_admits_memory_transport_and_retires_exact_session(self):
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            # create=True lets this exact test exercise the baseline run owner,
            # which rejected a memory-backed session before launching anything.
            with patch.object(session,'transport_root',return_value=pathlib.Path(memory),create=True), \
                 patch.object(session,'command',return_value=([sys.executable,'-c','raise SystemExit(90)'],b'')), \
                 patch.object(session,'environment',return_value=os.environ.copy()):
                outcome=session.run(spec)
            self.assertTrue(outcome['cleanup_confirmed'] and outcome['transport_retired'])
            self.assertFalse(directory.exists() or durable.exists())

    def test_pinned_windows_handshake_remains_c_drive_and_only_transport_uses_memory(self):
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            with patch.object(session,'transport_root',return_value=pathlib.Path(memory)):
                self.assertEqual(session.session_directories(spec),(directory,durable))
                env={};session.transport_environment(spec,env)
                self.assertEqual(env,{'PRESSURE_VESSEL_FILESYSTEMS_RW':memory})
                cmd,_=session.command(spec)
                for field in ['ready','gate']:
                    self.assertEqual(cmd[cmd.index('--'+field)+1],
                                     'C:\\bridge\\sessions\\'+spec['session']+'\\'+spec['session']+'.'+field)
                env={};session.transport_environment(dict(spec,shared_runtime=False),env);self.assertEqual(env,{})

    def test_windows_views_are_exact_memory_files_and_refuse_foreign_or_replaced_sources(self):
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            names=('ap1.control','ap1.audio','ap11.ui','ap12.status','ap10.delivery','ap18.results')
            for name in names:(directory/name).write_bytes(name.encode());(directory/name).chmod(0o600)
            (durable/'owner.json').write_text('retained durable owner')
            with patch.object(session,'transport_root',return_value=pathlib.Path(memory)):
                (durable/'ap12.status').symlink_to('/does/not/exist')
                with self.assertRaisesRegex(RuntimeError,'already exists'):session.windows_transport_views(spec)
                self.assertFalse((durable/'ap1.audio').exists())
                (durable/'ap12.status').unlink()
                (directory/'ap11.ui').unlink();(directory/'ap11.ui').symlink_to(directory/'ap1.audio')
                with self.assertRaisesRegex(RuntimeError,'ownership/type'):session.windows_transport_views(spec)
                self.assertFalse((durable/'ap1.audio').exists())
                (directory/'ap11.ui').unlink();(directory/'ap11.ui').write_bytes(b'ui');(directory/'ap11.ui').chmod(0o600)
                session.windows_transport_views(spec)
                for name in names:
                    self.assertTrue((durable/name).is_symlink())
                    self.assertTrue((durable/name).samefile(directory/name))
                    self.assertEqual((durable/name).read_bytes(),(directory/name).read_bytes())
                self.assertFalse((durable/'owner.json').is_symlink())
                with self.assertRaisesRegex(RuntimeError,'already exists'):session.windows_transport_views(spec)
                session.retire_directories(spec)
                self.assertFalse(directory.exists() or durable.exists())

    def test_production_gate_waits_for_exact_windows_views_and_closes_both_owners(self):
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            for name in ('ap1.control','ap1.audio','ap11.ui','ap12.status'):
                (directory/name).write_bytes(b'exact-'+name.encode());(directory/name).chmod(0o600)
            # Use the production command's binding and exact C: paths. The fake
            # peer models the pinned Windows gate, then opens those physical
            # views. No vendor processing/Windows compilation claim.
            with patch.object(session,'transport_root',return_value=pathlib.Path(memory)):
                cmd,binding=session.command(spec)
            (durable/(spec['session']+'.ready')).write_bytes(binding)
            program="""import pathlib,sys,time
d=pathlib.Path(sys.argv[1]);sid=sys.argv[2]
print('{"event":"lifecycle","state":"readiness_announced"}',flush=True)
end=time.monotonic()+4
while not (d/(sid+'.gate')).exists():
 if time.monotonic()>end:raise SystemExit(82)
 time.sleep(.01)
assert (d/(sid+'.gate')).read_bytes()==(d/(sid+'.ready')).read_bytes()
for n in ('ap1.control','ap1.audio','ap11.ui','ap12.status'):
 assert (d/n).is_symlink() and (d/n).read_bytes()==b'exact-'+n.encode()
assert (d/'ap18.results').is_symlink() and (d/'ap18.results').read_bytes()[:4]==b'LVRS'
print('{"event":"lifecycle","state":"scanner_completed"}',flush=True)
"""
            with patch.object(session,'transport_root',return_value=pathlib.Path(memory)), \
                 patch.object(session,'command',return_value=([sys.executable,'-c',program,str(durable),spec['session']],binding)), \
                 patch.object(session,'environment',return_value=os.environ.copy()), \
                 patch.object(session,'FaultStatus',return_value=None):
                outcome=session.run(spec)
            self.assertTrue(outcome['gated'])
            self.assertIsNone(outcome['error'])
            self.assertTrue(outcome['cleanup_confirmed'] and outcome['transport_retired'])
            self.assertFalse(directory.exists() or durable.exists())

    def test_replaced_disk_foreign_and_malformed_storage_are_refused(self):
        import copy
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            with patch.object(session,'transport_root',return_value=pathlib.Path(memory)):
                for key,value in [('schema',2),('schema',True),('inode',0),('inode',False),('device',0),('extra',True)]:
                    bad=copy.deepcopy(spec);bad['transport'][key]=value
                    with self.assertRaises(RuntimeError):session.session_directories(bad)
                for key in ['inspect','keeper','vendor_access']:
                    with self.assertRaises(RuntimeError):session.session_directories(dict(spec,**{key:True}))
                with self.assertRaises(RuntimeError):session.session_directories(dict(spec,directory=str(durable)))
                old=directory.with_name('retained');directory.rename(old);directory.mkdir(mode=0o700)
                with self.assertRaisesRegex(RuntimeError,'replaced'):session.retire_directories(spec)
                self.assertTrue(old.exists() and directory.exists() and durable.exists())
                (pathlib.Path(memory)/'storage-v1').write_bytes(b'foreign')
                with self.assertRaisesRegex(RuntimeError,'foreign'):session.transport_environment(spec,{})
            # The selected mechanism excludes the old disk-backed placement.
            # No timing threshold or sleeping fake worker can turn it into RAM.
            with self.assertRaisesRegex(RuntimeError,'tmpfs'):session.memory_directory(durable)

    def test_retirement_waits_for_both_owners_and_preserves_sibling_and_record(self):
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            (directory/'ap1.control').touch();(durable/'owner.json').write_text('durable ownership')
            sibling=pathlib.Path(memory)/('cd'*16);sibling.mkdir(mode=0o700);(sibling/'live').write_bytes(b'sibling')
            native,owner=socket.socketpair();observations=[]
            def consume():
                native.settimeout(5);observations.append(native.recv(1))
                observations.append((directory.exists(),durable.exists()))
                native.shutdown(socket.SHUT_WR);observations.append(native.recv(1))
                observations.append((directory.exists(),durable.exists()))
            thread=threading.Thread(target=consume);thread.start()
            try:
                with patch.object(session,'transport_root',return_value=pathlib.Path(memory)), \
                     patch.object(session,'command',return_value=([sys.executable,'-c','raise SystemExit(90)'],b'')), \
                     patch.object(session,'environment',return_value=os.environ.copy()):
                    outcome=session.run(spec,owner)
                thread.join(timeout=6);self.assertFalse(thread.is_alive())
                self.assertEqual(observations,[b'F',(True,True),b'R',(False,False)])
                self.assertTrue(outcome['cleanup_confirmed'] and outcome['transport_retired'])
                self.assertEqual(outcome['transport_storage'],spec['transport'])
                self.assertEqual((sibling/'live').read_bytes(),b'sibling')
                receipt=json.loads(pathlib.Path(spec['report']).with_suffix('.ownership.json').read_text())
                self.assertTrue(receipt['transport_retired'])
            finally:native.close();owner.close()

    def test_failed_memory_retirement_never_sends_a_false_receipt(self):
        with tempfile.TemporaryDirectory(dir=pathlib.Path.cwd()) as disk, tempfile.TemporaryDirectory(dir='/dev/shm') as memory:
            spec,durable,directory=self.fixture(disk,memory)
            (directory/'ap1.control').touch()
            native,owner=socket.socketpair();observations=[]
            def consume():
                native.settimeout(5);observations.append(native.recv(1));native.shutdown(socket.SHUT_WR)
                native.settimeout(.5)
                try:observations.append(native.recv(1))
                except TimeoutError:observations.append('no retirement acknowledgement')
            thread=threading.Thread(target=consume);thread.start()
            try:
                with patch.object(session,'transport_root',return_value=pathlib.Path(memory)), \
                     patch.object(session,'command',return_value=([sys.executable,'-c','raise SystemExit(90)'],b'')), \
                     patch.object(session,'environment',return_value=os.environ.copy()), \
                     patch.object(session,'retire_directories',side_effect=OSError('injected unlink refusal')):
                    outcome=session.run(spec,owner)
                thread.join(timeout=6);self.assertFalse(thread.is_alive())
                self.assertEqual(observations,[b'F','no retirement acknowledgement'])
                self.assertTrue(outcome['cleanup_confirmed']);self.assertFalse(outcome['transport_retired'])
                self.assertTrue(directory.exists() and durable.exists())
                receipt=json.loads(pathlib.Path(spec['report']).with_suffix('.ownership.json').read_text())
                self.assertFalse(receipt['transport_retired'])
            finally:native.close();owner.close()


class CompanionDiagnosticTests(unittest.TestCase):
    def test_private_capture_capacity_time_and_separate_streams(self):
        with tempfile.TemporaryDirectory() as temp:
            p=pathlib.Path(temp);out=session.PrivateCapture(p/'stdout',capacity=4)
            err=session.PrivateCapture(p/'stderr',capacity=3)
            try:
                out.write(b'abcdef');err.write(b'xyz!')
                self.assertEqual((out.retained,out.discarded),(4,2))
                self.assertEqual((err.retained,err.discarded),(3,1))
                out.deadline=0;out.write(b'late')
                self.assertEqual((out.retained,out.discarded),(4,6))
                self.assertEqual((p/'stdout').read_bytes(),b'abcd')
                self.assertEqual((p/'stderr').read_bytes(),b'xyz')
                self.assertEqual((p/'stdout').stat().st_mode&0o777,0o600)
            finally:out.close();err.close()

    def test_wrapper_arguments_are_not_registered_image_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            root=pathlib.Path(temp);image=root/'agent.exe';image.write_bytes(b'exact image')
            proc=root/'proc'/'123';proc.mkdir(parents=True)
            (proc/'cmdline').write_bytes(str(image).encode()+b'\0')
            (proc/'maps').write_text('')
            artifact={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
            app={'executable':artifact,'helpers':[artifact,artifact],'environment':{'runner':{'files':[]}}}
            class Scope:
                proc_root=root/'proc'
                def identity(self,pid):return None
            record={'pid':123,'ppid':1}
            self.assertEqual(session.vendor_process_metadata(Scope(),record,app)['registered_images'],[])
            st=image.stat()
            (proc/'maps').write_text(f'1000-2000 r--p 00000000 {os.major(st.st_dev):x}:{os.minor(st.st_dev):x} {st.st_ino} /container/alias/agent.exe\n')
            self.assertEqual(len(session.vendor_process_metadata(Scope(),record,app)['registered_images']),3)

    def test_diagnostic_modes_do_not_accept_arbitrary_executable_or_verb(self):
        root=pathlib.Path('/fixture/environment')
        app={'environment':{'root':str(root),'runner':{'entry_point':'/runner/entry','proton':'/runner/proton'}},
             'executable':{'path':str(root/'compatdata/pfx/drive_c/ASC/main.exe')},
             'helpers':[{'path':str(root/'compatdata/pfx/drive_c/ASC/agent.exe')}]}
        for mode,verb,image in [('agent_probe','runinprefix','agent.exe'),('runinprefix_probe','runinprefix','main.exe'),('initialized_probe','run','main.exe'),('accessibility_probe','runinprefix','main.exe')]:
            cmd,cwd=session.vendor_launch({'application':app,'mode':mode})
            self.assertEqual(cmd[4],verb);self.assertTrue(cmd[-1].endswith(image))
        for mode in ['normal','agent_probe','runinprefix_probe','initialized_probe','accessibility_probe']:
            self.assertEqual(session.vendor_compatibility(mode),{'disable_windows_accessibility':mode in ('normal','accessibility_probe')})
        with self.assertRaises(RuntimeError):session.vendor_compatibility('global')
        with self.assertRaises(RuntimeError):session.vendor_launch({'application':app,'mode':'arbitrary'})
        env=session.vendor_diagnostic_environment({},pathlib.Path('/private/log'),True)
        self.assertEqual(env['PROTON_LOG'],'0')
        self.assertNotIn('+all',env['WINEDEBUG']);self.assertIn('trace+process',env['WINEDEBUG'])
        self.assertIn('trace+unwind',env['WINEDEBUG']);self.assertIn('trace+loaddll',env['WINEDEBUG'])
        self.assertEqual(session.vendor_diagnostic_environment({},pathlib.Path('/private/log'),False),{})


class CgroupFixture:
    group='/user.slice/linux-vst-bridge-vendor-arturia-software-center.service'
    def __init__(self, path, supervisor=99999):
        self.path=path;self.proc=path/'proc';self.cg=path/'cgroup';self.supervisor=supervisor
        self.unit=self.cg/self.group.lstrip('/');self.unit.mkdir(parents=True)
        self.record(supervisor,1,1);self.set_members([])
    def record(self,pid,start,parent,group=None,state='S'):
        p=self.proc/str(pid);p.mkdir(parents=True,exist_ok=True)
        fields=['0']*50;fields[0]=state;fields[1]=str(parent);fields[19]=str(start)
        (p/'stat').write_text(str(pid)+' (fixture) '+' '.join(fields))
        (p/'cgroup').write_text('0::'+(self.group if group is None else group)+'\n')
    def set_members(self,pids):
        (self.unit/'cgroup.procs').write_text('\n'.join(map(str,[self.supervisor,*pids])))
    def scope(self):
        return ownership.CompanionCgroup(proc_root=self.proc,cgroup_root=self.cg,supervisor=self.supervisor)


class CompanionCgroupTests(unittest.TestCase):
    def test_fast_reparented_agent_and_unknown_handoff_keep_unit_alive(self):
        with tempfile.TemporaryDirectory() as temp:
            f=CgroupFixture(pathlib.Path(temp));scope=f.scope()
            # The parent was never sampled. Both new processes have already
            # been reparented by the time the outer launcher returns 5.
            f.record(101,1001,1);f.record(102,1002,1)
            f.set_members([101,102]);members=scope.members()
            self.assertEqual([p['pid'] for p in members],[101,102])
            self.assertEqual(session.vendor_operation_state(5,len(members)),'unknown')
            f.set_members([102])
            self.assertEqual(session.vendor_operation_state(0,len(scope.members())),'unknown')
            f.set_members([])
            self.assertEqual(session.vendor_operation_state(0,len(scope.members())),'completed')

    def test_same_user_session_and_foreign_cgroup_never_authorize_adoption(self):
        with tempfile.TemporaryDirectory() as temp:
            f=CgroupFixture(pathlib.Path(temp));scope=f.scope()
            f.record(101,1001,1);f.record(102,1002,1,group='/user.slice/unrelated.service')
            f.set_members([101,102])
            self.assertEqual([p['pid'] for p in scope.members()],[101])
            f.record(99999,1,1,group='/user.slice/unrelated.service')
            with self.assertRaises(RuntimeError):f.scope()

    def test_normal_main_and_agent_close_requires_empty_physical_group(self):
        with tempfile.TemporaryDirectory() as temp:
            f=CgroupFixture(pathlib.Path(temp));scope=f.scope()
            f.record(101,1001,1);f.record(102,1002,1);f.set_members([101,102])
            self.assertEqual(session.vendor_operation_state(0,len(scope.members())),'unknown')
            f.set_members([102]);self.assertEqual(session.vendor_operation_state(0,len(scope.members())),'unknown')
            f.set_members([]);self.assertTrue(scope.cleanup(lambda:None,timeout=.1))

    def test_cancel_signals_exact_members_with_pidfd_and_rechecks_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            f=CgroupFixture(pathlib.Path(temp));scope=f.scope()
            f.record(101,1001,1);f.record(102,1002,1,group='/user.slice/unrelated.service');f.set_members([101,102])
            with patch.object(os,'pidfd_open',create=True,return_value=77) as opened, \
                 patch.object(os,'close') as closed, patch.object(signal,'pidfd_send_signal',create=True) as sent:
                scope.signal_members(signal.SIGTERM)
                opened.assert_called_once_with(101);sent.assert_called_once_with(77,signal.SIGTERM);closed.assert_called_once_with(77)
            def recycled(_):f.record(101,2002,1);return 78
            with patch.object(os,'pidfd_open',create=True,side_effect=recycled), \
                 patch.object(os,'close'),patch.object(signal,'pidfd_send_signal',create=True) as sent:
                scope.signal_members(signal.SIGTERM);sent.assert_not_called()

    @unittest.skipUnless(sys.platform=='linux','real subreaper and pidfd fixture')
    def test_production_owner_survives_unsampled_double_fork_until_cancel(self):
        with tempfile.TemporaryDirectory() as temp:
            root=pathlib.Path(temp);env=root/'environment';env.mkdir();(env/'operation.lock').touch()
            image=root/'image';image.write_bytes(b'exact fixture');artifact={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
            app={'executable':artifact,'helpers':[artifact,artifact],
                 'environment':{'root':str(env),'runner':{'files':[]}}}
            pidfile=root/'handoff';report=root/'result.json';f=CgroupFixture(root/'kernel',os.getpid())
            base_scope=f.scope()
            class LiveFixture:
                proc_root=f.proc;group=f.group
                def identity(self,pid):return base_scope.identity(pid)
                def members(self):
                    ids=[]
                    if pidfile.exists():
                        for pid in json.loads(pidfile.read_text()):
                            try:
                                raw=(pathlib.Path('/proc')/str(pid)/'stat').read_text()
                                p=f.proc/str(pid);p.mkdir(exist_ok=True);(p/'stat').write_text(raw);(p/'cgroup').write_text('0::'+f.group+'\n');ids.append(pid)
                            except FileNotFoundError:pass
                    f.set_members(ids);return base_scope.members()
                def cleanup(self,reap):
                    deadline=time.monotonic()+4
                    while time.monotonic()<deadline:
                        reap()
                        if not [p for p in self.members() if p['state']!='Z']:return True
                        base_scope.signal_members(signal.SIGTERM);time.sleep(.02)
                    return False
            code='''import os,sys,json,time
middle=os.fork()
if middle==0:
 child=os.fork()
 if child==0:
  os.setsid();time.sleep(30);os._exit(0)
 with open(sys.argv[1],'w') as f:json.dump([child],f)
 os._exit(0)
os.waitpid(middle,0)
print('private stdout',flush=True)
print('private stderr',file=sys.stderr,flush=True)
os._exit(5)
'''
            observed=[];done=threading.Event();oldterm=signal.getsignal(signal.SIGTERM);oldint=signal.getsignal(signal.SIGINT)
            sibling=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
            def stop_after_handoff():
                deadline=time.monotonic()+5
                while time.monotonic()<deadline and not done.is_set():
                    try:
                        value=json.loads(report.read_text())
                        if value['launcher_exit']==5 and value['owned_live']==1:
                            observed.append(value);os.kill(os.getpid(),signal.SIGTERM);return
                    except (FileNotFoundError,json.JSONDecodeError):pass
                    time.sleep(.02)
                if not done.is_set():os.kill(os.getpid(),signal.SIGTERM)
            watcher=threading.Thread(target=stop_after_handoff);watcher.start()
            try:
                with patch.object(session,'CompanionCgroup',return_value=LiveFixture()), \
                     patch.object(session,'vendor_launch',return_value=([sys.executable,'-c',code,str(pidfile)],root)), \
                     patch.object(session,'environment',return_value=os.environ.copy()):
                    self.assertTrue(session.vendor_application({'application':app,'report':str(report),'mode':'runinprefix_probe'}))
                self.assertEqual(len(observed),1);self.assertEqual(observed[0]['state'],'unknown')
                final=json.loads(report.read_text());self.assertEqual(final['state'],'cancelled');self.assertTrue(final['cleanup_confirmed'])
                self.assertNotIn('private stdout',report.read_text());self.assertIsNone(sibling.poll())
                logs=list(root.glob('private-diagnostic-*'));self.assertEqual(len(logs),1)
                self.assertIn(b'private stdout',(logs[0]/'stdout.log').read_bytes())
                self.assertIn(b'private stderr',(logs[0]/'stderr.log').read_bytes())
            finally:
                done.set();watcher.join(timeout=6);signal.signal(signal.SIGTERM,oldterm);signal.signal(signal.SIGINT,oldint)
                sibling.terminate();sibling.wait(timeout=3)
                session.ctypes.CDLL(None).prctl(36,0,0,0,0)

if __name__ == '__main__':
    unittest.main()

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux atomic mapped-status reader')
class ResultCustodyTests(unittest.TestCase):
    def test_empty_interrupted_first_write_and_immutable_complete_slot(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);sid='18'*16
            session.ResultStatus.create(root,sid)
            with self.assertRaises(FileExistsError):session.ResultStatus.create(root,sid)
            reader=session.ResultStatus(root,sid)
            try:
                self.assertIsNone(reader.snapshot()['rejection'])
                # Independent child dies with only an inactive partial slot.
                code="import os,mmap,struct,sys;f=open(sys.argv[1],'r+b');m=mmap.mmap(f.fileno(),1024);struct.pack_into('<Q',m,512,99);os._exit(7)"
                p=subprocess.run([sys.executable,'-c',code,str(root/'ap18.results')])
                self.assertEqual(p.returncode,7);self.assertIsNone(reader.snapshot()['rejection'])
                row=[17,3,91,1440,1,1,0,21,256,0,0,1,0,0xffffffff,0xffffffff,0,0xffffffff,0xffffffff,0xffffffff,0,0,0,0,42,0,0x4000000000000000,0,0,0,1,0]
                # The Windows full-path test writes the same explicit word layout.
                with (root/'ap18.results').open('r+b') as f:
                    f.seek(512);f.write(session.struct.pack('<31Q',*row));f.seek(64);f.write(session.struct.pack('<Q',1))
                result=reader.snapshot()['rejection']
                self.assertEqual(result['reason'],'ValueAboveOne');self.assertEqual(result['request_sequence'],91)
                self.assertEqual(result['event_type'],-1);self.assertEqual(result['parameter_id'],42)
                # Interrupted attempted replacement never commits over prior slot.
                with (root/'ap18.results').open('r+b') as f:f.seek(128);f.write(b'\xff'*248)
                self.assertEqual(reader.snapshot()['rejection'],result)
                self.assertNotIn('payload',result)
            finally:reader.close()
    def test_version_identity_extent_and_permissions_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);sid='18'*16;session.ResultStatus.create(root,sid)
            with self.assertRaisesRegex(RuntimeError,'identity/version'):session.ResultStatus(root,'19'*16)
            p=root/'ap18.results';p.chmod(0o644)
            with self.assertRaisesRegex(RuntimeError,'ownership/extent'):session.ResultStatus(root,sid)
            p.chmod(0o600)
            with p.open('r+b') as f:f.seek(4);f.write(session.struct.pack('<I',2))
            with self.assertRaisesRegex(RuntimeError,'identity/version'):session.ResultStatus(root,sid)
    def test_supervisor_retains_rejection_before_abnormal_child_cleanup(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);sid='18'*16
            directory=root/'compatdata/pfx/drive_c/bridge/sessions'/sid;directory.mkdir(parents=True,mode=0o700)
            artifact=root/'host';artifact.write_bytes(b'fixture')
            binding={'path':str(artifact),'sha256':hashlib.sha256(b'fixture').hexdigest()}
            spec={'registration':{'host':binding,'module':binding,'environment':{'root':str(root),'runner':{'files':[]}}},'session':sid,'directory':str(directory),'report':str(root/'report.json'),'inspect':False}
            program="""
import os,sys,mmap,ctypes
f=open(sys.argv[1],'r+b');m=mmap.mmap(f.fileno(),1024);a=ctypes.addressof(ctypes.c_char.from_buffer(m))
l=ctypes.CDLL('libatomic.so.1');s=getattr(l,'__atomic_store_8');s.argtypes=[ctypes.c_void_p,ctypes.c_uint64,ctypes.c_int]
row=[17,3,91,1440,1,0,0,21,256,0,0,1,0,0xffffffff,0xffffffff,0,0xffffffff,0xffffffff,0xffffffff,0,0,0,0,42,0,0x4000000000000000,0,0,0,1,0]
for i,v in enumerate(row):s(a+512+i*8,v,5)
s(a+64,1,5)
os._exit(86)
"""
            cleanup=session.cleanup_process
            def before_cleanup(child,owned):
                saved=json.loads((root/'report.fault.json').read_text())
                self.assertEqual(saved['before_containment']['result_status']['rejection']['request_sequence'],91)
                self.assertTrue((directory/'ap18.results').exists())
                return cleanup(child,owned)
            with patch.object(session,'command',return_value=([sys.executable,'-c',program,str(directory/'ap18.results')],b'')),patch.object(session,'environment',return_value=os.environ.copy()),patch.object(session,'cleanup_process',side_effect=before_cleanup):
                result=session.run(spec)
            self.assertEqual(result['raw_exit'],86)
            self.assertEqual(result['fault_status']['before_containment']['result_status']['rejection']['reason'],'ValueAboveOne')
            self.assertTrue(result['cleanup_confirmed'] and result['transport_retired'])
            self.assertFalse(directory.exists())

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux retirement authority and process containment')
class ProcessRetirementTests(unittest.TestCase):
    def test_status_is_session_bound_complete_first_write_and_not_a_crash_receipt(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);sid='28'*16
            session.RetirementStatus.create(root,sid)
            with self.assertRaisesRegex(RuntimeError,'identity/version'):session.RetirementStatus(root,'29'*16)
            reader=session.RetirementStatus(root,sid)
            try:
                self.assertIsNone(reader.snapshot())
                with (root/'ap18.retirement').open('r+b') as f:
                    f.seek(192);f.write(session.struct.pack('<8Q',127,1,20,256,1,0,0,0))
                self.assertIsNone(reader.snapshot())
                with (root/'ap18.retirement').open('r+b') as f:f.seek(64);f.write(session.struct.pack('<Q',1))
                first=reader.snapshot();self.assertEqual(first['session'],sid)
                self.assertEqual(first['state'],'process_scoped_retirement_ready')
                with (root/'ap18.retirement').open('r+b') as f:f.seek(128);f.write(b'\xff'*64)
                self.assertEqual(reader.snapshot(),first)
                with (root/'ap18.retirement').open('r+b') as f:f.seek(192);f.write(session.struct.pack('<Q',126))
                with self.assertRaisesRegex(RuntimeError,'incomplete'):reader.snapshot()
            finally:reader.close()

    def test_supervisor_contains_ready_cohort_once_not_unrelated_sibling(self):
        for mode in ('ready','missing','incomplete'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);sid='28'*16
                directory=root/'compatdata/pfx/drive_c/bridge/sessions'/sid;directory.mkdir(parents=True,mode=0o700)
                artifact=root/'host';artifact.write_bytes(b'fixture')
                binding={'path':str(artifact),'sha256':hashlib.sha256(b'fixture').hexdigest()}
                spec={'registration':{'host':binding,'module':binding,'environment':{'root':str(root),'runner':{'files':[]}}},'session':sid,'directory':str(directory),'report':str(root/'report.json'),'inspect':False}
                program=r'''
import os,sys,mmap,ctypes,time
f=open(sys.argv[1],'r+b');m=mmap.mmap(f.fileno(),256);a=ctypes.addressof(ctypes.c_char.from_buffer(m))
l=ctypes.CDLL('libatomic.so.1');s=getattr(l,'__atomic_store_8');s.argtypes=[ctypes.c_void_p,ctypes.c_uint64,ctypes.c_int]
if sys.argv[2]=='missing':os._exit(86)
for i,v in enumerate([127 if sys.argv[2]=='ready' else 126,1,20,256,1,0,0,0]):s(a+192+i*8,v,5)
s(a+64,1,5)
time.sleep(30)
'''
                sibling=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
                cleanup=session.cleanup_process;retire=session.retire_directories
                try:
                    env=dict(os.environ,LVB_VENDOR_RETIREMENT='process_scoped_vendor_retirement')
                    with patch.object(session,'command',return_value=([sys.executable,'-c',program,str(directory/'ap18.retirement'),mode],b'')),patch.object(session,'environment',return_value=env),patch.object(session,'cleanup_process',wraps=cleanup) as cleaned,patch.object(session,'retire_directories',wraps=retire) as retired:
                        outcome=session.run(spec)
                    self.assertEqual(cleaned.call_count,1);self.assertEqual(retired.call_count,1)
                    self.assertTrue(outcome['cleanup_confirmed'] and outcome['transport_retired'])
                    self.assertFalse(directory.exists());self.assertIsNone(sibling.poll())
                    if mode=='ready':
                        self.assertIsNone(outcome['error']);self.assertEqual(outcome['vendor_retirement']['milestones'],127);self.assertEqual(outcome['retirement_disposition'],'process_scoped_vendor_retirement')
                    else:self.assertIsNotNone(outcome['error']);self.assertIsNone(outcome['vendor_retirement'])
                finally:sibling.terminate();sibling.wait(timeout=3)

    def test_non_dsp_routes_do_not_inherit_final_instance_retirement(self):
        for route in ('inspect','vendor_access'):
            with self.subTest(route=route),tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);sid='28'*16
                directory=root/'compatdata/pfx/drive_c/bridge/sessions'/sid;directory.mkdir(parents=True,mode=0o700)
                artifact=root/'host';artifact.write_bytes(b'fixture')
                binding={'path':str(artifact),'sha256':hashlib.sha256(b'fixture').hexdigest()}
                spec={'registration':{'host':binding,'module':binding,'environment':{'root':str(root),'runner':{'files':[]}}},'session':sid,'directory':str(directory),'report':str(root/'report.json'),'inspect':route=='inspect','vendor_access':route=='vendor_access'}
                program="import os;assert 'LVB_VENDOR_RETIREMENT' not in os.environ;print('{\"event\":\"lifecycle\",\"state\":\"scanner_completed\"}',flush=True)"
                with patch.object(session,'command',return_value=([sys.executable,'-c',program],b'')),patch.object(session,'environment',return_value=dict(os.environ,LVB_VENDOR_RETIREMENT='process_scoped_vendor_retirement')):
                    outcome=session.run(spec)
                self.assertIsNone(outcome['error']);self.assertIsNone(outcome['vendor_retirement'])
                self.assertTrue(outcome['cleanup_confirmed'] and outcome['transport_retired'])
