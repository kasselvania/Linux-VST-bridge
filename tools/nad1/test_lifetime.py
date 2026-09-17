import copy,json,pathlib,sys,tempfile,unittest
from unittest.mock import patch
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[2]/'bridge-manager/runtime'))
import application_campaign as c
import ownership
class Lifetime(unittest.TestCase):
 def test_exact_two_live_generations_and_mutations(self):
  with tempfile.TemporaryDirectory() as tmp:
   report=pathlib.Path(tmp)/'result.json';op='a'*32;group='/unit/linux-vst-bridge-renderer-'+op+'.service'
   result={'state':'running','dependency':{'ready_tested':True},'effective':{},'renderer':{'launch_binding':{'status':'bound'}}}
   rows=[{'pid':i,'start_ticks':20+i,'cgroup':group,'linux_exit':None,'phase':phase,'relationship':'direct_launcher'} for i,phase in enumerate(('dependency_start','application_runner'),1)]
   ledger={'dropped_process_observations':0,'processes':rows}
   def bounded(path,kind):
    pid=int(path.parent.name)
    if path.name=='cgroup':return ('0::'+group+'\n').encode()
    return (str(pid)+' (fixture) S '+'0 '*18+str(20+pid)+'\n').encode()
   def write(r,l):report.write_text(json.dumps(r));(report.parent/(op+'-ledger.private.json')).write_text(json.dumps({'ledger':l}))
   with patch.object(ownership,'bounded',side_effect=bounded):
    write(result,ledger);self.assertEqual(set(c.lifetime_sample(report,op)['generations']),{'dependency_start','application_runner'})
    for field in ('drop','duplicate','reused','exited','foreign','not_ready','unbound','terminal'):
     r=copy.deepcopy(result);l=copy.deepcopy(ledger)
     if field=='drop':l['dropped_process_observations']=1
     elif field=='duplicate':l['processes'].append(rows[0])
     elif field=='reused':l['processes'][0]['start_ticks']+=1
     elif field=='exited':l['processes'][0]['linux_exit']=0
     elif field=='foreign':l['processes'][0]['cgroup']='/other'
     elif field=='not_ready':r['dependency']['ready_tested']=False
     elif field=='unbound':r['renderer']['launch_binding']['status']='unavailable'
     else:r['state']='completed'
     write(r,l)
     with self.subTest(field=field),self.assertRaises(ValueError):c.lifetime_sample(report,op)
