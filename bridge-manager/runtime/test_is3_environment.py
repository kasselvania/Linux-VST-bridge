import hashlib,pathlib,tempfile,unittest
import session
class TargetEnvironmentTests(unittest.TestCase):
    def test_exact_closed_and_unchanged_normal(self):
        with tempfile.TemporaryDirectory() as t:
            p=pathlib.Path(t)/'fixture.exe';p.write_bytes(b'owned')
            a={'path':str(p),'sha256':hashlib.sha256(b'owned').hexdigest()}
            spec={'operation':'a'*32,'installer':a,'format':'pe_executable'}
            base={'WINEDEBUG':'-all','WINEDLLOVERRIDES':'prior=n'}
            self.assertIs(session.is3_target_environment(spec,None,base)[0],base)
            for mode in ('baseline','unix_override','restored'):
                f={'schema':1,'mode':mode,'operation':spec['operation'],'artifact':a}
                env,r=session.is3_target_environment(spec,f,base)
                self.assertEqual(env['WINEDLLOVERRIDES'],'powershell.exe=' if mode=='unix_override' else 'prior=n')
                self.assertEqual(base['WINEDLLOVERRIDES'],'prior=n')
                self.assertEqual(r['phase'],'target_runner_after_prefix_initialization')
            for key,value in [('mode','arbitrary'),('operation','b'*32),('artifact',dict(a,sha256='0'*64))]:
                f={'schema':1,'mode':'unix_override','operation':spec['operation'],'artifact':a};f[key]=value
                with self.assertRaises(RuntimeError):session.is3_target_environment(spec,f,base)
            p.write_bytes(b'changed')
            with self.assertRaises(RuntimeError):session.is3_target_environment(spec,{'schema':1,'mode':'unix_override','operation':spec['operation'],'artifact':a},base)
    def test_after_bootstrap_before_target_and_no_cli_field(self):
        s=pathlib.Path(session.__file__).read_text()
        call=s.index('launch_env,is3_receipt=is3_target_environment')
        self.assertLess(s.index('prefix initialization missing'),call)
        self.assertLess(call,s.index("child=launch(argv,'target_runner')"))
        self.assertNotIn("spec.get('source_owned_is3')",s)

class FixtureCaptureTests(unittest.TestCase):
    def test_split_oracle_survives_noise_and_is_bounded(self):
        c=session.IS3FixtureCapture();c.feed(b'IS3_ENV_', 'stdout');c.feed(b'V1 owned\n','stdout')
        for _ in range(10000):c.feed(b'noise\n','stderr')
        self.assertEqual(c.value()['oracle_rows'],['IS3_ENV_V1 owned'])
        c.feed(b'x'*5000,'stdout');c.feed(b'IS3_CAP_V1 fake\n','stdout')
        self.assertEqual(len(c.rows),1)
        for _ in range(30):c.feed(b'IS3_CAP_V1 bounded\n','stdout')
        self.assertEqual(len(c.rows),18);self.assertGreater(c.dropped,0)
