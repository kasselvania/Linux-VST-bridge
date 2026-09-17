"""Generated NAUI2 owner tests; no commercial image or GUI is used."""
import copy,hashlib,importlib.util,json,os,pathlib,signal,subprocess,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).parent))
import session as s

class RendererTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=pathlib.Path(self.temp.name).resolve()
        self.path=self.root/'Native Access.exe';self.path.write_bytes(b'MZ'+b'x'*1024)
        self.artifact={'path':str(self.path),'sha256':hashlib.sha256(self.path.read_bytes()).hexdigest()}
        self.image=s.RendererImage({'artifact':self.artifact,'size':1026});self.addCleanup(self.image.close)
        self.e=s.RendererEvidence(self.artifact,self.root,self.image);t=self.e.trace
        t.arm_root('a'*32,'b'*64,1026);self.e.begin();self.e.begin()
        self.e.feed(('IS2_ROOT_V1 '+'a'*32+' '+'b'*64+' 2 '+self.artifact['sha256']+' 1026 100 1000 50 500\n').encode(),'stderr')
    def create(self,role='gpu-process',pid=200,ts='10.200'):
        image=s.windows(self.path,self.root/'compatdata/pfx').replace('\\','\\\\')
        data=f'10.100:0064:0001:trace:process:CreateProcessInternalW app L"{image}" cmdline L"app --type={role}", inherit 0\n{ts}:0064:0001:trace:process:CreateProcessInternalW started process pid {pid:04x} tid 0002\n'
        self.e.feed(data.encode(),'stderr')
    def log(self,body='GPU process exited unexpectedly: exit_code=34',source='gpu_process_host.cc'):
        return f'[200:2:0916/170000.123:ERROR:{source}(12)] {body}\n'.encode()
    def test_exact_root_role_and_exit(self):
        self.create();self.e.feed(b'11.000:00c8:0002:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 5, process_exiting 1.\n','stderr')
        v=self.e.value();self.assertTrue(v['complete']);self.assertEqual(v['cause'],'gpu')
        self.assertEqual(v['processes'][1]['role'],'gpu');self.assertEqual(v['processes'][1]['exit_status'],5)
        self.assertFalse(v['linux_windows_join_performed'])
    def test_all_categories_blocked_by_windows_loss(self):
        self.create()
        cases=[self.log(),self.log('Failed to launch child: error_code=39','child_process_launcher_helper.cc'),
               self.log('render-process-gone: crashed','electron_api_web_contents.cc'),b'10.300:00c8:0002:err:gdi:alloc_handle out of GDI object handles\n']
        for line in cases:
            self.e.facts=[];self.e.feed(line,'stderr');self.assertNotEqual(self.e.value()['cause'],'unresolved')
            self.e.trace.dropped=1;self.assertEqual(self.e.value()['cause'],'unresolved');self.e.trace.dropped=0
    def test_self_exit_loss_cannot_confer_authority(self):
        self.create();self.e.feed(b'11.000:00c8:0002:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 5, process_exiting 1.\n','stderr')
        self.e.trace.dropped=1;self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_reused_pid_invalidates_old_unique_diagnostic(self):
        self.create();self.e.feed(self.log(),'stderr')
        self.e.feed(b'11.000:00c8:0002:trace:process:NtTerminateProcess handle 0xffffffffffffffff, exit_code 0, process_exiting 1.\n','stderr')
        self.create(ts='12.000');self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_same_name_other_image_unknown(self):
        self.create();self.e.trace.rows[-1]['image_identity']['sha256']='c'*64
        self.e.feed(self.log(),'stderr');self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_missing_root_and_wrong_stream(self):
        self.e.trace.binding['status']='unavailable';self.create();self.e.feed(self.log(),'stderr');self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_saturation_and_truncation_unresolved(self):
        self.create();self.e.feed(self.log(),'stderr');self.e.feed(b'x'*5000,'stderr')
        self.assertFalse(self.e.value()['complete']);self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_cancellation_preserves_earlier_failure(self):
        self.create();self.e.feed(self.log(),'stderr');self.e.trace.cancelled=True
        self.e.feed(self.log('Failed to launch child: error_code=39','child_process_launcher_helper.cc'),'stderr')
        self.assertEqual(self.e.value()['cause'],'gpu')
    def test_competing_causes_unresolved(self):
        self.create();self.e.feed(self.log()+self.log('Failed to launch child: error_code=39','child_process_launcher_helper.cc'),'stderr');self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_log_prose_not_authority(self):
        self.create();self.e.feed(b'quote '+self.log(),'stderr');self.assertEqual(self.e.value()['cause'],'unresolved')
    def test_image_cache_replacement_mutation_alias_extent(self):
        self.path.write_bytes(b'MZ'+b'y'*1024)
        with self.assertRaises(ValueError):self.image.check()
        for size in (True,0,-1,257*1024*1024):
            with self.assertRaises(ValueError):s.RendererImage({'artifact':self.artifact,'size':size})
        alias=self.root/'alias';alias.symlink_to(self.path)
        with self.assertRaises(ValueError):s.RendererImage({'artifact':dict(self.artifact,path=str(alias)),'size':1026})
    def test_legacy_launch_and_policy_unselected_unchanged(self):
        app={'environment':{'root':str(self.root),'runner':{'entry_point':'entry','proton':'proton'}},'executable':self.artifact,'helpers':[self.artifact]}
        argv,cwd=s.vendor_launch({'application':app})
        self.assertEqual(argv,['entry','--verb=run','--','proton','runinprefix',s.windows(self.path,self.root/'compatdata/pfx')])
        self.assertEqual(cwd,self.path.parent)
    def test_closed_spec_refuses_before_launch(self):
        for spec in ({},{'schema':1,'kind':'renderer_application','arguments':['--no-sandbox']}):
            with patch.object(s.subprocess,'Popen') as launch:
                with self.assertRaises(ValueError):s.renderer_application(spec)
                launch.assert_not_called()

    def test_independent_classifier_completeness_and_closed_process_schema(self):
        self.create();self.e.feed(self.log(),'stderr');v=self.e.value()
        classify=lambda p,f,d,c:s.renderer_cause(p,f,d,c,self.image.sha)
        self.assertEqual(classify(v['processes'],v['facts'],0,True),'gpu')
        self.assertEqual(classify(v['processes'],v['facts'],1,True),'unresolved')
        for d in (None,True,-1,1.2,4294967296):
            with self.assertRaises(ValueError):classify(v['processes'],v['facts'],d,True)
        for key in v['processes'][1]:
            rows=copy.deepcopy(v['processes']);del rows[1][key]
            with self.assertRaises(ValueError):classify(rows,v['facts'],0,True)
        for field,value in [('domain','linux'),('epoch',1),('ordinal',1),('role','other'),('role_request_sha256',None),('image_sha256','bad'),('exit_domain','linux_wait'),('exit_status',True)]:
            rows=copy.deepcopy(v['processes']);rows[1][field]=value
            with self.assertRaises(ValueError):classify(rows,v['facts'],0,True)
        for authority in ('wine_pid_lifetime_complete_trace','unknown'):
            facts=copy.deepcopy(v['facts']);facts[0]['authority']=authority
            with self.assertRaises(ValueError):classify(v['processes'],facts,0,True)
    def test_runner_tail_loss_does_not_change_complete_positive_authority(self):
        self.create();self.e.feed(self.log(),'stderr')
        # Text sink retention is independent of streaming Windows parse authority.
        capture=s.PrivateCapture(self.root/'bounded.private',8,60)
        capture.write(b'x'*100);capture.close();self.assertGreater(capture.discarded,0)
        self.assertEqual(self.e.value()['cause'],'gpu')
    @unittest.skipUnless(sys.platform.startswith('linux'),'Linux production ledger/subreaper')
    def test_production_creation_failure_and_exact_positive_receipt(self):
        from test_installer import Scope
        for fail in (True,False):
            with self.subTest(fail=fail):
                (self.root/'compatdata/pfx').mkdir(parents=True,exist_ok=True)
                (self.root/'compatdata/pfx/system.reg').touch()
                op=('d' if fail else 'e')*32;report=self.root/(op+'.json')
                app={'environment':{'root':str(self.root),'runner':{'entry_point':'fixture','proton':'fixture'}},
                     'files':{'Native Access.exe':{'artifact':self.artifact,'size':1026}}}
                spec={'operation':op,'application':app,'renderer_policy':'software_rendering','application_identity':'f'*64,
                      'software_sha256':'c'*64,'installer_launch':{'path':'adapter'},'report':str(report)}
                real=subprocess.Popen;children=[];old=[signal.getsignal(sig) for sig in (signal.SIGTERM,signal.SIGINT)]
                def launch(*args,**kwargs):
                    self.assertEqual(kwargs['cwd'],self.path.parent)
                    request=(self.root/(op+'-launch.private')).read_bytes().decode('utf-16le').splitlines()
                    self.assertEqual(request[0],'NAUI2_LAUNCH_V1');self.assertEqual(request[7],'software_rendering')
                    if fail:raise OSError('generated launch failure')
                    frame='IS2_ROOT_V1 '+' '.join(request[1:6])+' 100 1000 50 500\n'
                    child=real([sys.executable,'-c','import sys;sys.stderr.write('+repr(frame)+')'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    children.append(child);return child
                try:
                    with patch.object(s,'renderer_validate',return_value=app),patch.object(s,'CompanionCgroup',return_value=Scope()),patch.object(s,'environment',return_value={}),patch.object(s.subprocess,'Popen',side_effect=launch):
                        self.assertEqual(s.renderer_application(spec),not fail)
                    r=json.loads(report.read_text());self.assertEqual(r['requested'],'software_rendering')
                    self.assertEqual(r['effective'] is None,fail);self.assertEqual(r['state'],'failed' if fail else 'completed')
                    self.assertTrue(r['cleanup_confirmed']);self.assertEqual(r['owned_live'],0)
                    if not fail:self.assertEqual(r['effective']['operation'],op)
                finally:
                    for sig,handler in zip((signal.SIGTERM,signal.SIGINT),old):signal.signal(sig,handler)
                    for child in children:
                        if child.poll() is None:child.kill();child.wait()
                    s.ctypes.CDLL(None).prctl(36,0,0,0,0)
    def test_rejected_spec_retains_exact_empty_unit_receipt(self):
        spec={'operation':'a'*32,'report':str(self.root/'refused.json'),'renderer_policy':'inherited'}
        class Empty:
            def members(self):return []
        with patch.object(s,'CompanionCgroup',return_value=Empty()),patch.object(s.subprocess,'Popen') as launch:
            self.assertFalse(s.renderer_application(spec));launch.assert_not_called()
        r=json.loads((self.root/'refused.json').read_text());self.assertTrue(r['cleanup_confirmed']);self.assertIsNone(r['effective'])
    def test_focus_filters_by_mapped_image_and_linux_generation(self):
        proc=self.root/'proc';proc.mkdir();(proc/'7').mkdir();(proc/'8').mkdir()
        dev,ino=self.image.stamp[:2]
        (proc/'7/maps').write_text(f'0-1 r-xp 0 {os.major(dev):x}:{os.minor(dev):x} {ino} private\n')
        (proc/'8/maps').write_text('0-1 r-xp 0 0:0 999 unrelated\n')
        class Scope:
            proc_root=proc
            def members(self):return [{'pid':p,'start_ticks':10} for p in (7,8)]
            def identity(self,p):return {'pid':p,'start_ticks':10}
        def focus(scope,*_,**__):self.assertEqual(scope.members(),[{'pid':7,'start_ticks':10}]);return 'confirmed'
        with patch.object(s,'vendor_focus',side_effect=focus):self.assertEqual(s.renderer_focus(Scope(),self.image),'confirmed')

if __name__=='__main__':unittest.main()
