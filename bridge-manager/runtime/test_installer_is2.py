import hashlib,json,pathlib,tempfile,unittest
import session

class IS2Tests(unittest.TestCase):
    op='a'*32;token='b'*64;digest='c'*64
    def trace(self):
        t=session.InstallerWindowsTrace({'path':'/owned/installer.exe','sha256':self.digest},pathlib.Path('/owned/environment'))
        t.arm_root(self.op,self.token,100);t.begin();t.begin();return t
    def frame(self,pid=32,created=100):return f'IS2_ROOT_V1 {self.op} {self.token} 2 {self.digest} 100 {pid} {created} 16 99\n'.encode()
    def event(self,t,pid,body,fn='CreateProcessInternalW'):return t.feed(f'10.000:{pid:04x}:0004:trace:process:{fn} {body}\n'.encode())
    def create(self,t,parent,pid,body='app (null) cmdline L"unknown", inherit 0'):
        self.event(t,parent,body);self.event(t,parent,f'started process pid {pid:04x} tid 0004');return t.rows[-1]
    def exit(self,t,pid,status=0):self.event(t,pid,f'handle 0xffffffffffffffff, exit_code {status}, process_exiting 1.','NtTerminateProcess')
    def test_root_token_epoch_artifact_and_one_root(self):
        t=self.trace();self.assertTrue(t.root_frame(self.frame()));self.assertEqual(t.binding['status'],'bound');self.assertEqual(t.rows[0]['windows_creation_time'],100)
        row=self.create(t,32,48);self.assertTrue(row['target_tree']);self.exit(t,48,37);self.assertEqual(t.first_failure['status'],37)
        t.root_frame(self.frame());self.assertEqual(t.binding['status'],'unavailable')
        self.assertFalse(self.create(t,32,64)['target_tree'])
    def test_alias_unknown_trace_path_uses_exact_adapter_handle_proof(self):
        t=self.trace();row=self.create(t,16,32)
        self.assertIsNone(row['image_identity']);t.root_frame(self.frame())
        self.assertEqual(t.binding['status'],'bound');self.assertEqual(len(t.rows),1)
        t=self.trace();row=self.create(t,16,32);row['image_identity']={'sha256':'f'*64}
        t.root_frame(self.frame());self.assertEqual(t.binding['status'],'unavailable')
    def test_wrong_creator_and_creation_refusal_do_not_bind(self):
        t=self.trace();self.create(t,17,32);t.root_frame(self.frame());self.assertEqual(t.binding['status'],'unavailable')
        t=self.trace();t.root_frame(f'IS2_REFUSED_V1 {self.op} {self.token} 2 740\n'.encode())
        self.assertEqual(t.binding['status_domain'],'win32_create_process_error');self.assertEqual(t.binding['status_code'],740)
        self.assertEqual(t.rows,[])
        t.root_frame(self.frame());self.assertEqual(t.binding['status'],'unavailable')
    def test_unbound_wrong_token_artifact_truncated_epoch(self):
        for frame in [self.frame().replace(self.token.encode(),b'd'*64),self.frame().replace(self.digest.encode(),b'e'*64),self.frame()[:-1],b'IS2_ROOT_V1 bad\n']:
            t=self.trace();t.root_frame(frame);self.assertEqual(t.binding['status'],'unavailable');self.assertFalse(self.create(t,32,48)['target_tree'])
        t=self.trace();t.begin();t.root_frame(self.frame());self.assertNotEqual(t.binding['status'],'bound')
    def test_same_basename_or_digest_not_launch_authority(self):
        t=self.trace();self.create(t,16,32,'app L"Z:\\\\owned\\\\installer.exe" cmdline L"x", inherit 0')
        self.assertFalse(t.rows[-1]['target_root']);self.assertFalse(t.rows[-1]['target_tree'])
        self.exit(t,32);t.root_frame(self.frame());self.assertNotEqual(t.binding['status'],'bound')
    def test_retired_root_pid_reuse_does_not_inherit(self):
        t=self.trace();t.root_frame(self.frame());self.exit(t,32);self.assertFalse(self.create(t,32,48)['target_tree']);self.exit(t,48,41);self.assertIsNone(t.first_failure)
    def test_later_epoch_does_not_rebind_same_digest(self):
        t=self.trace();t.root_frame(self.frame());history=json.dumps(t.rows);t.begin();self.assertFalse(self.create(t,32,48)['target_tree']);self.assertEqual(json.loads(history)[0],t.rows[0])
    def body(self,code):
        cmd='"C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" -C "'+code+'"'
        return 'app (null) cmdline L'+json.dumps(cmd)+', inherit 1, flags 0x10'
    def scripts(self):
        q="Get-CimInstance -ClassName Win32_Process | ? {$_.Path -and $_.Path.StartsWith('C:\\Source Owned', 'CurrentCultureIgnoreCase')}"
        return ['if (('+q+').Count -gt 0) { exit 0 } else { exit 1 }',q+' | % { Stop-Process -Id $_.ProcessId -Force }']
    def test_script_query_close_recheck_zero_is_not_success(self):
        t=self.trace();t.root_frame(self.frame());query,close=self.scripts()
        for n,code in enumerate([query,close,query]):
            self.create(t,32,48+n,self.body(code));self.exit(t,48+n)
        rows=t.presence.rows;self.assertEqual([r['operation_class'] for r in rows],['presence_query','close_request','presence_query'])
        self.assertEqual(rows[2]['preceding_close'],2)
        for r in rows:self.assertIsNone(r['matched_object_count']);self.assertEqual(r['recheck_result'],'unavailable');self.assertEqual(r['helper_exit']['status'],0)
        self.assertEqual(rows[1]['close_result'],'unavailable');self.assertNotIn('Source Owned',json.dumps(rows))
    def test_script_bound_generation_not_name_or_temporal_guess(self):
        t=self.trace();query,close=self.scripts();self.create(t,32,48,self.body(close));self.exit(t,48)
        self.create(t,32,49,self.body(query));self.assertEqual(t.presence.rows[0]['authority'],'unbound_runner_trace');self.assertIsNone(t.presence.rows[1]['preceding_close'])
    def test_unknown_script_and_bound(self):
        q,c=self.scripts();request=session.InstallerPresence.request(self.body(q+'; arbitrary()'));self.assertEqual(request['operation_class'],'unknown')
        self.assertIsNone(session.InstallerPresence.request(self.body('x'*4097)))
        t=self.trace();t.root_frame(self.frame())
        for n in range(70):self.create(t,32,100+n,self.body(q))
        self.assertEqual(len(t.presence.rows),64);self.assertEqual(t.presence.dropped,6)
    def test_cancel_cleanup_cannot_erase_query_and_helper_failure(self):
        t=self.trace();t.root_frame(self.frame());q,c=self.scripts();self.create(t,32,48,self.body(c));self.exit(t,48,37);before=json.dumps(t.presence.value());failure=t.first_failure.copy();t.cancelled=True;self.exit(t,32,15)
        self.assertEqual(json.dumps(t.presence.value()),before);self.assertEqual(t.first_failure,failure)
    def test_stdout_cannot_bind_root(self):
        with tempfile.TemporaryDirectory() as d:
            tx=session.InstallerTransaction(self.op,pathlib.Path(d),pathlib.Path(d)/'result.json',{'path':'/owned/installer.exe','sha256':self.digest})
            tx.windows_trace.arm_root(self.op,self.token,100);tx.begin_phase();tx.begin_phase();tx.feed(self.frame(),'stdout');self.assertEqual(tx.windows_trace.binding['status'],'unavailable');tx.feed(self.frame(),'stderr');self.assertEqual(tx.windows_trace.binding['status'],'bound')

if __name__=='__main__':unittest.main()
