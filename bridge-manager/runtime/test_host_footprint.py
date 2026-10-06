"""Windows host family scheduling weight: raised nice for everything but audio."""
import os,pathlib,tempfile,unittest
from unittest.mock import patch
import session


class FakeSystem:
    SCHED_OTHER=0;PRIO_PROCESS=0
    def __init__(self,priority,policy):self.priority=dict(priority);self.policy=dict(policy);self.calls=[]
    def sched_getscheduler(self,tid):
        if tid not in self.policy:raise ProcessLookupError(tid)
        return self.policy[tid]
    def getpriority(self,which,tid):return self.priority[tid]
    def setpriority(self,which,tid,value):
        if value<self.priority[tid]:raise PermissionError('lowering nice needs privilege')
        self.priority[tid]=value;self.calls.append((tid,value))


def stat(path,ident,start):
    fields=['S']+['0']*18+[str(start)]+['0']*5
    path.write_text(f'{ident} (name with ) paren) {" ".join(fields)}\n')


class HostFootprintTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=pathlib.Path(self.tmp.name)
    def process(self,pid,start,threads):
        p=self.root/str(pid);(p/'task').mkdir(parents=True);stat(p/'stat',pid,start)
        for tid,comm in threads.items():
            t=p/'task'/str(tid);t.mkdir();(t/'comm').write_text(comm+'\n');stat(t/'stat',tid,start+1)
    def footprint(self,system,env=None,clock=lambda:0.0):
        with patch.dict(os.environ,env or {},clear=False):
            return session.HostFootprint({'session':'42'*16},proc_root=self.root,system=system,clock=clock)

    def test_raises_every_ordinary_thread_except_the_audio_render_thread(self):
        self.process(100,900,{101:'lvb-audio',102:'host.exe',103:'sh_opt0',104:'winedevice'})
        self.process(200,950,{200:'wineserver'})
        system=FakeSystem({101:0,102:0,103:0,104:12,200:0},{101:2,102:0,103:0,104:0,200:0})
        f=self.footprint(system)
        f.poll({(100,900),(200,950)}) # no render start yet, but a new family: one pass
        self.assertEqual(sorted(system.calls),[(102,10),(103,10),(200,10)])
        self.assertEqual(system.priority[101],0) # the real-time audio thread is untouched
        self.assertEqual(system.priority[104],12) # never lowered
        v=f.value()
        self.assertEqual((v['passes'],v['applied'],v['already'],v['skipped_audio'],v['skipped_policy'],v['errors']),(1,3,1,1,0,0))

    def test_render_start_repeats_the_pass_and_idle_polls_are_throttled(self):
        self.process(100,900,{102:'host.exe'})
        system=FakeSystem({102:0},{102:0});now=[0.0]
        f=self.footprint(system,clock=lambda:now[0])
        f.poll({(100,900)});self.assertEqual(f.value()['passes'],1)
        f.poll({(100,900)});f.poll({(100,900)});self.assertEqual(f.value()['passes'],1)
        system.priority[102]=0 # a thread that reverted (fresh start) is covered by the next render start
        f.started();f.poll({(100,900)})
        self.assertEqual(f.value()['passes'],2);self.assertEqual(system.priority[102],10)
        # Family growth within the throttle window still waits a second.
        self.process(300,970,{300:'wineserver'});system.priority[300]=0;system.policy[300]=0
        now[0]+=0.5;f.poll({(100,900),(300,970)});self.assertEqual(f.value()['passes'],2)
        now[0]+=0.6;f.poll({(100,900),(300,970)});self.assertEqual(f.value()['passes'],3)
        self.assertEqual(system.priority[300],10)

    def test_recycled_pid_vanished_thread_and_non_ordinary_policies_are_skipped(self):
        self.process(100,900,{102:'host.exe',105:'gone',106:'batch'})
        system=FakeSystem({102:0,106:0},{102:0,106:3}) # 105 has no scheduler entry: vanished
        f=self.footprint(system)
        f.poll({(100,901)}) # start ticks differ: a recycled pid is never touched
        self.assertEqual(system.calls,[])
        f.started();f.poll({(100,900)})
        self.assertEqual(system.calls,[(102,10)])
        v=f.value();self.assertEqual((v['applied'],v['skipped_policy'],v['errors']),(1,1,1))

    def test_operator_value_disables_tunes_and_rejects_nonsense(self):
        self.process(100,900,{102:'host.exe'})
        system=FakeSystem({102:0},{102:0})
        f=self.footprint(system,{'LVB_HOST_NICE':'0'});f.started();f.poll({(100,900)})
        self.assertEqual(system.calls,[]);self.assertEqual(f.value()['nice'],0)
        f=self.footprint(system,{'LVB_HOST_NICE':'15'});f.started();f.poll({(100,900)})
        self.assertEqual(system.calls,[(102,15)])
        f=self.footprint(system,{'LVB_HOST_NICE':'25'});f.started();f.poll({(100,900)})
        self.assertEqual(f.value()['unavailable'],'host_nice_value_invalid');self.assertEqual(len(system.calls),1)
        f=self.footprint(system,{'LVB_HOST_NICE':'ten'})
        self.assertEqual(f.value()['unavailable'],'host_nice_value_invalid')


if __name__=='__main__':unittest.main()
