"""MF2 production installer loop; replace launch/cgroup OS boundaries only."""
import json,os,pathlib,signal,subprocess,sys,tempfile,unittest
from unittest.mock import patch
import session,ownership
from test_session import CgroupFixture

class Scope:
    # Actual Linux fork tree for the OS-boundary test. Dedicated-unit identity
    # validation itself is independently tested with CompanionCgroup fixtures.
    proc_root=pathlib.Path('/proc')
    def __init__(self):self.started=False;self.samples=[];self.supervisor=os.getpid()
    identity=ownership.CompanionCgroup.identity
    def group_of(self,pid):return '/test-owned' if self.identity(pid) else None
    def contains(self,group):return group=='/test-owned'
    def members(self):
        rows=[]
        for r in ownership.descendant_identities(self.supervisor):
            v=self.identity(r['pid'])
            if v:rows.append(dict(v,cgroup='/test-owned'))
        self.samples.append(len(rows));return rows
    def signal_members(self,sig):
        for r in self.members():
            try:os.kill(r['pid'],sig)
            except ProcessLookupError:pass

class InstallerTests(unittest.TestCase):
    def test_launcher_exit_between_poll_and_reap_preserves_nonzero_status(self):
        class Child:
            pid=42
            returncode=None
            def poll(self):return None
        child=Child()
        with patch.object(session.os,'waitpid',side_effect=[(42,23<<8),(0,0)]):session.installer_reap(child)
        self.assertEqual(child.returncode,23)

    def test_cgroup_is_bound_to_operation_not_another_unit(self):
        with tempfile.TemporaryDirectory() as tmp:
            class InstallerCgroup(CgroupFixture):group='/user.slice/linux-vst-bridge-installer-'+('ab'*16)+'.service'
            f=InstallerCgroup(pathlib.Path(tmp))
            for op in ['cd'*16,'../foreign','a'*31]:
                with self.assertRaises(RuntimeError):ownership.CompanionCgroup(proc_root=f.proc,cgroup_root=f.cg,supervisor=f.supervisor,installer_operation=op)
            c=ownership.CompanionCgroup(proc_root=f.proc,cgroup_root=f.cg,supervisor=f.supervisor,installer_operation='ab'*16)
            f.record(101,33,1);f.record(102,44,1,group='/user.slice/unrelated.service');f.set_members([101,102]);self.assertEqual([r['pid'] for r in c.members()],[101])
    @unittest.skipUnless(sys.platform.startswith('linux'),'Linux supervisor subreaper')
    def test_actual_installer_loop_waits_for_descendants_and_separates_failure(self):
        for code in [0,23]:
            with self.subTest(code=code),tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);(root/'operation.lock').touch();(root/'home').mkdir();(root/'compatdata/pfx').mkdir(parents=True);(root/'compatdata/pfx/system.reg').touch();artifact=root/'installer';artifact.write_bytes(b'fixture')
                a={'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()}
                spec={'schema':2,'operation':'ab'*16,'environment':{'root':str(root),'runner':{'entry_point':'fixed-runner','proton':'fixed-proton','files':[]}},'installer':a,'format':'pe_executable','report':str(root/'result.json')}
                real_popen=subprocess.Popen;children=[]
                def launch_owned(args,**kwargs):
                    program='print("prefix ready")' if len(children)==0 else f'import os,time;pid=os.fork();time.sleep(.2 if pid else .4);print("private installer payload");raise SystemExit({code} if pid else 0)'
                    child=real_popen([sys.executable,'-c',program],stdout=subprocess.PIPE,stderr=subprocess.PIPE);children.append(child);return child
                scope=Scope();old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)]
                try:
                    with patch.object(session,'CompanionCgroup',return_value=scope),patch.object(session.subprocess,'Popen',side_effect=launch_owned) as launch,patch.object(session,'environment',return_value={}):
                        self.assertEqual(session.install(spec),code==0)
                    self.assertGreater(max(scope.samples),1);v=json.loads((root/'result.json').read_text());self.assertTrue(v['cleanup_confirmed']);self.assertEqual(v['owned_live'],0);self.assertEqual(v['raw_exit'],code);self.assertEqual(v['state'],'completed' if code==0 else 'failed');self.assertNotIn('private installer payload',(root/'result.json').read_text());self.assertGreater(v['retained_diagnostic_bytes'],0);self.assertTrue(v['private_diagnostics_written'])
                    self.assertEqual(launch.call_args.args[0],['fixed-runner','--verb=run','--','fixed-proton','runinprefix',str(artifact)])
                    self.assertEqual(launch.call_args.kwargs['env']['HOME'],str(root/'home'))
                    self.assertEqual(launch.call_args_list[0].args[0],['fixed-runner','--verb=run','--','fixed-proton','getcompatpath','/'])
                    self.assertEqual(launch.call_count,2)
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    for child in children:
                        if child.poll() is None:child.kill();child.wait()
                    session.ctypes.CDLL(None).prctl(36,0,0,0,0)
    @unittest.skipUnless(sys.platform.startswith('linux'),'Linux supervisor subreaper')
    def test_launch_error_is_terminal_and_cleanup_is_not_fabricated(self):
        for clean in [True,False]:
            with self.subTest(clean=clean),tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);(root/'operation.lock').touch();artifact=root/'installer';artifact.write_bytes(b'fixture')
                class Empty:
                    def members(self):
                        if not clean:raise RuntimeError('unavailable cgroup')
                        return []
                spec={'schema':2,'operation':'ab'*16,'environment':{'root':str(root),'runner':{'entry_point':'fixed','proton':'fixed','files':[]}},'installer':{'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()},'format':'msi_compound','report':str(root/'result.json')}
                old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)]
                try:
                    with patch.object(session,'CompanionCgroup',return_value=Empty()),patch.object(session.subprocess,'Popen',side_effect=OSError('private path must not leak')),patch.object(session,'environment',return_value={}):self.assertFalse(session.install(spec))
                    v=json.loads((root/'result.json').read_text());self.assertEqual(v['state'],'failed' if clean else 'cleanup_unconfirmed');self.assertEqual(v['cleanup_confirmed'],clean);self.assertNotIn('private path',(root/'result.json').read_text())
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    session.ctypes.CDLL(None).prctl(36,0,0,0,0)

