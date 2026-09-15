import copy
import pathlib
import struct
import sys
import unittest
HERE=pathlib.Path(__file__).parent
sys.path[:0]=[str(HERE),str(HERE.parent/'uio1'),str(HERE.parent/'uio2')]
from xi2 import packets,device_packet,RawScope,verify_device_extension
from timeline import summarize,projected
from session import Labels

class Tests(unittest.TestCase):
    def packet(self,kind=20,source=11,target=90):
        b=bytearray(80);b[0]=35;b[1]=131
        struct.pack_into('<IHH5I4iHHHHI',b,4,12,kind,2,300,1,1,target,0,10<<16,20<<16,3<<16,4<<16,0,0,source,0,0)
        return b
    def test_xi_explicit_opcode_kind_length_source_and_target(self):
        b=self.packet();r=device_packet(b,131,{90},{2,11})
        self.assertEqual((r['kind'],r['source'],r['target']),('xi_touch_end',11,90))
        for args in [(130,{90},{2,11}),(131,{91},{2,11}),(131,{90},{2})]:self.assertIsNone(device_packet(b,*args))
        for kind in (2,3,99):self.assertIsNone(device_packet(self.packet(kind),131,{90},{2,11}))
        with self.assertRaises(ValueError):device_packet(b[:-1],131,{90},{2,11})
        b[48]=9
        with self.assertRaises(ValueError):device_packet(b,131,{90},{2,11})
    def test_variable_generic_event_then_core_not_guessed_as_fixed32(self):
        core=bytes([5])+bytes(31);self.assertEqual([len(b) for b in packets(self.packet()+core)],[80,32])
        for b in (core[:4],self.packet()[:-1],bytes([35,131,0,0])+struct.pack('<I',99999)+bytes(24)):
            with self.assertRaises(ValueError):list(packets(b))
    def test_big_endian_xi(self):
        b=bytearray(80);b[:2]=bytes([35,131]);struct.pack_into('>IHH5I4iHHHHI',b,4,12,18,2,300,1,1,90,0,10<<16,20<<16,3<<16,4<<16,0,0,11,0,0)
        self.assertEqual(device_packet(b,131,{90},{2,11},True)['kind'],'xi_touch_begin')
    def test_raw_scope_is_queried_not_delivered_and_foreign_excluded(self):
        s=RawScope({90},{11})
        self.assertFalse(s.admit('raw_touch_begin',12,1,90));self.assertFalse(s.admit('raw_touch_begin',11,1,91))
        self.assertFalse(s.admit('raw_touch_end',11,1,90))
        self.assertTrue(s.admit('raw_touch_begin',11,1,90))
        self.assertTrue(s.admit('raw_touch_end',11,1,91));self.assertFalse(s.contacts)
        self.assertFalse(s.admit('raw_motion',11,0,91))
        for i in range(16):self.assertTrue(s.admit('raw_touch_begin',11,i,90))
        self.assertFalse(s.admit('raw_touch_begin',11,17,90))
    def test_lazily_added_pointer_requires_fresh_exact_census(self):
        old={2:dict(use=1,attachment=3,name_sha256='a')}
        new={**old,6:dict(use=3,attachment=2,name_sha256='b')}
        verify_device_extension(old,new)
        for bad in ({}, {2:dict(use=1,attachment=3,name_sha256='reused')}, {**old,6:dict(use=4,attachment=2,name_sha256='keyboard')}):
            with self.assertRaises(RuntimeError):verify_device_extension(old,bad)
        self.assertFalse(RawScope({90},new).admit('raw_button_up',6,1,91))
    def raw(self):
        def w(source,message,qpc,capture=0,result=0):return dict(action=1,source=source,message=message,qpc=qpc,capture=capture,result=result,hwnd=123456)
        r=dict(schema=1,profile_fingerprint='a'*64,actions=[dict(action=1)],x11=[dict(action=1,kind='core_up',observed_ns=100)],raw=[dict(action=1,kind='raw_touch_end',observed_ns=99)],win32=[w(5,0x202,100),w(6,0x202,101),w(14,0x202,102),w(15,0x202,104),w(4,0,110,result=3)],gui=[dict(action=1,kind=k,parameter=1,value=.2,poll_before_ns=t,observed_ns=t+1) for k,t in [(101,20),(102,30),(103,108)]],brackets=[dict(windows_qpc=0,frequency=1000000000,linux_before_ns=0,linux_after_ns=1)],frequency=1000000000,frames=[],drops={},completed=True,cleanup={})
        return r
    def boundary(self,r):return summarize(r)['actions'][0]['boundary']
    def test_complete_release_capture_gesture(self):
        self.assertEqual(self.boundary(self.raw()),'release_and_gesture_end_observed')
    def test_touch_end_without_core_or_windows(self):
        r=self.raw();r['x11']=[];r['win32']=[]
        self.assertEqual(self.boundary(r),'xi_touch_end_without_core_or_win32_release')
    def test_core_release_without_windows(self):
        r=self.raw();r['win32']=[]
        self.assertEqual(self.boundary(r),'core_release_without_win32_dispatch')
    def test_hook_swallow_and_getmessage_rewrite_are_distinct(self):
        r=self.raw();r['win32'][1]['result']=1
        self.assertEqual(self.boundary(r),'downstream_mouse_hook_nonzero')
        r=self.raw();r['win32'].append(dict(action=1,source=13,message=0x202,qpc=105,result=1,capture=0))
        self.assertEqual(self.boundary(r),'retrieved_release_rewritten_to_null')
    def test_delayed_retrieval_with_live_heartbeat(self):
        r=self.raw()
        for w in r['win32']:
            if w['source']!=4:w['qpc']+=5_000_000_000
        a=summarize(r)['actions'][0]
        self.assertGreater(a['x11_to_win32_retrieval_lower_ms'],4900)
        self.assertLess(a['heartbeat_max_ms'],10)
        self.assertIsNone(projected(20_000_000_000,r['brackets']))
    def test_wndproc_up_capture_retained_not_automatically_defect(self):
        r=self.raw();r['win32'][3]['capture']=123456
        self.assertEqual(self.boundary(r),'wndproc_release_capture_still_present')
    def test_capture_clear_gesture_retained_values_after_release(self):
        r=self.raw();r['gui'][-1].update(kind=102,poll_before_ns=110,observed_ns=111)
        self.assertEqual(self.boundary(r),'capture_clear_gesture_end_unobserved')
        self.assertEqual(summarize(r)['actions'][0]['parameter_values_observed_after_release'],1)
        r['gui'][-1]['poll_before_ns']=90
        self.assertEqual(summarize(r)['actions'][0]['parameter_values_observed_after_release'],0)
    def test_procedure_entry_without_return_never_claims_complete_delivery(self):
        r=self.raw();r['win32']=[w for w in r['win32'] if w['source']!=15]
        self.assertEqual(self.boundary(r),'wndproc_release_return_unobserved')
    def test_pinned_fixture_imports_its_own_cleanup_owner(self):
        import importlib.util
        spec=importlib.util.spec_from_file_location('uio3_fixture_test',HERE/'fixture.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        self.assertEqual(module.finish.__module__,'uio3_session')
    def test_multiple_contacts_do_not_hide_a_late_release_behind_an_early_one(self):
        r=self.raw();r['x11'].append(dict(action=1,kind='core_up',observed_ns=200))
        late=dict(r['win32'][2],qpc=5_000_000_000);r['win32'].append(late)
        a=summarize(r)['actions'][0]
        self.assertEqual(a['core_release_count'],2)
        self.assertGreater(a['windows_release_tail_after_last_x11_release_lower_ms'],4990)
        self.assertEqual(a['release_tail_basis'],'observed_after_all_X11_releases_not_cross_namespace_contact_pairing')
    def test_an_earlier_end_does_not_close_a_later_gesture(self):
        r=self.raw();r['gui'].append(dict(r['gui'][0],observed_ns=120))
        self.assertNotEqual(self.boundary(r),'release_and_gesture_end_observed')
        self.assertEqual(summarize(r)['actions'][0]['gesture_open_count'],1)
    def test_pointerup_need_not_generate_core_up(self):
        r=self.raw();r['x11']=[]
        for w in r['win32']:
            if w['message']==0x202:w['message']=0x247
        self.assertEqual(self.boundary(r),'release_and_gesture_end_observed')
        self.assertTrue(summarize(r)['actions'][0]['pointer_up_observed'])
    def test_wm_touch_up_detail_route(self):
        r=self.raw();r['x11']=[]
        r['win32']=[dict(action=1,source=12,message=0x240,qpc=102,capture=0,key_class=1,buttons=4),dict(action=1,source=15,message=0x240,qpc=105,capture=0,result=0)]
        self.assertEqual(self.boundary(r),'release_and_gesture_end_observed')
        self.assertTrue(summarize(r)['actions'][0]['touch_up_observed'])
    def test_sent_call_hook_is_not_actual_procedure_delivery(self):
        r=self.raw()
        for w in r['win32']:
            if w['source']==14:w['source']=2
            if w['source']==15:w['source']=3
        self.assertEqual(self.boundary(r),'core_release_without_win32_dispatch')
    def test_mapping_failure_still_detaches_and_closes_every_owner(self):
        from session import Session
        from unittest.mock import Mock
        from types import SimpleNamespace
        import tempfile,json
        with tempfile.TemporaryDirectory() as d:
            fault=SimpleNamespace(snapshot=lambda:{},close=Mock())
            s=Session(SimpleNamespace(admission={'profile_fingerprint':'a'*64},fault=fault),pathlib.Path(d))
            s.obs=SimpleNamespace(action=Mock(),stop=Mock(),take=Mock(side_effect=ValueError('broken record')),status=Mock(),close=Mock())
            s.gui=SimpleNamespace(dropped=[0,0],close=Mock());s.graph=SimpleNamespace(close=Mock());s.x=SimpleNamespace(close=Mock())
            s.close()
            s.obs.stop.assert_called_once();s.obs.close.assert_called_once();s.gui.close.assert_called_once();s.graph.close.assert_called_once();s.x.close.assert_called_once();fault.close.assert_called_once()
            result=json.loads((pathlib.Path(d)/'timeline.json').read_text())
            self.assertFalse(result['completed']);self.assertEqual(result['cleanup']['errors'],['ValueError'])
    def test_bounds_terminal_and_no_touch_are_not_success(self):
        r=self.raw();r['drops']['win32']=1;self.assertEqual(self.boundary(r),'incomplete_observation_capacity')
        r['terminal']={'secret':'must not export'};self.assertEqual(self.boundary(r),'terminal_instance_failure')
        r=self.raw();r['raw']=[];r['x11']=[];r['win32']=[]
        self.assertEqual(self.boundary(r),'release_not_observed')
    def test_summary_excludes_raw_identifiers_payloads_and_paths(self):
        import json
        r=self.raw();r['path']='SECRET_PATH';r['stop']='FileNotFoundError: SECRET_PATH';r['cleanup']={'helper':{'cleanup':{'pid':123456,'owned_descendants_zero':True,'process_group_empty':True}}}
        s=json.dumps(summarize(r));self.assertNotIn('SECRET',s);self.assertNotIn('123456',s);self.assertNotIn('hwnd',s)
    def test_mailbox_no_input_coordinates_retries_or_skips(self):
        for bad in ({'point':[1,2]},True,'1',1.0,4,-1):
            with self.assertRaises(ValueError):Labels().accept(bad)
        s=Labels()
        with self.assertRaises(ValueError):s.accept(2)
        self.assertTrue(s.accept(1));self.assertFalse(s.accept(1));self.assertTrue(s.accept(2))
        with self.assertRaises(ValueError):s.accept(1)
        self.assertTrue(s.accept(3))
    def test_no_input_calls_in_human_session(self):
        import ast
        tree=ast.parse((HERE/'session.py').read_text())
        self.assertFalse([n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr in ('button','key','move','activate','settle_pointer')])
    def test_helper_bad_cleanup_refuses(self):
        from session import finish
        from types import SimpleNamespace
        good=dict(exit=0,overflow=0,cleanup=dict(owned_descendants_zero=True,process_group_empty=True))
        self.assertEqual(finish(SimpleNamespace(finish=lambda:good)),good)
        for field in ('owned_descendants_zero','process_group_empty'):
            r=copy.deepcopy(good);r['cleanup'][field]=1
            with self.assertRaises(RuntimeError):finish(SimpleNamespace(finish=lambda:r))

if __name__=='__main__':unittest.main()
