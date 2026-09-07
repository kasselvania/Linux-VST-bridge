"""Bounded SDK-derived descriptors preserve identity and reject false layouts."""
import unittest
from ap8_descriptor import generate
class Descriptor(unittest.TestCase):
 def records(self,effect=True):
  bus=lambda d,i,t=0,m=0,c=2:dict(state='ap8_bus',media=m,direction=d,index=i,channels=c,type=t,flags=1,arrangement=3 if m==0 else 0,name='Test')
  return ([bus(0,0),bus(0,1,1)] if effect else [])+[bus(1,0),bus(0,0,m=1,c=0 if effect else 16),dict(state='ap8_inspected',latency_samples=176,float32_result=0),dict(state='ap8_parameters',parameters=[[0,'Mix','%',0,1,.5,.5]]),dict(state='ap8_parameter_count',count=1)]
 def gen(self,r):return generate(r,'00'*16,'ab'*32)
 def test_effect_sidechain_and_stable_identity(self):
  a=self.gen(self.records());self.assertIn('effect=true',a);self.assertIn('{0,0,1,2,1,1,3,',a)
  self.assertEqual([s for s in a.splitlines() if '_UID 'in s],[s for s in self.gen(self.records(False)).splitlines() if '_UID 'in s])
 def test_arrangements_indices_and_precision_are_not_guessed(self):
  for k,v in [('arrangement',0),('channels',1),('index',2)]:
   r=self.records();r[0][k]=v
   with self.assertRaises(ValueError):self.gen(r)
  r=self.records();r[-3]['float32_result']=1
  with self.assertRaises(ValueError):self.gen(r)
if __name__=='__main__':unittest.main()
