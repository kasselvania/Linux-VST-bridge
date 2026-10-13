"""Arturia's ordinary completion must not call a renderer-only local helper."""
import hashlib,json,os,pathlib,signal,subprocess,sys,tempfile,unittest
from unittest.mock import Mock,patch
import session as s
class VendorCompletion(unittest.TestCase):
 def test_clean_companion_exit_reports_completed(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp).resolve();image=root/'fixture';image.write_bytes(b'generated')
   artifact={'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()}
   app={'executable':artifact,'helpers':[],'environment':{'root':str(root),'runner':{'files':[]}}}
   scope=Mock();scope.members.return_value=[];scope.group='generated-owned-group'
   library=Mock();library.prctl.return_value=0
   signals=[signal.getsignal(x) for x in (signal.SIGTERM,signal.SIGINT)]
   try:
    with patch.object(s,'CompanionCgroup',return_value=scope),patch.object(s.ctypes,'CDLL',return_value=library),patch.object(s.os,'waitid',return_value=None),patch.object(s,'vendor_launch',return_value=([sys.executable,'-c','pass'],root)),patch.object(s,'environment',return_value=os.environ.copy()):
     self.assertTrue(s.vendor_application({'application':app,'report':str(root/'result.json')}))
    r=json.loads((root/'result.json').read_bytes());self.assertEqual(r['state'],'completed');self.assertIsNone(r['error']);self.assertEqual(r['launcher_exit'],0);self.assertTrue(r['cleanup_confirmed'])
   finally:
    for sig,old in zip((signal.SIGTERM,signal.SIGINT),signals):signal.signal(sig,old)

class SelectedApplicationOwnership(unittest.TestCase):
 def test_selected_application_uses_exact_cohort_and_keeps_late_descendant_owned(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);proc=root/'proc';groups=root/'groups';selector='ab'*32
   group='/user.slice/user-1000.slice/user@1000.service/app.slice/linux-vst-bridge-vendor-'+selector+'.service'
   (proc/'100').mkdir(parents=True);(proc/'100'/'cgroup').write_text('0::'+group+'\n')
   owner=groups/group.lstrip('/');owner.mkdir(parents=True);(owner/'cgroup.procs').write_text('100\n')
   nested=owner/'download';nested.mkdir();(nested/'cgroup.procs').write_text('101\n')
   (proc/'101').mkdir();(proc/'101'/'cgroup').write_text('0::'+group+'/download\n')
   fields=['S','100']+['0']*48;fields[19]='12345'
   (proc/'101'/'stat').write_text('101 (unknown helper) '+' '.join(fields))
   cohort=s.CompanionCgroup(proc_root=proc,cgroup_root=groups,supervisor=100,vendor_application=selector)
   members=cohort.members();self.assertEqual([r['pid'] for r in members],[101])
   self.assertEqual(s.vendor_operation_state(0,len(members)),'unknown')
   (nested/'cgroup.procs').write_text('');self.assertEqual(cohort.members(),[])
   self.assertEqual(s.vendor_operation_state(0,0),'completed')
   with self.assertRaises(Exception):s.CompanionCgroup(proc_root=proc,cgroup_root=groups,supervisor=100,vendor_application='cd'*32)
   with self.assertRaises(Exception):s.CompanionCgroup(proc_root=proc,cgroup_root=groups,supervisor=100,vendor_application=selector,installer_operation='ef'*16)
 def test_selected_helpers_have_neutral_roles_and_default_compatibility(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);process=root/'42';process.mkdir();(process/'maps').write_text('')
   artifacts=[]
   for i in range(4):
    image=root/f'image{i}.exe';image.write_bytes(b'MZ first-party image');artifacts.append({'path':str(image),'sha256':hashlib.sha256(image.read_bytes()).hexdigest()})
   (process/'exe').symlink_to(artifacts[1]['path']);scope=Mock();scope.proc_root=root;scope.identity.return_value=None
   for count in (1,3):
    app={'schema':2,'id':'selected','executable':artifacts[0],'helpers':artifacts[1:1+count],'environment':{'runner':{'files':[]}}}
    row=s.vendor_process_metadata(scope,{'pid':42,'ppid':1},app)
    self.assertEqual(row['registered_images'][0]['role'],'selected_helper')
    self.assertEqual(s.vendor_compatibility('normal',app),{'disable_windows_accessibility':False})
    with self.assertRaises(Exception):s.vendor_compatibility('agent_probe',app)
 def test_late_submission_never_launches_after_exact_reconciliation(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=pathlib.Path(tmp);op='cd'*16;report=root/'result.json'
   report.write_text(json.dumps({'schema':3,'operation_id':op,'state':'failed','owned_live':0,'cleanup_confirmed':True}))
   app={'executable':{},'helpers':[],'environment':{'root':str(root)}}
   with patch.object(s,'vendor_launch') as launch:
    self.assertEqual(s.vendor_application({'application':app,'report':str(report),'operation_id':op}),1)
    launch.assert_not_called()
   self.assertEqual(json.loads(report.read_text())['state'],'failed')
