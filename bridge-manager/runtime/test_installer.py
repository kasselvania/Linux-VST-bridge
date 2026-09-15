"""MF2 production installer loop; replace launch/cgroup OS boundaries only."""
import json,os,pathlib,signal,subprocess,sys,tempfile,unittest
from unittest.mock import patch
import session,ownership
from test_session import CgroupFixture

class Scope:
    def __init__(self,child,tail=0,clean=True):self.child=child;self.tail=tail;self.clean=clean;self.started=False;self.samples=[]
    def members(self):
        if not self.started:self.started=True;return []
        if self.child.poll() is None:rows=[{'pid':self.child.pid,'state':'S'}]
        elif self.tail:self.tail-=1;rows=[{'pid':999,'state':'S'}]
        else:rows=[]
        self.samples.append((self.child.returncode,len(rows)));return rows
    def cleanup(self,reap):
        if self.child.poll() is None:self.child.terminate();self.child.wait(timeout=3)
        reap();return self.clean

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
                root=pathlib.Path(tmp);(root/'operation.lock').touch();(root/'home').mkdir();artifact=root/'installer';artifact.write_bytes(b'fixture')
                a={'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()}
                spec={'schema':2,'operation':'ab'*16,'environment':{'root':str(root),'runner':{'entry_point':'fixed-runner','proton':'fixed-proton','files':[]}},'installer':a,'format':'pe_executable','report':str(root/'result.json')}
                child=subprocess.Popen([sys.executable,'-c',f'import time;time.sleep(.05);print("private installer payload");raise SystemExit({code})'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                scope=Scope(child,tail=4);old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)]
                try:
                    with patch.object(session,'CompanionCgroup',return_value=scope),patch.object(session.subprocess,'Popen',return_value=child) as launch,patch.object(session,'environment',return_value={}):
                        self.assertEqual(session.install(spec),code==0)
                    self.assertIn((code,1),scope.samples);v=json.loads((root/'result.json').read_text());self.assertTrue(v['cleanup_confirmed']);self.assertEqual(v['owned_live'],0);self.assertEqual(v['raw_exit'],code);self.assertEqual(v['state'],'completed' if code==0 else 'failed');self.assertNotIn('private installer payload',(root/'result.json').read_text());self.assertGreater(v['discarded_diagnostic_bytes'],0)
                    self.assertEqual(launch.call_args.args[0],['fixed-runner','--verb=run','--','fixed-proton','run',str(artifact)])
                    self.assertEqual(launch.call_args.kwargs['env']['HOME'],str(root/'home'))
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    if child.poll() is None:child.kill();child.wait()
                    session.ctypes.CDLL(None).prctl(36,0,0,0,0)
    @unittest.skipUnless(sys.platform.startswith('linux'),'Linux supervisor subreaper')
    def test_launch_error_is_terminal_and_cleanup_is_not_fabricated(self):
        for clean in [True,False]:
            with self.subTest(clean=clean),tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);(root/'operation.lock').touch();artifact=root/'installer';artifact.write_bytes(b'fixture')
                class Empty:
                    def members(self):return []
                    def cleanup(self,reap):return clean
                spec={'schema':2,'operation':'ab'*16,'environment':{'root':str(root),'runner':{'entry_point':'fixed','proton':'fixed','files':[]}},'installer':{'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()},'format':'msi_compound','report':str(root/'result.json')}
                old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)]
                try:
                    with patch.object(session,'CompanionCgroup',return_value=Empty()),patch.object(session.subprocess,'Popen',side_effect=OSError('private path must not leak')),patch.object(session,'environment',return_value={}):self.assertFalse(session.install(spec))
                    v=json.loads((root/'result.json').read_text());self.assertEqual(v['state'],'failed' if clean else 'cleanup_unconfirmed');self.assertEqual(v['cleanup_confirmed'],clean);self.assertNotIn('private path',(root/'result.json').read_text())
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    session.ctypes.CDLL(None).prctl(36,0,0,0,0)
