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
struct AdmissionIO {
 int receipt=0;bool publish_during_acquire=false,anchor_alive=true,opened=false,identity=true;
 uint32_t open_error=5,mask=0,witnesses=0,acquisitions=0,refreshes=0,censuses=0;
 uint64_t created=456;Nad1StopStatus fresh{1};
 int exit_witness(uint32_t,uint64_t){++witnesses;return receipt;}
 Nad1ProcessAdmission acquire(uint32_t,uint64_t){
  ++acquisitions;
  if(publish_during_acquire){receipt=1;anchor_alive=false;}
  return {opened,opened&&identity,opened?created:0,opened&&identity?0u:open_error};
 }
 Nad1StopStatus refresh(){++refreshes;return fresh;}
 uint32_t endpoints(uint32_t){++censuses;return mask;}
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
 // Receipt already published before admission remains the direct handoff case.
 AdmissionIO before;before.receipt=1;auto a=nad1_admit_stop_generation(before,{1},0,123,456);
 assert(a.valid&&a.exit_witness&&a.pid==123&&a.created==456&&before.acquisitions==0&&before.witnesses==1);
 IO before_stop{{a.first,{1}}};assert(nad1_observe_stop(before_stop,a.valid).confirmed&&before_stop.controls==0);
 // Exact race: first receipt lookup misses; acquisition publishes the receipt
 // and releases the original anchor handle; one fresh STOPPED/read/census wins.
 AdmissionIO during;during.publish_during_acquire=true;auto b=nad1_admit_stop_generation(during,{1},0,123,456);
 assert(b.valid&&b.exit_witness&&!during.anchor_alive&&during.witnesses==2&&during.acquisitions==1&&during.refreshes==1&&during.censuses==1);
 IO during_stop{{b.first,{1}}};assert(nad1_observe_stop(during_stop,b.valid).confirmed&&during_stop.controls==0);
 // Pending and running observations use the same bounded handoff after failed acquisition.
 for(auto state:{3u,4u}){AdmissionIO crossing;crossing.publish_during_acquire=true;
  auto admitted=nad1_admit_stop_generation(crossing,{state},state==4?123u:0u,123,456);
  assert(admitted.valid&&admitted.exit_witness&&admitted.first.state==1&&crossing.witnesses==1&&crossing.refreshes==1&&crossing.censuses==1);
  IO observed{{admitted.first,{1}}};assert(nad1_observe_stop(observed,admitted.valid).confirmed&&observed.controls==0);
 }
 // Missing/malformed evidence, non-stopped refresh and live listeners refuse.
 AdmissionIO missing;auto m=nad1_admit_stop_generation(missing,{1},0,123,456);assert(!m.valid&&m.identity_error==5&&missing.witnesses==2);
 AdmissionIO malformed;malformed.receipt=-1;auto malformed_result=nad1_admit_stop_generation(malformed,{3},0,123,456);assert(!malformed_result.valid&&malformed_result.identity_error==13);
 AdmissionIO not_stopped;not_stopped.publish_during_acquire=true;not_stopped.fresh={3};assert(!nad1_admit_stop_generation(not_stopped,{3},0,123,456).valid);
 AdmissionIO listeners;listeners.publish_during_acquire=true;listeners.mask=3;assert(!nad1_admit_stop_generation(listeners,{3},0,123,456).valid);
 // An opened process with unresolved identity cannot switch to receipt authority.
 AdmissionIO unresolved;unresolved.opened=true;unresolved.identity=false;unresolved.publish_during_acquire=true;
 auto u=nad1_admit_stop_generation(unresolved,{3},0,123,456);assert(!u.valid&&!u.exit_witness&&unresolved.refreshes==0&&unresolved.witnesses==0);
}
