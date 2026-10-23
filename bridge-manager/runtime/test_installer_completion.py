"""Retained-handle status cannot be supplied by trace, cleanup or outer zero."""
import json,os,pathlib,signal,subprocess,sys,tempfile,time,unittest
from unittest.mock import patch
import session
import test_installer_is2 as is2
from test_installer_first_launch import PROFILE,app_tree,create,STARTED

class CompletionTests(unittest.TestCase):
    op=is2.IS2Tests.op;token=is2.IS2Tests.token;digest=is2.IS2Tests.digest
    def trace(self,root='/owned/environment'):
        t=session.InstallerWindowsTrace({'path':'/owned/installer.exe','sha256':self.digest},root)
        t.arm_root(self.op,self.token,100,completion=True,adapter_sha256='d'*64);t.begin();t.begin()
        t.root_frame(is2.IS2Tests().frame());t.rows[0]['created_timestamp']='9.000';return t
    def frame(self,**changes):
        fields=dict(op=self.op,token=self.token,epoch=2,digest=self.digest,size=100,pid=32,created=100,
            adapter=16,adapter_created=99,began=10000,ended=70000,status=0)
        fields.update(changes)
        return ('IS2_EXIT_V1 '+' '.join(map(str,fields.values()))+'\n').encode()
    def test_exact_held_root_status_is_distinct_from_legacy_self_exit(self):
        t=self.trace();self.assertIsNone(t.completion()['exit_status'])
        is2.IS2Tests().exit(t,32);self.assertIsNone(t.root_success())
        t.root_frame(self.frame());self.assertEqual(t.completion()['status'],'succeeded')
        self.assertEqual(t.root_success(),{'started':9000,'ended':70000,'milliseconds':True})
        t=self.trace();t.root_frame(self.frame());self.assertIsNone(t.rows[0]['self_exit'])
        self.assertEqual(t.completion()['status'],'succeeded')
    def test_every_binding_field_version_and_numeric_bound_is_required(self):
        for change in [dict(op='d'*32),dict(token='e'*64),dict(epoch=3),dict(digest='f'*64),dict(size=101),
            dict(pid=33),dict(created=101),dict(adapter=17),dict(adapter_created=98),dict(began=70001),
            dict(ended=2**32),dict(ended=3610001),dict(status=2**32)]:
            with self.subTest(change=change):
                t=self.trace();t.root_frame(self.frame(**change));self.assertEqual(t.completion()['status'],'unconfirmed')
        for frame in [self.frame().replace(b'EXIT_V1',b'EXIT_V2'),self.frame()[:-1],b'IS2_EXIT_V1 bad\n']:
            t=self.trace();t.root_frame(frame);self.assertEqual(t.completion()['status'],'unconfirmed')
        t=self.trace();t.begin();t.root_frame(self.frame());self.assertIsNone(t.root_success())
    def test_nonzero_including_signaled_259_is_terminal_failure(self):
        for code in [23,259,0x80000000,0xffffffff]:
            t=self.trace();t.root_frame(self.frame(status=code))
            self.assertEqual(t.completion()['status'],'failed');self.assertEqual(t.first_failure['status'],code)
            t.cancelled=True;t.root_frame(self.frame());self.assertEqual(t.first_failure['status'],code)
    def test_duplicate_and_conflict_refuse_even_after_legacy_row_retirement(self):
        for first in ['handle','trace']:
            t=self.trace()
            if first=='trace':is2.IS2Tests().exit(t,32,23)
            t.root_frame(self.frame())
            if first=='handle':is2.IS2Tests().exit(t,32,23)
            self.assertEqual(t.completion()['status'],'unconfirmed')
        t=self.trace();is2.IS2Tests().exit(t,32);is2.IS2Tests().exit(t,32,23);t.root_frame(self.frame())
        self.assertEqual(t.completion()['status'],'unconfirmed')
        t=self.trace();t.root_frame(self.frame());t.root_frame(self.frame())
        self.assertEqual(t.completion()['status'],'unconfirmed')
        t=self.trace();t.root_frame(self.frame());is2.IS2Tests().create(t,16,32)
        self.assertEqual(t.completion()['status'],'unconfirmed')
    def test_cancellation_and_timeout_stay_unsuccessful_through_late_zero(self):
        with tempfile.TemporaryDirectory() as d:
            tx=session.InstallerTransaction(self.op,pathlib.Path(d),pathlib.Path(d)/'result.json')
            tx.windows_trace=self.trace();tx.durable={'classification':'installed'}
            tx.windows_trace.cancelled=True;tx.windows_trace.root_frame(self.frame())
            self.assertEqual(tx.summary(0,True,True)['outcome'],'cancelled')
            tx.windows_trace=self.trace();tx.windows_trace.root_frame(self.frame())
            self.assertEqual(tx.summary(0,True,False,error='installer_time_bound')['outcome'],'failed')
            self.assertIsNone(tx.windows_trace.root_success())
            self.assertEqual(tx.summary(0,True,False)['outcome'],'failed')
            tx.windows_trace=self.trace()
            self.assertEqual(tx.summary(0,True,False)['outcome'],'installer_completion_unconfirmed')
            self.assertEqual(tx.summary(0,False,False)['outcome'],'cleanup_unconfirmed')
    def test_only_stderr_complete_bounded_frames_confer_authority(self):
        for stream,frame in [('stdout',self.frame()),('stderr',self.frame()[:-1]),
            ('stderr',b'IS2_EXIT_V1 '+b'x'*5000+b'\n')]:
            with tempfile.TemporaryDirectory() as d:
                tx=session.InstallerTransaction(self.op,pathlib.Path(d),pathlib.Path(d)/'result.json')
                tx.windows_trace=self.trace();tx.feed(frame,stream);tx.finish()
                self.assertEqual(tx.windows_trace.completion()['status'],'unconfirmed')
        with tempfile.TemporaryDirectory() as d:
            tx=session.InstallerTransaction(self.op,pathlib.Path(d),pathlib.Path(d)/'result.json')
            tx.windows_trace=self.trace();frame=self.frame();tx.feed(frame[:90],'stderr');tx.feed(frame[90:],'stderr')
            self.assertEqual(tx.windows_trace.completion()['status'],'succeeded')
            tx.feed(frame[:-1],'stderr');tx.finish();self.assertIsNone(tx.windows_trace.root_success())
    def test_integer_app_close_window_is_conservative_and_has_no_upper_bound(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d);app_tree(root)
            for created,eligible in [('9.999',False),('10.000',True),('70.001',True)]:
                t=self.trace(root);t.feed(create(created).encode());t.feed(STARTED.replace('1.001',created).encode())
                t.root_frame(self.frame())
                with patch.object(session,'mapped_file_identity',return_value=(1,2,3)):
                    self.assertEqual(bool(session.installer_started_profiled_images(t,[PROFILE])),eligible)
            t=self.trace(root);t.root_frame(self.frame())
            for start in [None,'10.001','4294967.296','9.0001']:
                t.rows[0]['created_timestamp']=start;self.assertIsNone(t.root_success())
            t.rows[0]['created_timestamp']='9.000';t.root_frame(self.frame(ended=70001))
            self.assertIsNone(t.root_success()) # duplicate never widens eligibility

