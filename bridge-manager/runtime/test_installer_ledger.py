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
    def test_durable_installed_partial_and_not_installed_independent_of_exit(self):
        empty={k:{} for k in ('files','logs','services','uninstall')};empty['incomplete']=[]
        self.assertEqual(session.InstallerWitnesses.compare(empty,empty)['classification'],'not_installed')
        partial={**empty,'files':{'ProgramData/cache/prerequisite.msi':{'sha256':'ab'}}}
        self.assertEqual(session.InstallerWitnesses.compare(empty,partial)['classification'],'partial_installation')
        installed={**partial,'files':{'Program Files/fixture/app.exe':{'sha256':'cd'}},'uninstall':{'key':{'InstallLocation':'C:\\Program Files\\fixture'}}}
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
