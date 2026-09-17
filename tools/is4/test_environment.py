import hashlib,unittest
from environment import validate

def rows():
    out=[]
    for m,v in [('baseline','prior=n'),('unavailable','prior=n;powershell.exe='),('restored','prior=n')]:
        for o in ('expected','child'):
            out.append(f'IS3_ENV_V1 mode={m} origin={o} present=1 length={len(v)} sha256={hashlib.sha256(v.encode("utf-16le")).hexdigest()} duplicate_count=0')
    return out
class EnvironmentTests(unittest.TestCase):
    def test_delivery_only(self):self.assertFalse(validate(rows())['loader_authority'])
    def test_duplicates_and_mismatch(self):
        for i in range(6):
            r=rows();r[i]=r[i].replace('duplicate_count=0','duplicate_count=1')
            with self.assertRaises(ValueError):validate(r)
        r=rows();r[1]=r[1].replace('length=7','length=8')
        with self.assertRaises(ValueError):validate(r)
    def test_order_bounds_and_missing(self):
        r=rows()
        for bad in (r[:-1],r[::-1],r+r[:1],['x'*300]+r[1:]):
            with self.assertRaises(ValueError):validate(bad)
if __name__=='__main__':unittest.main()
