import unittest
from child_exit import select

class Selection(unittest.TestCase):
    def test_exact_tree_image_and_session(self):
        records={1:dict(pid=1,ppid=0),2:dict(pid=2,ppid=1),3:dict(pid=3,ppid=2),4:dict(pid=4,ppid=0)}
        owner=['/usr/bin/python3','registered-supervisor','private-owner-record']
        command=['exact-host','--session','nonce','--scanner-sha256','image']
        args={1:owner,3:command,4:command}
        self.assertEqual(select(records,owner,'nonce','image',args,{3:True,4:True})[1]['pid'],3)
        for sid,image,maps in [('other','image',{3:True}),('nonce','other',{3:True}),('nonce','image',{3:False,4:True})]:
            with self.subTest(sid=sid,image=image,maps=maps),self.assertRaises(RuntimeError):
                select(records,owner,sid,image,args,maps)
        # A name/argument match outside the exact process tree is never owned.
        records[3]['ppid']=0
        with self.assertRaises(RuntimeError):select(records,owner,'nonce','image',args,{3:True,4:True})
        records[3]['ppid']=2;records[4]['ppid']=2
        with self.assertRaises(RuntimeError):select(records,owner,'nonce','image',args,{3:True,4:True})
        args[2]=owner
        with self.assertRaises(RuntimeError):select(records,owner,'nonce','image',args,{3:True})

if __name__=='__main__':unittest.main()
