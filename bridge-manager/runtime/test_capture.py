"""CA1 bounded reporter and production orchestration regressions."""
import hashlib,json,os,pathlib,subprocess,sys,tempfile,unittest
from unittest.mock import patch
import session

class CaptureTests(unittest.TestCase):
    def spec(self,p):
        p=p.resolve();sid='ab'*16;directory=p/sid;directory.mkdir(mode=0o700)
        image=p/'test.exe';image.write_bytes(b'not a PE')
        art={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
        return dict(session=sid,registration=dict(host=art,module=art,environment={'root':str(p),'runner':{'files':[]}}),
            crash_capture=dict(schema=1,id=sid,session=sid,directory=str(directory)),inspect=True,directory=str(p/'compatdata/pfx/drive_c/bridge/sessions'/sid),report=str(p/'result.json'))
    def line(self,pid,tid,channel,func,body):return f'10.012:{pid:04x}:{tid:04x}:trace:{channel}:{func} {body}\n'.encode()
    def exception(self,c,pid=32,tid=36):
        c.write('stderr',self.line(pid,tid,'seh','dispatch_exception','code=c0000005 (EXCEPTION_ACCESS_VIOLATION) flags=0 addr=0000000140001006'))
        c.write('stderr',self.line(pid,tid,'unwind','RtlVirtualUnwind2','type 1 base 140000000 rip 140001006 rva 1006 rsp 111fe70'))
    def test_tail_saturation_cannot_spend_module_or_terminal_reserve(self):
        with tempfile.TemporaryDirectory() as d:
            spec=self.spec(pathlib.Path(d));c=session.IncidentCapture(spec)
            c.write('stderr',self.line(32,36,'loaddll','build_module','Loaded L"Z:\\test.exe" at 0000000140000000: native'))
            for _ in range(2000):c.write('stderr',b'10.012:0020:0024:err:module:load '+b'x'*1000+b'\n')
            self.exception(c)
            self.assertGreater(c.context.dropped_bytes,0);self.assertEqual(len(c.modules),1)
            self.assertEqual(len(c.exceptions),1);self.assertEqual(c.exceptions[0]['frames'][0]['rva'],0x1006)
            self.assertEqual(c.terminal.dropped_bytes,0)
    def test_malformed_oversize_partial_and_registers_are_not_payload(self):
        with tempfile.TemporaryDirectory() as d:
            c=session.IncidentCapture(self.spec(pathlib.Path(d)))
            c.write('stderr',b'x'*(c.LINE*3));c.write('stderr',b'bad end\n');self.exception(c)
            c.write('stderr',self.line(32,36,'seh','dispatch_exception','rax=secret_argument'))
            self.assertEqual(c.counts['oversized_lines'],1);self.assertEqual(len(c.exceptions),1)
            self.assertNotIn('secret',str(c.terminal.value()))
            self.assertLessEqual(sum(map(len,c.pending.values())),2*c.LINE)
    def test_signed_exit_status_identity_handled_and_fatal_reserve(self):
        with tempfile.TemporaryDirectory() as d:
            c=session.IncidentCapture(self.spec(pathlib.Path(d)));self.exception(c)
            c.write('stderr',self.line(33,36,'process','NtTerminateProcess','handle (nil), exit_code 0, process_exiting 0.'))
            self.assertNotIn(32,c.exits)
            c.write('stderr',self.line(32,36,'process','NtTerminateProcess','handle 0x40, exit_code 0, process_exiting 0.'))
            self.assertNotIn(32,c.exits) # A real handle is not a self-exit record.
            c.write('stderr',self.line(32,36,'process','NtTerminateProcess','handle (nil), exit_code -1073741819, process_exiting 0.'))
            self.assertEqual(c.exits[32]['exit_code'],0xc0000005)
            for i in range(32):self.exception(c,40+i,80+i)
            self.assertEqual(len(c.exceptions),16);self.assertEqual(len(c.fatal_reserved),1)
            self.assertEqual(c.fatal_reserved[0]['windows_pid'],32)
            c.write('stderr',self.line(80,81,'process','NtTerminateProcess','handle (nil), exit_code 23, process_exiting 0.'))
            self.assertEqual(c.exits[80]['exit_code'],23)
    def test_first_terminal_unchanged_and_cross_session_refused(self):
        with tempfile.TemporaryDirectory() as d:
            spec=self.spec(pathlib.Path(d));c=session.IncidentCapture(spec)
            original={'session':spec['session'],'status':3,'state_revision':8}
            c.retain_terminal({'before_containment':{'terminal_instance':original}})
            original['status']=99
            c.retain_terminal({'before_containment':{'terminal_instance':original}})
            self.assertEqual(c.first_terminal['status'],3)
            c.first_terminal=None
            with self.assertRaises(ValueError):c.retain_terminal({'before_containment':{'terminal_instance':{'session':'ff'*16}}})
    def test_long_error_excerpt_keeps_readable_summary_bounded(self):
        with tempfile.TemporaryDirectory() as d:
            c=session.IncidentCapture(self.spec(pathlib.Path(d)))
            for _ in range(20):c.context.write(b'error '+b'x'*8180)
            c.finish({'raw_exit':0,'cleanup_confirmed':True,'transport_retired':True})
            self.assertLessEqual((c.path/'summary.txt').stat().st_size,65536)
            self.assertIn('Summary lines omitted by capacity:',(c.path/'summary.txt').read_text())
            self.assertEqual(json.loads((c.path/'status.json').read_text())['state'],'finalized')
    def test_missing_module_never_guesses_preferred_base_or_symbols(self):
        with tempfile.TemporaryDirectory() as d:
            c=session.IncidentCapture(self.spec(pathlib.Path(d)))
            self.assertIsNone(c.frame(32,0x140001006)['module'])
            self.assertIsNone(c.frame(32,0x140001006)['symbol'])
    def test_disarm_does_not_stop_draining_and_private_report_excludes_raw_share(self):
        with tempfile.TemporaryDirectory() as d:
            spec=self.spec(pathlib.Path(d));c=session.IncidentCapture(spec)
            (c.path/'cancel.json').write_text('true');c.check();self.assertFalse(c.active)
            c.write('stderr',b'private payload');self.assertEqual(c.counts['disabled_bytes'],15)
            c.finish({'raw_exit':-9,'cleanup_confirmed':True,'transport_retired':True})
            share=json.loads((c.path/'share.json').read_text());self.assertEqual(share['exceptions'],[])
            self.assertNotIn(str(d),json.dumps(share));self.assertNotIn(spec['session'],json.dumps(share))
            self.assertEqual(os.stat(c.path/'summary.txt').st_mode&0o777,0o600)
            self.assertIsNone(json.loads((c.path/'incident.json').read_text())['processes']['windows_host']['unix_wait_status'])

@unittest.skipUnless(sys.platform.startswith('linux'),'production procfs cleanup')
class ProductionCaptureTests(unittest.TestCase):
    spec=CaptureTests.spec
    def run_child(self,fail_finalize=False,capture_on=True):
        with tempfile.TemporaryDirectory() as d:
            p=pathlib.Path(d);spec=self.spec(p);pathlib.Path(spec['directory']).mkdir(mode=0o700,parents=True)
            if not capture_on:spec.pop('crash_capture')
            program="import sys;sys.stderr.write('noise\\n'*20000);print('{\"event\":\"lifecycle\",\"state\":\"scanner_completed\"}',flush=True)"
            sibling=subprocess.Popen(['/bin/sleep','30'],start_new_session=True)
            try:
                with patch.object(session,'command',return_value=([sys.executable,'-c',program],b'')),patch.object(session,'environment',return_value=os.environ.copy()):
                    if fail_finalize:
                        with patch.object(session.IncidentCapture,'finish',side_effect=OSError('deliberate disk failure')):result=session.run(spec)
                    else:result=session.run(spec)
                self.assertTrue(result['cleanup_confirmed']);self.assertTrue(result['transport_retired']);self.assertIsNone(sibling.poll())
                if capture_on:
                    self.assertEqual(bool(result['incident']['reporting_error']),fail_finalize)
                    state=json.loads((p/spec['session']/'status.json').read_text());self.assertFalse(state['capture_enabled'])
                else:self.assertNotIn('incident',result)
            finally:sibling.terminate();sibling.wait(timeout=5)
    def test_production_capture_and_finalization_failure_do_not_block_cleanup(self):self.run_child();self.run_child(True)
    def test_off_path_remains_off(self):self.run_child(capture_on=False)

if __name__=='__main__':unittest.main()
