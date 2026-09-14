import copy
import sys
from pathlib import Path
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parent))
from direct import receipt_graph,post_action
from test_transient import fixture,E
class Tests(unittest.TestCase):
    def case(self):
        a,r,b,l=fixture();target=a['windows'][0]
        for i,x in enumerate(r['x11']):x['observation']=dict(button=1,state=0 if i==0 else 256,screen=[20,20],observed_ns=150000+i)
        for i,x in enumerate(r['win32']):x['observation']=dict(qpc=150+i)
        return a,r,b,target
    def test_last_complete_pre_input_graph_not_post_input_graph(self):
        a,r,b,t=self.case();self.assertEqual(receipt_graph([a,b],r,E,(20,20),t),a)
        with self.assertRaisesRegex(RuntimeError,'pre-input'):receipt_graph([b],r,E,(20,20),t)
    def test_wrong_point_modifiers_extra_input_and_wrong_popup_refused(self):
        a,r,b,t=self.case()
        for edit in ('point','modifier','held_button','extra','popup'):
            q=copy.deepcopy(r);target=copy.deepcopy(t)
            if edit=='point':q['x11'][0]['observation']['screen']=[100,100]
            if edit=='modifier':q['x11'][0]['observation']['state']=1
            if edit=='held_button':q['x11'][0]['observation']['state']=512
            if edit=='extra':q['x11'].append(q['x11'][0])
            if edit=='popup':target['hwnd']=999
            with self.subTest(edit=edit),self.assertRaises(RuntimeError):receipt_graph([a,b],q,E,(20,20),target)
    def test_resize_observation_does_not_authorize_a_new_popup(self):
        a,r,b,l=fixture();b['windows'][0]=copy.deepcopy(b['windows'][0])
        b['windows'][0]['rect']=[0,0,1000,700]
        self.assertIsNone(post_action(E,1,'resize_choice',a,r,b,l,None))
        with self.assertRaises(RuntimeError):post_action(E,1,'open_menu',a,r,b,l,None)
        b['windows'][0]['xid']=999
        with self.assertRaises(RuntimeError):post_action(E,1,'resize_choice',a,r,b,l,None)
    def test_adapter_has_no_input_injection_or_browser_dependency(self):
        import ast
        tree=ast.parse((Path(__file__).parent/'direct.py').read_text())
        calls=[n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)]
        for forbidden in ('button','key','move','act','execute','click_pair'):
            self.assertNotIn(forbidden,calls)
if __name__=='__main__':unittest.main()
