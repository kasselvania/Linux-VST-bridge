// Exercises the production stop observation law with generated SCM/handle facts.
#include "../is2/nad1_stop.h"
#include <cassert>
#include <vector>
struct IO {
 std::vector<Nad1StopStatus> states;unsigned index=0,controls=0,sleeps=0,begins=0;
 uint64_t clock=0,control_delay=0;uint32_t control_error=0,wait_result=0,mask=0;
 Nad1StopStatus query(){return states[index<states.size()?index++:states.size()-1];}
 uint64_t now(){return clock;}
 void begin_control(uint32_t){++begins;}
 uint32_t stop(){++controls;clock+=control_delay;return control_error;}
 uint32_t wait(uint32_t& error){error=wait_result==0xffffffffu?6:0;return wait_result;}
 uint32_t endpoints(){return mask;}
 void sleep(){++sleeps;clock+=50;}
};
int main(){
 // Pending stop: retain exact handle, never send another control, poll progress.
 IO pending{{{3,1,1000},{3,2,1000},{1,0,0}}};auto p=nad1_observe_stop(pending,true);
 assert(p.confirmed&&pending.controls==0&&p.progress==2&&p.queries==3);
 // Race into pending after the initial running observation rejects control.
 IO racing{{{4},{3,1,500},{1}}};racing.control_error=1061;auto r=nad1_observe_stop(racing,true);
 assert(r.confirmed&&racing.controls==1&&r.control_error==1061);
 // STOPPED has no PID in this law; process generation exit and listeners remain required.
 IO stopped{{{1},{1}}};auto s=nad1_observe_stop(stopped,true);assert(s.confirmed&&stopped.controls==0);
 for(unsigned mask:{1u,2u,3u,4u}){IO x{{{4},{1}}};x.mask=mask;auto y=nad1_observe_stop(x,true);assert(!y.confirmed&&y.endpoints==mask&&y.elapsed==12000);}
 IO live{{{4},{1}}};live.wait_result=258;assert(!nad1_observe_stop(live,true).confirmed);
 IO bad_handle{{{4},{1}}};bad_handle.wait_result=0xffffffffu;auto h=nad1_observe_stop(bad_handle,true);assert(!h.confirmed&&h.wait_error==6);
 IO unowned{{{3},{1}}};assert(!nad1_observe_stop(unowned,false).confirmed&&unowned.controls==0);
 IO query_loss{{{4},{0,0,0,0,0,5}}};auto q=nad1_observe_stop(query_loss,true);assert(!q.confirmed&&q.last.error==5&&query_loss.controls==1);
 IO refused{{{4},{4}}};refused.control_error=5;auto f=nad1_observe_stop(refused,true);assert(!f.confirmed&&refused.controls==1&&f.control_error==5);
 IO delayed{{{4},{3,5,2000},{3,6,2000},{1}}};auto d=nad1_observe_stop(delayed,true);assert(d.confirmed&&d.last.state==1&&d.elapsed==100&&delayed.controls==1);
 IO stalled{{{4},{3,1,1000}}};auto t=nad1_observe_stop(stalled,true);assert(!t.confirmed&&t.elapsed==12000&&t.progress==1&&t.last.checkpoint==1);
 // Time in ControlService counts against the existing bound; no fresh 12s window.
 IO slow_control{{{4},{3}}};slow_control.control_delay=12500;auto c=nad1_observe_stop(slow_control,true);
 assert(!c.confirmed&&c.elapsed==12500&&c.control_elapsed==12500&&slow_control.sleeps==0&&slow_control.controls==1);
}
