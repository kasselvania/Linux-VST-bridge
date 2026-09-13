"""CA1 bounded reporter and production orchestration regressions."""
import hashlib,json,mmap,os,pathlib,struct,subprocess,sys,tempfile,unittest
from types import SimpleNamespace
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
    def test_unload_is_path_specific_and_preserves_prior_exception_binding(self):
        with tempfile.TemporaryDirectory() as d:
            c=session.IncidentCapture(self.spec(pathlib.Path(d)))
            c.write('stderr',self.line(32,36,'loaddll','build_module','Loaded L"Z:\\test.exe" at 140000000: native'))
            c.modules[0].update(size_of_image=65536,sha256='ab'*32)
            c.write('stderr',self.line(32,36,'loaddll','free_modref','Unloaded module L"Z:\\other.dll" : builtin'))
            self.assertEqual(c.frame(32,0x140001006,at='10.020')['offset'],0x1006)
            line=self.line(32,36,'loaddll','free_modref','Unloaded module L"Z:\\test.exe" : native').replace(b'10.012',b'10.030')
            c.write('stderr',line)
            self.assertEqual(c.frame(32,0x140001006,at='10.020')['offset'],0x1006)
            self.assertIsNone(c.frame(32,0x140001006,at='10.030')['module'])
            self.assertIsNone(c.frame(32,0x140001006)['module'])
            # A later same-path load at another actual base cannot bind old IPs.
            c.write('stderr',self.line(32,36,'loaddll','build_module','Loaded L"Z:\\test.exe" at 150000000: native').replace(b'10.012',b'10.040'))
            c.modules[1].update(size_of_image=65536,sha256='ab'*32)
            self.assertIsNone(c.frame(32,0x150001006,at='10.020')['module'])
            self.assertEqual(c.frame(32,0x150001006,at='10.050')['offset'],0x1006)
            c.write('stderr',self.line(32,36,'loaddll','unload_unknown','unknown unload form').replace(b'10.012',b'10.060'))
            self.assertEqual(c.frame(32,0x150001006,at='10.050')['offset'],0x1006)
            self.assertIsNone(c.frame(32,0x150001006,at='10.070')['module'])
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
    line=CaptureTests.line
    @staticmethod
    def pe(name):
        # Source-owned minimal PE with one exported code address. No binary
        # fixture, proprietary module or external compiler is needed here.
        b=bytearray(1536);b[:2]=b'MZ';struct.pack_into('<I',b,60,128)
        b[128:132]=b'PE\0\0';struct.pack_into('<H',b,134,1);struct.pack_into('<H',b,148,240)
        struct.pack_into('<H',b,152,0x20b);struct.pack_into('<I',b,208,8192)
        struct.pack_into('<II',b,264,0x1100,80)
        struct.pack_into('<IIII',b,400,1024,0x1000,1024,512)
        struct.pack_into('<IIIII',b,788,1,1,0x1150,0x1160,0x1170)
        struct.pack_into('<I',b,848,0x1000);struct.pack_into('<I',b,864,0x1180)
        b[896:896+len(name)+1]=name.encode()+b'\0'
        return b
    def mapped_capture(self,p):
        c=session.IncidentCapture(self.spec(p));image=p/'mapped.dll';image.write_bytes(self.pe('original_export'))
        f=image.open('rb');mapping=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ);f.close()
        self.addCleanup(mapping.close)
        ident=c.identity(os.getpid());key=(ident['pid'],ident['start_ticks'])
        c.observe({key},SimpleNamespace(pid=os.getpid()))
        row=next(x for x in c.processes[key]['mapped_files'] if x['path']==str(image))
        meta=image.stat()
        self.assertEqual(row['owner'],dict(pid=os.getpid(),start_ticks=ident['start_ticks']))
        self.assertEqual((row['device_major'],row['device_minor'],row['inode']),
                         (os.major(meta.st_dev),os.minor(meta.st_dev),meta.st_ino))
        self.assertEqual(row['resolved_path'],str(image));self.assertFalse(row['deleted']);self.assertIsNone(row['unavailable'])
        c.write('stderr',self.line(32,36,'loaddll','build_module',f'Loaded L"Z:{str(image).replace(chr(47),chr(92))}" at 140000000: native'))
        return c,image
    def test_mapped_file_hash_and_exports_use_one_verified_open_file(self):
        with tempfile.TemporaryDirectory() as d:
            c,image=self.mapped_capture(pathlib.Path(d));digest=hashlib.sha256(image.read_bytes()).hexdigest()
            opened=[];real_hash=hashlib.file_digest;real_pe=session.capture_pe
            def hashed(f,algorithm):opened.append(f);return real_hash(f,algorithm)
            def parsed(f):self.assertIs(f,opened[-1]);return real_pe(f)
            with patch.object(session.hashlib,'file_digest',side_effect=hashed),patch.object(session,'capture_pe',side_effect=parsed):
                c.finish({'raw_exit':0,'cleanup_confirmed':True,'transport_retired':True})
            self.assertEqual(c.modules[0]['sha256'],digest)
            self.assertEqual(c.frame(32,0x140001006)['symbol']['name'],'original_export')
    def test_replaced_mapped_path_cannot_supply_replacement_hash_or_symbols(self):
        with tempfile.TemporaryDirectory() as d:
            c,image=self.mapped_capture(pathlib.Path(d))
            replacement=image.with_suffix('.new');replacement.write_bytes(self.pe('replacement_export'));replacement.replace(image)
            with patch.object(session,'capture_pe',side_effect=AssertionError('replacement must not be parsed')):
                result=c.finish({'raw_exit':5,'cleanup_confirmed':True,'transport_retired':True})
            self.assertIsNone(result['modules'][0]['sha256']);self.assertIsNone(c.frame(32,0x140001006)['symbol'])
            self.assertEqual(result['modules'][0]['linux_path'],str(image))
            self.assertEqual(result['modules'][0]['identity_unavailable'],'mapped file replaced/unverifiable')
    def test_deleted_mapping_retains_identity_but_not_replacement_authority(self):
        with tempfile.TemporaryDirectory() as d:
            c,image=self.mapped_capture(pathlib.Path(d));old=image.stat().st_ino;image.unlink()
            c.dirty=True;ident=c.identity(os.getpid());c.observe({(ident['pid'],ident['start_ticks'])},SimpleNamespace(pid=os.getpid()))
            rows=c.processes[(ident['pid'],ident['start_ticks'])]['mapped_files']
            self.assertTrue(any(x['path']==str(image) and x['deleted'] and x['inode']==old for x in rows))
            image.write_bytes(self.pe('replacement_export'));c.resolve_modules()
            self.assertIsNone(c.modules[0]['sha256']);self.assertIsNone(c.frame(32,0x140001006)['symbol'])
            self.assertEqual(c.modules[0]['identity_unavailable'],'mapped file deleted/unverifiable')
    def test_symlink_replacement_is_not_followed(self):
        with tempfile.TemporaryDirectory() as d:
            c,image=self.mapped_capture(pathlib.Path(d));other=image.with_suffix('.other');other.write_bytes(self.pe('replacement_export'))
            image.unlink();image.symlink_to(other)
            c.resolve_modules();self.assertIsNone(c.modules[0]['sha256']);self.assertIsNone(c.frame(32,0x140001006)['symbol'])
            self.assertEqual(c.modules[0]['identity_unavailable'],'mapped file deleted/unverifiable')
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
