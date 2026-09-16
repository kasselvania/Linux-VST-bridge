"""IS1 exact process custody and durable outcome; no product fixture."""
import json,os,pathlib,signal,sys,tempfile,unittest
from types import SimpleNamespace
from unittest.mock import patch
import ownership,session

class Scope:
    def __init__(self):self.rows={};self.listed=set();self.supervisor=900
    def identity(self,pid):return self.rows.get(pid)
    def group_of(self,pid):return '/owned' if pid in self.rows else None
    def contains(self,group):return group=='/owned'
    def members(self):return [dict(self.rows[p],cgroup='/owned') for p in self.listed]
    def add(self,pid,start,ppid=900,state='S',code=None,listed=True):
        self.rows[pid]=dict(pid=pid,start_ticks=start,ppid=ppid,state=state,exit_code=code)
        if listed:self.listed.add(pid)

class LedgerTests(unittest.TestCase):
    def test_wnowait_short_child_is_persisted_before_reap(self):
        scope=Scope();scope.add(42,100,state='Z',code=37<<8,listed=False)
        order=[];ledger=ownership.InstallerLedger(scope,lambda v:order.append(('persist',json.loads(json.dumps(v)))))
        info=SimpleNamespace(si_pid=42,si_status=37,si_code=1)
        def wait(*_):
            self.assertTrue(any(v['first_failure'] for k,v in order));scope.rows.pop(42);order.append(('reap',{}));return 42,37<<8
        with patch.object(ownership.os,'waitid',side_effect=[info,None],create=True),patch.object(ownership.os,'waitpid',side_effect=wait),patch.multiple(ownership.os,CLD_EXITED=1,P_ALL=0,WEXITED=4,WNOWAIT=16,create=True):
            self.assertEqual(ledger.harvest(),[])
        self.assertEqual(ledger.first_failure['status'],37);self.assertEqual(len(ledger.records),1)
        self.assertTrue(ledger.records[(42,100)]['linux_exit']['reaped'])
        self.assertIsNone(ledger.records[(42,100)]['windows_exit'])
    def test_exact_cgroup_and_start_not_recycled_pid_or_name(self):
        scope=Scope();scope.add(42,100);ledger=ownership.InstallerLedger(scope,lambda _:None);ledger.sample()
        scope.add(42,101);ledger.sample();self.assertEqual(len(ledger.records),2)
        self.assertEqual(ledger.records[(42,100)]['exit_availability'],'unavailable_parent_reaped_or_unobserved')
        self.assertIsNone(ledger.records[(42,101)]['parent'])
    def test_zombie_not_listed_is_still_sampled_before_reap(self):
        scope=Scope();scope.add(42,100);ledger=ownership.InstallerLedger(scope,lambda _:None);ledger.sample()
        scope.rows[42].update(state='Z',exit_code=41<<8);scope.listed.clear()
        self.assertEqual(len(ledger.sample()),1);self.assertEqual(ledger.first_failure['status'],41)
    def test_outer_and_child_cancellation_cleanup_are_separate(self):
        scope=Scope();scope.add(42,100);scope.add(43,101,42);ledger=ownership.InstallerLedger(scope,lambda _:None)
        outer=SimpleNamespace(pid=42,returncode=None);ledger.launcher(outer,'target_runner');ledger.sample()
        ledger.exit(ledger.records[(43,101)],37,'waitpid');first=ledger.first_failure.copy()
        ledger.cancelled=True;ledger.exit(ledger.records[(42,100)],-15,'waitpid')
        self.assertEqual(ledger.first_failure,first);self.assertEqual(first['relationship'],'descendant')
        scope.rows.clear();scope.listed.clear()
        with patch.object(ownership.os,'waitid',return_value=None,create=True),patch.multiple(ownership.os,P_ALL=0,WEXITED=4,WNOWAIT=16,create=True):self.assertTrue(ledger.cleanup())
        self.assertEqual(ledger.first_failure,first)
    def test_persistence_failure_cannot_obstruct_reap(self):
        scope=Scope();scope.add(42,100,state='Z',code=23<<8,listed=False)
        def fail(_):raise OSError('disk full')
        ledger=ownership.InstallerLedger(scope,fail)
        def reap(*_):scope.rows.clear();return 42,23<<8
        with patch.object(ownership.os,'waitid',side_effect=[SimpleNamespace(si_pid=42,si_status=23,si_code=1),None],create=True),patch.object(ownership.os,'waitpid',side_effect=reap),patch.multiple(ownership.os,CLD_EXITED=1,P_ALL=0,WEXITED=4,WNOWAIT=16,create=True):ledger.harvest()
        self.assertGreater(ledger.persistence_failures,0);self.assertEqual(ledger.first_failure['status'],23)
    def test_bounds_unknown_role_and_parent(self):
        scope=Scope();ledger=ownership.InstallerLedger(scope,lambda _:None);ledger.LIMIT=2
        for pid in range(3):scope.add(pid+40,pid+100)
        ledger.sample();self.assertEqual(len(ledger.records),2);self.assertGreater(ledger.dropped,0)
        self.assertTrue(all(r['role']=='unknown' for r in ledger.records.values()))

