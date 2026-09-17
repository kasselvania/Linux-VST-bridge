import test_support
import pathlib,tempfile,unittest
from campaign import campaign,MODES
from identity import atomic_new,digest,envelope
from test_fixtures import fake_package
from test_comparison import proofs
class CampaignTests(unittest.TestCase):
    def test_committed_owner_orders_consumes_and_publishes_atomically(self):
        with tempfile.TemporaryDirectory() as t:
            root=pathlib.Path(t);m,seal=fake_package(root/'package');calls=[];prepared=proofs()
            def run(package,out,mode,s,source):
                calls.append(mode);out.mkdir();p=prepared[MODES.index(mode)]
                p['identity']=envelope(m,seal);p['payload_sha256']=m['payload']['sha256'];p['binding'].update(artifact_sha256=p['payload_sha256'],size=m['payload']['size']);p['unix_environment']['fixture_sha256']=p['payload_sha256']
                p['staged_runtime_sha256']=m['files']['session.py'];p['source_sha256']={k:m['files'][k] for k in ('run.py','report.py','capability.cpp')}
                p['policy_owner_digests']={k:m['files'][f] for k,f in [('manager','policy-owner'),('supervisor','session.py'),('ownership','ownership.py')]}
                capture=out/(p['binding']['operation']+'-is3-observation.private.json');atomic_new(capture,{'loader_rows_private':[],'dropped_records':0});p['observation_private_sha256']=digest(capture)
                atomic_new(out/'proof-private.json',p)
            result=campaign(root/'package',root/'result',seal,m['source'],run)
            self.assertEqual(calls,list(MODES));self.assertEqual(result['disposition'],'IS4_OPERATION_SCOPED_HONEST_ABSENCE_PROVED')
            self.assertTrue((root/'result/loader-authority.json').exists())
    def test_bad_seal_and_first_failure_never_launch_next_session(self):
        with tempfile.TemporaryDirectory() as t:
            root=pathlib.Path(t);m,seal=fake_package(root/'package');calls=[]
            def fail(*args):calls.append(args[2]);raise ValueError('failure')
            with self.assertRaises(ValueError):campaign(root/'package',root/'bad','0'*64,m['source'],fail)
            self.assertFalse(calls)
            with self.assertRaises(ValueError):campaign(root/'package',root/'result',seal,m['source'],fail)
            self.assertEqual(calls,['baseline']);self.assertFalse((root/'result/loader-authority.json').exists())