@unittest.skipUnless(sys.platform.startswith('linux'),'Linux installer supervisor')
class CompletionLoopTests(unittest.TestCase):
    def test_real_loop_keeps_cancel_deadline_and_missing_completion_unsuccessful(self):
        from test_installer import Scope
        for mode in ['valid','missing','conflicting','cancelled','deadline']:
            with self.subTest(mode=mode),tempfile.TemporaryDirectory() as d:
                root=pathlib.Path(d);(root/'operation.lock').touch();(root/'home').mkdir()
                artifact=root/'installer.exe';artifact.write_bytes(b'fixture')
                a={'path':str(artifact),'sha256':session.hashlib.sha256(b'fixture').hexdigest()}
                op='ab'*16
                spec={'schema':2,'operation':op,'environment':{'root':str(root),'runner':{
                    'entry_point':'fixed','proton':'fixed','files':[]}},'installer':a,'installer_launch':a,
                    'format':'pe_executable','report':str(root/'result.json')}
                real_popen=subprocess.Popen;children=[];real_clock=time.monotonic;offset=[0];transactions=[]
                real_transaction=session.InstallerTransaction;real_harvest=session.InstallerLedger.harvest
                released=root/'stand-in-released'
                def transaction(*args):
                    tx=real_transaction(*args);transactions.append(tx);return tx
                def launch(args,**kwargs):
                    if not children:
                        (root/'compatdata/pfx').mkdir(parents=True);(root/'compatdata/pfx/system.reg').touch()
                        code='print("prefix ready")'
                    else:
                        lines=(root/(op+'-launch-request.private')).read_bytes().decode('utf-16le').splitlines()
                        self.assertEqual(lines[0],'IS2_LAUNCH_V2');token=lines[2]
                        common=f'{op} {token} 2 {a["sha256"]} 7 32 100 16 99'
                        trace=f'IS2_ROOT_V1 {common}\n'
                        if mode!='missing':trace+=f'IS2_EXIT_V1 {common} 10000 11000 0\n'
                        if mode=='conflicting':trace+=f'IS2_EXIT_V1 {common} 10000 11000 23\n'
                        code=f'import sys,time;sys.stderr.write({trace!r});sys.stderr.flush();time.sleep(.15)'
                        if mode in ('cancelled','deadline'):
                            # The root's exit is reported, but the stand-in stays
                            # until the test has cancelled or passed the bound: a
                            # slow runner must not see it exit first (#502).
                            code=(f'import os,sys,time;sys.stderr.write({trace!r});sys.stderr.flush();'
                                f'end=time.monotonic()+30\nwhile not os.path.exists({str(released)!r}):\n'
                                ' if time.monotonic()>end:sys.exit(73)\n time.sleep(.005)')
                    child=real_popen([sys.executable,'-c',code],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    children.append(child);return child
                def harvest(ledger):
                    live=real_harvest(ledger)
                    if mode in ('cancelled','deadline') and transactions and transactions[0].windows_trace.handle_exit is not None and not offset[0]:
                        if mode=='cancelled':os.kill(os.getpid(),signal.SIGTERM);offset[0]=.001
                        if mode=='deadline':offset[0]=3601
                        released.touch()
                    return live
                old=[signal.getsignal(s) for s in (signal.SIGTERM,signal.SIGINT)]
                try:
                    with patch.object(session,'CompanionCgroup',return_value=Scope()),\
                        patch.object(session.subprocess,'Popen',side_effect=launch),\
                        patch.object(session,'environment',return_value={}),\
                        patch.object(session,'InstallerTransaction',side_effect=transaction),\
                        patch.object(session.InstallerLedger,'harvest',harvest),\
                        patch.object(session.time,'monotonic',side_effect=lambda:real_clock()+offset[0]),\
                        patch.object(session.InstallerWitnesses,'compare',return_value={'classification':'installed','delta':{'logs':{'added':[],'changed':[]}}}),\
                        patch.object(session,'installer_retire_profiled') as close:
                        self.assertEqual(session.managed_install(spec),mode=='valid')
                    close.assert_not_called()
                    value=json.loads((root/'result.json').read_text());self.assertTrue(value['cleanup_confirmed'])
                    self.assertEqual(value['state'],'completed' if mode=='valid' else 'cancelled' if mode=='cancelled' else 'failed')
                    self.assertEqual(value['transaction']['outcome']=='installed',mode=='valid')
                    if mode=='deadline':self.assertEqual(value['error'],'installer_time_bound')
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    for child in children:
                        if child.poll() is None:child.kill();child.wait()
                    session.ctypes.CDLL(None).prctl(36,0,0,0,0)

if __name__=='__main__':unittest.main()
