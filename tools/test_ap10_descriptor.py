"""Bounded SDK-derived descriptors preserve identity and reject false layouts."""
import unittest
from ap8_descriptor import generate
class Descriptor(unittest.TestCase):
 def records(self,effect=True):
  bus=lambda d,i,t=0,m=0,c=2:dict(state='ap8_bus',media=m,direction=d,index=i,channels=c,type=t,flags=1,arrangement=3 if m==0 else 0,name='Test')
  return ([bus(0,0),bus(0,1,1)] if effect else [])+[bus(1,0),bus(0,0,m=1,c=0 if effect else 16),*([bus(1,0,m=1,c=16)] if not effect else []),dict(state='ap8_inspected',latency_samples=176,float32_result=0),dict(state='ap8_parameters',parameters=[[0,'Mix','%',0,1,.5,.5]]),dict(state='ap8_parameter_count',count=1)]
 def gen(self,r):return generate(r+[dict(state='ap12_class',class_id='00'*16,name='Vendor Test',vendor='Test vendor',version='1.0',subcategories='Fx' if any(x.get('media')==0 and x.get('direction')==0 for x in r) else 'Instrument|Synth',metadata_tier='factory_2')],'00'*16,'ab'*32)
 def test_effect_sidechain_and_stable_identity(self):
  a=self.gen(self.records());self.assertIn('effect=true',a);self.assertIn('{0,0,1,2,1,1,3,',a)
  self.assertIn('{1,1,0,16,0,1,0,', self.gen(self.records(False)))
  self.assertEqual([s for s in a.splitlines() if '_UID 'in s],[s for s in self.gen(self.records(False)).splitlines() if '_UID 'in s])
 def test_arrangements_indices_and_precision_are_not_guessed(self):
  for k,v in [('arrangement',0),('channels',1),('index',2)]:
   r=self.records();r[0][k]=v
   with self.assertRaises(ValueError):self.gen(r)
  r=self.records();r[-3]['float32_result']=1
  with self.assertRaises(ValueError):self.gen(r)
 def test_vendor_name_is_metadata_and_does_not_rebind_projects(self):
  def named(name,cid='00'*16):return generate(self.records()+[dict(state='ap12_class',class_id=cid,name=name,vendor='Test vendor',version='1.0',subcategories='Fx',metadata_tier='factory_2')],'00'*16,'ab'*32)
  a=named('Vendor Original');b=named('Vendor Renamed')
  self.assertNotEqual(a,b)
  self.assertEqual([s for s in a.splitlines() if '_UID 'in s],[s for s in b.splitlines() if '_UID 'in s])
  for name,cid in [('', '00'*16),('Name', '11'*16),('x'*64,'00'*16),('x\x00y','00'*16)]:
   with self.assertRaises(ValueError):named(name,cid)
 def test_instrument_with_input_is_still_instrument(self):
  r=self.records()+[dict(state='ap12_class',class_id='00'*16,name='Input instrument',vendor='Test',version='1.0',subcategories='Instrument|Sampler',metadata_tier='factory_3_unicode')]
  a=generate(r,'00'*16,'ab'*32);self.assertIn('effect=false',a)
  for categories,tier in [('', 'factory_1'),('Fx|Instrument','factory_2'),('Analyzer','factory_2')]:
   r[-1].update(subcategories=categories,metadata_tier=tier)
   with self.assertRaises(ValueError):generate(r,'00'*16,'ab'*32)
 def test_discovery_remains_available_when_vendor_refuses_state(self):
  r=self.records()
  r[-3]['state']='ap12_capabilities'
  r.append(dict(state='ap8_failure',reason='getComponentState returned 1'))
  self.assertIn('effect=true',self.gen(r))
  self.assertEqual(r[-1]['state'],'ap8_failure')
if __name__=='__main__':unittest.main()