class WitnessTests(unittest.TestCase):
    def test_removal_only_durable_surfaces_are_partial_and_keep_exact_delta(self):
        for category,record in [('files',{'sha256':'ab','format':'pe_executable'}),
                                ('uninstall',{'InstallLocation':'C:\\Fixture'}),
                                ('services',{'Start':3})]:
            with self.subTest(category=category):
                empty={k:{} for k in ('files','logs','services','uninstall')}
                empty['incomplete']=[]
                before={**empty,category:{'removed-witness':record}}
                value=session.InstallerWitnesses.compare(before,empty)
                self.assertEqual(value['classification'],'partial_installation')
                self.assertEqual(value['delta'][category],{'added':[],'changed':[],'removed':['removed-witness']})
                installed={**empty,'files':{'Fixture/app.exe':{'sha256':'cd','format':'pe_executable'}},
                           'uninstall':{'application':{'InstallLocation':'C:\\Fixture'}}}
                self.assertEqual(session.InstallerWitnesses.compare(before,installed)['classification'],'installed')
        self.assertEqual(session.InstallerWitnesses.compare({**empty,'logs':{'log':{'sha256':'ab'}}},empty)['classification'],'not_installed')
    def test_live_operation_is_not_a_cleanup_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            tx=session.InstallerTransaction('ab'*16,tmp,pathlib.Path(tmp)/'r')
            live=tx.summary(None,False,False,ongoing=True)
            self.assertEqual(live['outcome'],'in_progress')
            self.assertEqual(live['safe_next_action'],'exact_owned_focus_or_stop')
            self.assertEqual(tx.summary(None,False,False)['outcome'],'cleanup_unconfirmed')
    def test_durable_installed_partial_and_not_installed_independent_of_exit(self):
        empty={k:{} for k in ('files','logs','services','uninstall')};empty['incomplete']=[]
        self.assertEqual(session.InstallerWitnesses.compare(empty,empty)['classification'],'not_installed')
        partial={**empty,'files':{'ProgramData/cache/prerequisite.msi':{'sha256':'ab'}}}
        self.assertEqual(session.InstallerWitnesses.compare(empty,partial)['classification'],'partial_installation')
        installed={**partial,'files':{'Program Files/fixture/app.exe':{'sha256':'cd','format':'pe_executable'}},'uninstall':{'key':{'InstallLocation':'C:\\Program Files\\fixture'}}}
        self.assertEqual(session.InstallerWitnesses.compare(empty,installed)['classification'],'installed')
        with tempfile.TemporaryDirectory() as tmp:
            tx=session.InstallerTransaction('ab'*16,tmp,pathlib.Path(tmp)/'r');tx.durable={'classification':'installed'}
            value=tx.summary(2,True,False);self.assertEqual(value['outcome'],'outer_nonzero_stage_unknown');self.assertEqual(value['durable_installation'],'installed')
    def test_diagnostics_noise_cannot_select_cause(self):
        with tempfile.TemporaryDirectory() as tmp:
            tx=session.InstallerTransaction('ab'*16,tmp,pathlib.Path(tmp)/'r')
            tx.feed(b'Xalia process disappeared exception\n','stderr');tx.feed(b'payload failed 2\n','stdout')
            self.assertGreater(tx.accessibility.bytes,0);self.assertIsNone(tx.summary(2,True,False)['first_failure'])
            for _ in range(1000):tx.feed(b'x'*5000+b'\n','stdout')
            self.assertEqual(tx.line_drops,1000);self.assertLess(tx.ordinary.bytes,262145)
    def test_registry_allowlist_and_no_account_capture(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp=str(pathlib.Path(tmp).resolve());prefix=pathlib.Path(tmp)/'compatdata/pfx';prefix.mkdir(parents=True)
            (prefix/'system.reg').write_text('[Software\\\\Account]\n"Token"="secret"\n[System\\\\ControlSet001\\\\Services\\\\Fixture]\n"ImagePath"="C:\\\\service.exe --token secret"\n"Start"=dword:00000003\n')
            (prefix/'user.reg').write_text('')
            value=session.InstallerWitnesses(tmp).snapshot();self.assertNotIn('secret',json.dumps(value));self.assertEqual(len(value['services']),1)
    def test_atomic_private_record_survives_stale_temporary(self):
        with tempfile.TemporaryDirectory() as tmp:
            path=pathlib.Path(tmp)/'record.json';path.with_suffix('.json.tmp').write_text('partial')
            session.installer_atomic(path,{'first_failure':37});session.installer_atomic(path,{'first_failure':37,'cleanup':True})
            self.assertEqual(json.loads(path.read_text())['first_failure'],37);self.assertEqual(path.stat().st_mode&0o777,0o600)

class WindowsTraceTests(unittest.TestCase):
    def setUp(self):
        self.trace=session.InstallerWindowsTrace({'path':'/fixture.exe','sha256':'ab'*32},'/private')
        self.trace.begin();self.tick=0
    def event(self,pid,function,body,channel='process'):
        self.tick+=1
        self.trace.feed(f'{self.tick}.000:{pid:04x}:{pid+4:04x}:trace:{channel}:{function} {body}'.encode())
    def request(self,parent):
        self.event(parent,'CreateProcessInternalW',r'app L"Z:\\fixture.exe" cmdline (null), inherit 0')
    def complete(self,parent,child):
        self.event(parent,'CreateProcessInternalW',f'started process pid {child:04x} tid {child+4:04x}')
    def create(self,parent,child):
        self.request(parent);self.complete(parent,child)
    def exit(self,pid,status=0):
        self.event(pid,'NtTerminateProcess',f'handle 0xffffffffffffffff, exit_code {status}, process_exiting 1.')
    def test_exited_root_unobserved_pid_reuse_has_no_child_or_failure_authority(self):
        self.create(32,232);self.exit(232)
        history=json.loads(json.dumps(self.trace.rows[0]))
        self.assertNotIn(232,self.trace.current)
        self.create(232,240);self.exit(240,37)
        self.assertFalse(self.trace.rows[1]['target_tree'])
        self.assertFalse(self.trace.rows[1]['target_root'])
        self.assertIsNone(self.trace.rows[1]['parent_ordinal'])
        self.assertIsNone(self.trace.first_failure)
        self.assertEqual(self.trace.rows[0],history)
    def test_pending_request_cannot_cross_exited_creator_generation(self):
        self.create(32,232);self.request(232);self.exit(232)
        self.assertNotIn((232,236),self.trace.pending)
        self.create(32,232) # fully observed replacement, not the pending creator
        self.complete(232,240);self.exit(240,37)
        self.assertFalse(self.trace.rows[-1]['target_tree'])
        self.assertIsNone(self.trace.rows[-1]['parent_ordinal'])
        self.assertIsNone(self.trace.first_failure)
    def test_observed_reused_pid_has_new_ordinal_and_independent_chain(self):
        self.create(32,232);self.create(232,240);self.exit(240);self.exit(232)
        history=json.loads(json.dumps(self.trace.rows))
        self.create(32,232);self.create(232,240)
        self.event(240,'StartServiceW','bounded API observation','service')
        self.exit(240,73)
        new_root,new_child=self.trace.rows[2:]
        self.assertEqual(new_root['creation_ordinal'],3)
        self.assertEqual(new_child['parent_ordinal'],3)
        self.assertTrue(new_child['target_tree'])
        self.assertEqual(self.trace.first_failure['creation_ordinal'],4)
        self.assertEqual(self.trace.first_failure['role'],'service_dependency')
        self.assertEqual(self.trace.rows[:2],history)
        self.assertEqual(new_child['linux_identity'],'unavailable_no_cross_id_inference')
    def test_service_evidence_and_repeated_exit_cannot_change_retired_generation(self):
        self.create(32,232);self.create(232,240);self.exit(240,37)
        history=json.loads(json.dumps(self.trace.value()))
        self.event(240,'CreateServiceW','bounded API observation','service')
        self.event(240,'StartServiceW','bounded API observation','service')
        self.exit(240,73)
        self.assertEqual(self.trace.value(),history)
        self.assertEqual(self.trace.first_failure['role'],'unknown')
    def test_pending_request_and_parent_cannot_cross_launch_epoch(self):
        self.create(32,232);self.request(232);self.trace.begin()
        self.complete(232,240);self.exit(240,37)
        self.assertFalse(self.trace.rows[-1]['target_tree'])
        self.assertIsNone(self.trace.first_failure)
    def test_exact_created_child_self_exit_is_separate_from_outer_and_noise(self):
        w=session.InstallerWindowsTrace({'path':'/fixture.exe','sha256':'ab'*32},'/private');w.begin()
        lines=[
            r'1.000:0020:0024:trace:process:CreateProcessInternalW app L"Z:\\fixture.exe" cmdline L"private arguments", inherit 1, flags 0, env 0',
            '1.010:0020:0024:trace:process:CreateProcessInternalW started process pid 00e8 tid 00ec',
            r'1.020:00e8:00ec:trace:process:CreateProcessInternalW app (null) cmdline L"\"Z:\\fixture.exe\" --child secret", inherit 0, flags 0, env 0',
            '1.030:00e8:00ec:trace:process:CreateProcessInternalW started process pid 00f0 tid 00f4',
            '1.040:00f0:00f4:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 37, process_exiting 1.',
            'Xalia exit_code 2 failure',
            '1.050:00e8:00ec:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 2, process_exiting 1.',
        ]
        for line in lines:w.feed(line.encode()+b'\n')
        self.assertEqual(w.first_failure['status'],37);self.assertEqual(w.first_failure['domain'],'wine_self_exit_observation')
        self.assertEqual(w.rows[1]['parent_ordinal'],w.rows[0]['creation_ordinal']);self.assertNotIn('secret',json.dumps(w.value()))
        w.cancelled=True;first=w.first_failure.copy();w.begin()
        w.feed(b'2.040:00f0:00f4:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 99, process_exiting 1.\n')
        self.assertEqual(w.first_failure,first);self.assertEqual(len(w.rows),2)
    def test_unassociated_exit_is_not_a_cgroup_or_stage_fact(self):
        w=session.InstallerWindowsTrace({'path':'/fixture.exe','sha256':'ab'*32},'/private');w.begin()
        w.feed(b'1.040:00f0:00f4:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 37, process_exiting 1.\n')
        self.assertIsNone(w.first_failure);self.assertEqual(w.rows,[])

class InstallerBoundaries(unittest.TestCase):
    def test_no_follow_parent_alias_and_oversized_image(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();real=root/'real';real.mkdir();(real/'app.exe').write_bytes(b'MZsafe')
            (root/'alias').symlink_to(real,target_is_directory=True)
            with self.assertRaises(OSError):session.installer_digest(root/'alias/app.exe')
            with self.assertRaises(ValueError):session.installer_digest(real/'app.exe',2)
    def test_service_role_requires_exact_same_created_process_api_evidence(self):
        w=session.InstallerWindowsTrace({'path':'/fixture.exe','sha256':'ab'*32},'/private');w.begin()
        for line in [r'1.000:0020:0024:trace:process:CreateProcessInternalW app L"Z:\\fixture.exe" cmdline (null), inherit 0',
            '1.010:0020:0024:trace:process:CreateProcessInternalW started process pid 00e8 tid 00ec',
            r'1.020:00e8:00ec:trace:process:CreateProcessInternalW app L"Z:\\fixture.exe" cmdline (null), inherit 0',
            '1.030:00e8:00ec:trace:process:CreateProcessInternalW started process pid 00f0 tid 00f4',
            '1.040:00f0:00f4:trace:service:StartServiceW 0000000000704540 0 0000000000000000',
            '1.050:00f0:00f4:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 73, process_exiting 1.']:
            w.feed(line.encode()+b'\n')
        self.assertEqual(w.first_failure['role'],'service_dependency')
        self.assertEqual(w.rows[-1]['role_evidence'][0]['function'],'StartServiceW')

    def test_accessibility_assertion_prose_does_not_become_startup_failure(self):
        s=session.InstallerStartup('ab'*16,{'path':'/unused','sha256':'cd'*32},'/private')
        s.feed(b'Xalia Assertion failed after process exited\n')
        self.assertIsNone(s.first_problem)

    def test_nonzero_child_does_not_override_successful_outer_with_guessed_error_domain(self):
        with tempfile.TemporaryDirectory() as tmp:
            tx=session.InstallerTransaction('ab'*16,tmp,pathlib.Path(tmp)/'r')
            tx.durable={'classification':'installed'}
            tx.windows_trace.first_failure={'role':'unknown','phase':'target_runner','relationship':'descendant','domain':'wine_self_exit_observation','status':3010,'cause':'unestablished'}
            value=tx.summary(0,True,False)
            self.assertEqual(value['outcome'],'installed')
            self.assertEqual(value['first_failure']['status'],3010)
            self.assertEqual(value['safe_next_action'],'review_retained_outcome_before_retry')

    def test_msi_sink_is_independent_bounded_and_never_a_process_cause(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp).resolve();tx=session.InstallerTransaction('ab'*16,root,root/'result.json')
            for _ in range(50):tx.feed(b'Action: source-owned MSI record\n'*1000,'msi')
            self.assertGreater(tx.msi.dropped_bytes,0);self.assertLessEqual(tx.msi.bytes,131072)
            self.assertEqual(tx.ordinary.bytes,0);self.assertIsNone(tx.windows_trace.first_failure)
            tx.finish();retained=root/(('ab'*16)+'-msi-private.log')
            self.assertTrue(retained.is_file());self.assertEqual(retained.stat().st_mode&0o777,0o600)
    def test_phase_boundary_cannot_join_two_incomplete_diagnostic_streams(self):
        with tempfile.TemporaryDirectory() as tmp:
            tx=session.InstallerTransaction('ab'*16,tmp,pathlib.Path(tmp)/'r')
            tx.feed(b'1.0:00f0:00f4:trace:process:NtTerminateProcess handle ','stderr')
            tx.begin_phase();tx.feed(b'0xffffffffffffffff, exit_code 37, process_exiting 1.\n','stderr')
            self.assertIsNone(tx.windows_trace.first_failure);self.assertEqual(tx.line_drops,1)

    def test_unquoted_command_needs_resolved_image_not_filename_guess(self):
        w=session.InstallerWindowsTrace({'path':'/fixture.exe','sha256':'ab'*32},'/private');w.begin()
        w.feed(br'1.000:0020:0024:trace:process:CreateProcessInternalW app (null) cmdline L"Z:\\fixture.exe secret", inherit 0')
        self.assertIsNone(w.pending[(32,36)]['image_request'])
        w.feed(br'1.010:0020:0024:trace:process:NtCreateUserProcess L"resolved" image L"Z:\\fixture.exe" cmdline L"secret" parent (nil) machine 0')
        w.feed(b'1.020:0020:0024:trace:process:CreateProcessInternalW started process pid 00e8 tid 00ec')
        self.assertTrue(w.rows[0]['target_root']);self.assertNotIn('secret',json.dumps(w.value()))