class StartupTests(unittest.TestCase):
    def test_fault_survives_modal_lifetime_cancellation_and_cleanup(self):
        s=session.InstallerStartup('ab'*16,{'path':'/unused','sha256':'cd'*32},'/private')
        for chunk in [b'0040:err:steamclient:steamclient_init unable to load ',b'native steamclient library\n']:
            s.feed(chunk)
        first=s.value()['first_problem'].copy()
        self.assertEqual(first['code'],'native_steamclient_load_failed')
        self.assertIsNone(s.value()['cancellation'])
        s.feed(b'Assertion failed!\n');s.cancellation();s.cancellation();s.stage('cohort_retired',outer_exit=-15)
        self.assertEqual(s.value()['first_problem'],first)
        self.assertEqual(sum(x['stage']=='cancellation_requested' for x in s.rows),1)
        self.assertEqual(s.value()['target_observation'],'unknown')
        self.assertEqual(s.value()['exception_stack'],'unavailable')
    def test_missing_diagnostics_and_saturation_never_infer_assertion(self):
        s=session.InstallerStartup('ab'*16,{'path':'/unused','sha256':'cd'*32},'/private')
        s.feed(b'x'*8000+b'\n');s.cancellation();s.stage('cohort_retired',outer_exit=-15)
        self.assertIsNone(s.value()['first_problem']);self.assertEqual(s.line_drops,1)
        for _ in range(1000):s.stage('bounded')
        self.assertEqual(len(s.rows),48);self.assertGreater(s.dropped,0)
        self.assertNotIn('/private',json.dumps(s.value()))

    def test_stdout_and_stderr_fragments_cannot_fabricate_loader_failure(self):
        s=session.InstallerStartup('ab'*16,{'path':'/unused','sha256':'cd'*32},'/private')
        s.feed(b'0040:err:steamclient:steamclient_init unable to load ', 'stdout')
        s.feed(b'native steamclient library\n','stderr')
        self.assertIsNone(s.value()['first_problem'])
        s.feed(b'native steamclient library\n','stdout')
        self.assertEqual(s.value()['first_problem']['code'],'native_steamclient_load_failed')

    def test_oversized_line_suffix_cannot_become_a_new_diagnostic(self):
        s=session.InstallerStartup('ab'*16,{'path':'/unused','sha256':'cd'*32},'/private')
        s.feed(b'x'*5000)
        s.feed(b'0040:err:steamclient:steamclient_init unable to load native steamclient library\n')
        self.assertIsNone(s.first_problem)
        s.feed(b'0040:err:steamclient:steamclient_init unable to load native steamclient library\n')
        self.assertEqual(s.first_problem['code'],'native_steamclient_load_failed')
