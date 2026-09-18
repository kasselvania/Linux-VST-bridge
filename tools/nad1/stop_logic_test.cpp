// Exercises the production stop observation law with generated SCM/handle facts.
#include "../is2/nad1_stop.h"
#include <cassert>
#include <vector>
struct IO {
 std::vector<Nad1StopStatus> states;unsigned index=0,controls=0,sleeps=0,begins=0;
 uint64_t clock=0,control_delay=0;uint32_t control_error=0,wait_result=0,mask=0;
 Nad1StopStatus control_status{};std::vector<uint32_t> waits{},masks{};unsigned wait_index=0,mask_index=0;
 Nad1StopStatus query(){return states[index<states.size()?index++:states.size()-1];}
 uint64_t now(){return clock;}
 void begin_control(uint32_t){++begins;}
 Nad1ControlResult stop(){++controls;clock+=control_delay;if(control_error)return {false,control_error,{}};
  auto status=control_status;if(!status.state&&!states.empty())status=states.front();return {true,0,status};}
 uint32_t wait(uint32_t& error){auto value=wait_index<waits.size()?waits[wait_index++]:wait_result;error=value==0xffffffffu?6:0;return value;}
 uint32_t endpoints(){return mask_index<masks.size()?masks[mask_index++]:mask;}
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
 // Normal submitted stop: complete ControlService status is retained, then
 // exact process and listener retirement establish confirmation.
 IO normal{{{4,0,0,0,0,0,1,16},{3,1,1000,0,0,0,0,16},{1,0,0,0,0,0,0,16}}};
 normal.control_status={3,1,1000,0,0,0,0,16};normal.waits={258,258,258,0};normal.masks={3,3,3,0};
 auto n=nad1_observe_stop(normal,true);
 assert(n.confirmed&&n.classification==Nad2StopClassification::Confirmed&&normal.controls==1&&n.control_submitted);
 assert(n.control_status.state==3&&n.control_status.type==16&&n.control_count==1);
 // Pending stop: retain exact handle, never send another control, poll progress.
 IO pending{{{3,1,1000},{3,2,1000},{1,0,0}}};pending.waits={258,258,0};pending.masks={3,3,0};
 auto p=nad1_observe_stop(pending,true);
 assert(p.confirmed&&p.classification==Nad2StopClassification::Confirmed&&pending.controls==0&&p.progress==2&&p.queries==3);
 // A submitted request with no changed SCM/process/listener fact is not progress.
 IO unchanged{{{4},{4}}};unchanged.wait_result=258;unchanged.mask=3;unchanged.control_status={4};
 auto unchanged_result=nad1_observe_stop(unchanged,true);
 assert(!unchanged_result.confirmed&&unchanged_result.classification==Nad2StopClassification::SubmittedNoTransition);
 assert(unchanged_result.transition_total==1&&unchanged_result.elapsed==12000&&unchanged.controls==1);
 // A controls-accepted mask change is diagnostic evidence, not stop progress.
 IO controls_only{{{4,0,0,0,0,0,1},{4,0,0,0,0,0,2}}};controls_only.wait_result=258;controls_only.mask=3;
 controls_only.control_status={4,0,0,0,0,0,1};
 auto controls_only_result=nad1_observe_stop(controls_only,true);
 assert(controls_only_result.progress==0&&controls_only_result.transition_total==2);
 assert(controls_only_result.classification==Nad2StopClassification::SubmittedNoTransition);
 // The complete ControlService return can itself establish a STOP_PENDING transition.
 IO control_pending{{{4},{4}}};control_pending.wait_result=258;control_pending.mask=3;
 control_pending.control_status={3,1,1000};
 auto control_pending_result=nad1_observe_stop(control_pending,true);
 assert(control_pending_result.control_status_available&&control_pending_result.control_status.state==3);
 assert(control_pending_result.classification==Nad2StopClassification::SubmittedProgressing);
 // An already pending stop may progress without completing and sends no control.
 IO pending_progress{{{3,1,1000},{3,2,900}}};pending_progress.wait_result=258;pending_progress.mask=3;
 auto pending_progress_result=nad1_observe_stop(pending_progress,true);
 assert(!pending_progress_result.confirmed&&pending_progress_result.classification==Nad2StopClassification::SubmittedProgressing);
 assert(pending_progress.controls==0&&pending_progress_result.progress==1);
 // A non-stop SCM state is not evidence of progress toward retirement.
 IO paused{{{4},{7}}};paused.wait_result=258;paused.mask=3;paused.control_status={4};
 auto paused_result=nad1_observe_stop(paused,true);
 assert(paused_result.last.state==7&&paused_result.classification==Nad2StopClassification::ObservationUnavailable);
 // Race into pending after the initial running observation rejects control.
 IO racing{{{4},{3,1,500},{1}}};racing.control_error=1061;auto r=nad1_observe_stop(racing,true);
 assert(r.confirmed&&racing.controls==1&&r.control_error==1061);
 // STOPPED has no PID in this law; process generation exit and listeners remain required.
 IO stopped{{{1},{1}}};auto s=nad1_observe_stop(stopped,true);assert(s.confirmed&&stopped.controls==0);
 for(unsigned mask:{1u,2u,3u}){IO x{{{1},{1}}};x.mask=mask;auto y=nad1_observe_stop(x,true);
  assert(!y.confirmed&&y.endpoints==mask&&y.elapsed==12000&&y.classification==Nad2StopClassification::StoppedProcessOrListenerRemains);}
 IO unknown_listener{{{1},{1}}};unknown_listener.mask=4;auto unknown=nad1_observe_stop(unknown_listener,true);
 assert(!unknown.confirmed&&unknown.classification==Nad2StopClassification::ObservationUnavailable);
 // An unavailable initial listener census is not repaired by a later census.
 IO initial_listener_failure{{{4},{4}}};initial_listener_failure.wait_result=258;initial_listener_failure.mask=3;
 initial_listener_failure.masks={4,3,3};initial_listener_failure.control_status={4};
 auto initial_listener_failure_result=nad1_observe_stop(initial_listener_failure,true);
 assert(initial_listener_failure_result.initial_endpoints==4&&initial_listener_failure_result.endpoints==3);
 assert(initial_listener_failure_result.classification==Nad2StopClassification::ObservationUnavailable);
 IO live{{{1},{1}}};live.wait_result=258;auto live_result=nad1_observe_stop(live,true);
 assert(!live_result.confirmed&&live_result.classification==Nad2StopClassification::StoppedProcessOrListenerRemains);
 IO bad_handle{{{4},{1}}};bad_handle.wait_result=0xffffffffu;auto h=nad1_observe_stop(bad_handle,true);assert(!h.confirmed&&h.wait_error==6);
 // An unavailable initial exact-process wait is not repaired by a later timeout.
 IO initial_wait_failure{{{4},{4}}};initial_wait_failure.waits={0xffffffffu,258,258};initial_wait_failure.mask=3;
 auto initial_wait_failure_result=nad1_observe_stop(initial_wait_failure,true);
 assert(!initial_wait_failure_result.confirmed&&initial_wait_failure_result.initial_wait_error==6);
 assert(initial_wait_failure_result.wait_error==0&&initial_wait_failure_result.classification==Nad2StopClassification::ObservationUnavailable);
 IO unowned{{{3},{1}}};auto unowned_result=nad1_observe_stop(unowned,false);
 assert(!unowned_result.confirmed&&unowned.controls==0&&unowned_result.classification==Nad2StopClassification::ObservationUnavailable);
 IO query_loss{{{4},{0,0,0,0,0,5}}};auto q=nad1_observe_stop(query_loss,true);
 assert(!q.confirmed&&q.last.error==5&&query_loss.controls==1&&q.classification==Nad2StopClassification::ObservationUnavailable);
 IO refused{{{4},{4}}};refused.control_error=5;auto f=nad1_observe_stop(refused,true);
 assert(!f.confirmed&&refused.controls==1&&f.control_error==5&&f.classification==Nad2StopClassification::NotSubmitted);
 IO delayed{{{4},{3,5,2000},{3,6,2000},{1}}};auto d=nad1_observe_stop(delayed,true);assert(d.confirmed&&d.last.state==1&&d.elapsed==100&&delayed.controls==1);
 IO stalled{{{4},{3,1,1000}}};auto t=nad1_observe_stop(stalled,true);assert(!t.confirmed&&t.elapsed==12000&&t.progress==1&&t.last.checkpoint==1);
 // Time in ControlService counts against the existing bound; no fresh 12s window.
 IO slow_control{{{4},{3}}};slow_control.control_delay=12500;auto c=nad1_observe_stop(slow_control,true);
 assert(!c.confirmed&&c.elapsed==12500&&c.control_elapsed==12500&&slow_control.sleeps==0&&slow_control.controls==1);
 // The fixed ledger retains its prefix while counting every later distinct fact.
 std::vector<Nad1StopStatus> many{{4}};for(uint32_t checkpoint=1;checkpoint<=20;checkpoint++)many.push_back({3,checkpoint,1000});
 IO saturated{many};saturated.wait_result=258;saturated.mask=3;saturated.control_status={4};
 auto ledger=nad1_observe_stop(saturated,true);
 assert(ledger.classification==Nad2StopClassification::SubmittedProgressing);
 assert(ledger.transition_total>NAD2_STOP_TRANSITION_CAP&&ledger.transition_retained==NAD2_STOP_TRANSITION_CAP);
 assert(ledger.transition_dropped==ledger.transition_total-ledger.transition_retained);
 assert(ledger.transitions.front().service.state==4&&ledger.transitions.back().service.checkpoint==20);
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
