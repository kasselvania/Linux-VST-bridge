// Full production Processor::process() + pinned SDK EventList (shallow copy).
// Only the backend C ABI is controlled to force deterministic multiple drains;
// this test makes no new Windows, mailbox, musical or physical-latency claim.
#include "processor.h"
#include "ap10_backend.h"
#include "contained_terminal.h"
#include "ap4_backend.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <cstdio>
#include <cstdlib>
#include <dlfcn.h>
#include <fstream>
#include <memory>
#include <string>
#include <unistd.h>

using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
void require(bool ok, const char* why) {
 if (!ok) { std::fprintf(stderr,"FAIL: %s\n",why); std::exit(1); }
}
uint32_t total=0, cursor=0, drains=0, failures=0, closes=0, seed=0;
bool sysex_only=false, odd=false, terminal_result=false, note_only=false;
bool phase_enabled=false,phase_incomplete=false;
uint32_t phase_queries=0,phase_calls=0,ordinary_calls=0;
int32 frames=0;
TChar character(uint32_t i, uint32_t j) {
 return TChar(0x100 + (seed*97+i*31+j)%0x7000);
}
uint8_t byte(uint32_t i, uint32_t j) { return uint8_t(seed*29+i*37+j); }
Event makeEvent(uint32_t i, std::array<uint8_t,512>& bytes,
                std::array<TChar,256>& text) {
 for (uint32_t j=0;j<bytes.size();++j) bytes[j]=byte(i,j);
 for (uint32_t j=0;j<text.size()-1;++j) text[j]=character(i,j);
 text.back()=0;
 Event e{}; e.sampleOffset=frames ? int32(i%uint32_t(frames)) : 0;
 e.ppqPosition=7.25; e.flags=0xc001;
 if(note_only){e.type=Event::kNoteOnEvent;e.noteOn={15,60,.25f,.75f,0,-1234};return e;}
 switch (sysex_only ? 0 : i%4) {
  case 0: e.type=Event::kDataEvent;
   e.data={odd ? 3u : 512u,DataEvent::kMidiSysEx,bytes.data()}; break;
  case 1: e.type=Event::kNoteExpressionTextEvent;
   e.noteExpressionText={42,-1234,255,text.data()}; break;
  case 2: e.type=Event::kChordEvent;
   e.chord={60,48,-1,255,text.data()}; break;
  case 3: e.type=Event::kScaleEvent;
   e.scale={60,-1,255,text.data()}; break;
 }
 return e;
}
void verify(EventList& sink, uint32_t count) {
 require(sink.getEventCount()==int32(count),"host event count");
 for (uint32_t i=0;i<count;++i) {
  Event e{}; require(sink.getEvent(int32(i),e)==kResultOk,"host getEvent");
  require(e.sampleOffset==(frames ? int32(i%uint32_t(frames)) : 0) &&
          e.busIndex==0 && e.ppqPosition==7.25 && e.flags==0xc001,"event timing/metadata");
  if (sysex_only || i%4==0) {
   require(e.type==Event::kDataEvent && e.data.size==(odd ? 3u : 512u) &&
           e.data.type==DataEvent::kMidiSysEx,"SysEx metadata");
   for (uint32_t j=0;j<e.data.size;++j)
    require(e.data.bytes[j]==byte(i,j),"earlier SysEx overwritten after process return");
  } else {
   const TChar* p=nullptr;
   if (i%4==1) {
    require(e.type==Event::kNoteExpressionTextEvent && e.noteExpressionText.textLen==255 &&
            e.noteExpressionText.noteId==-1234 && e.noteExpressionText.typeId==42,"expression text metadata");
    p=e.noteExpressionText.text;
   } else if (i%4==2) {
    require(e.type==Event::kChordEvent && e.chord.textLen==255 && e.chord.mask==-1,"chord metadata");
    p=e.chord.text;
   } else {
    require(e.type==Event::kScaleEvent && e.scale.textLen==255 && e.scale.mask==-1,"scale metadata");
    p=e.scale.text;
   }
   require(reinterpret_cast<uintptr_t>(p)%alignof(TChar)==0,"UTF-16 alignment");
   for (uint32_t j=0;j<255;++j)
    require(p[j]==character(i,j),"earlier UTF-16 overwritten after process return");
   require(p[255]==0,"UTF-16 terminator");
  }
 }
}
}

extern "C" {
uint32_t __wrap_if2_terminal_status(uint64_t){return terminal_result ? 1 : 0;}
uint32_t __wrap_ap9_open(const uint8_t*,uint64_t* h) {*h=1;return 0;}
uint32_t __wrap_ap22_curve_parameters(uint64_t,const uint32_t*,uint32_t){return 0;}
uint32_t __wrap_ap5_report_path(uint64_t,uint8_t* p,uint32_t n) {if(n)*p=0;return 0;}
uint32_t __wrap_ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*,uint32_t,uint32_t,uint32_t* traits) {
 traits[0]=512;traits[1]=traits[2]=0;return 0;
}
uint32_t __wrap_ap4_activate(uint64_t,uint32_t,uint32_t) {return 0;}
uint32_t __wrap_ap4_deactivate(uint64_t) {return 0;}
uint32_t __wrap_ap3_transition(uint64_t,uint32_t) {return 0;}
uint32_t __wrap_if2_close(uint64_t) {++closes;return 0;}
uint32_t __wrap_ap10_fail_results(uint64_t) {++failures;return 0;}
uint32_t __wrap_ap23_phase_trace_enabled(uint64_t,uint32_t* enabled) {
 ++phase_queries;*enabled=phase_enabled?1:0;return 0;
}
uint32_t __wrap_ap23_process_outputs(uint64_t,uint32_t n,uint32_t mode,const ap8_event_t*,uint32_t,
 const ap10_context_t*,uint64_t,const float* l,const float* r,float*const* outputs,uint32_t channels,
 uint64_t* silence,ap7_delivery_t* delivery,uint64_t entered_ns) {
 require(entered_ns!=0,"native callback-entry clock");
 ++ordinary_calls;
 require(channels==2 && mode==kRealtime,"actual callback mode and planar ABI");
 if(terminal_result)return IF2::contained;
 frames=int32(n);cursor=drains=0;*silence=0;*delivery={};delivery->delivered_frames=n;
 std::copy_n(l,n,outputs[0]);std::copy_n(r,n,outputs[1]);return 0;
}
uint32_t __wrap_ap23_process_outputs_trace(uint64_t,uint32_t n,uint32_t mode,const ap8_event_t*,uint32_t,
 const ap10_context_t*,uint64_t,const float* l,const float* r,float*const* outputs,uint32_t channels,
 uint64_t* silence,ap7_delivery_t* delivery,uint64_t entered_ns,ap23_phase_trace_t* trace) {
 require(trace&&trace->schema==1&&trace->size==sizeof(*trace),"versioned phase record");
 require(entered_ns==trace->cpp_entry_ns&&channels==2&&mode==kRealtime,"phase call identity");
 ++phase_calls;
 trace->rust_entry_ns=entered_ns+10;trace->phase_reached|=AP23::phase_rust_entry;
 trace->clock_valid|=AP23::phase_rust_entry;
 if(phase_incomplete){
  trace->rust_pre_return_ns=entered_ns+20;trace->phase_reached|=AP23::phase_rust_pre_return;
  trace->clock_valid|=AP23::phase_rust_pre_return;
  return AP23::cancelled;
 }
 trace->generation=7;trace->epoch=phase_calls<3?1:2;trace->host_call=phase_calls;
 trace->position=phase_calls-1;trace->phase_reached|=AP23::phase_identity;
 trace->delivery_mode=0;trace->exact=0;trace->allowance_ns=n*1000000000ull/48000;
 trace->wait_deadline_lower_ns=entered_ns+trace->allowance_ns;
 trace->wait_deadline_upper_ns=trace->wait_deadline_lower_ns+2;
 trace->phase_reached|=AP23::phase_policy;trace->clock_valid|=AP23::phase_policy;
 trace->predicate_kind=0;trace->predicate_required=phase_calls;
 trace->predicate_before=phase_calls-1;trace->predicate_after=phase_calls;
 trace->predicate_initial_satisfied=0;trace->predicate_final_satisfied=1;
 trace->phase_reached|=AP23::phase_predicate_before|AP23::phase_predicate_after|AP23::phase_wait;
 trace->wait_begin_ns=entered_ns+11;trace->wait_end_ns=entered_ns+12;
 trace->wait_total_ns=1;trace->wait_count=1;
 trace->clock_valid|=AP23::phase_wait;
 trace->presentation_done_ns=entered_ns+13;trace->phase_reached|=AP23::phase_presentation_done;
 trace->clock_valid|=AP23::phase_presentation_done;
 trace->backend_result=0;trace->phase_reached|=AP23::phase_backend_result;
 trace->rust_pre_return_ns=entered_ns+14;trace->phase_reached|=AP23::phase_rust_pre_return;
 trace->clock_valid|=AP23::phase_rust_pre_return;
 frames=int32(n);cursor=drains=0;*silence=0;*delivery={};delivery->delivered_frames=n;
 std::copy_n(l,n,outputs[0]);std::copy_n(r,n,outputs[1]);return 0;
}
uint32_t __wrap_ap10_take_results(uint64_t,ap10_results_t* packet) {
 packet->events=packet->points=packet->bytes=packet->reserved=0;
 if (cursor==total) return 0;
 ++drains;
 while (cursor<total) {
  std::array<uint8_t,512> bytes{};std::array<TChar,256> text{};
  auto e=makeEvent(cursor,bytes,text);
  if (!AP10Results::append(*packet,e,frames)) break;
  ++cursor;
 }
 return 0;
}
}

int main() {
 auto begin=reinterpret_cast<void(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
 auto end=reinterpret_cast<uint64_t(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
 require(begin && end,"run with the existing callback audit library preloaded");
 HostApplication host;
 auto p=std::make_unique<AP2::Processor>();
 require(p->initialize(&host)==kResultOk,"initialize");
 require(p->activateBus(kEvent,kOutput,0,true)==kResultOk,"activate declared output");
 ProcessSetup setup{kRealtime,kSample32,128,48000.};
 require(p->setupProcessing(setup)==kResultOk && p->setActive(true)==kResultOk &&
         p->setProcessing(true)==kResultOk,"start");
 EventList sink(600); // SDK addEvent copies Event bytes, including pointers.
 std::array<float,128> l{},r{},ol{},or_{};l.fill(.25f);r.fill(-.5f);
 float* ins[]={l.data(),r.data()};float* outs[]={ol.data(),or_.data()};
 AudioBusBuffers input{},output{};
 input.numChannels=output.numChannels=2;input.channelBuffers32=ins;output.channelBuffers32=outs;
 ProcessData d{};d.processMode=kRealtime;d.symbolicSampleSize=kSample32;
 d.outputEvents=&sink;
 auto run=[&](uint32_t n,int32 samples,bool only,bool odd_size,bool overflow) {
  total=n;sysex_only=only;odd=odd_size;++seed;sink.clear();
  d.numSamples=samples;d.numInputs=d.numOutputs=samples ? 1 : 0;
  d.inputs=samples ? &input : nullptr;d.outputs=samples ? &output : nullptr;
  auto prior_failures=failures;
  begin();auto status=p->process(d);auto effects=end();
  require(effects==0,"callback allocation/blocking/I/O effects");
  require(status==(overflow ? kResultFalse : kResultOk),"process capacity status");
  require(failures==prior_failures+(overflow ? 1 : 0),"explicit backend failure");
  require(drains>1,"multiple native drains actually exercised");
  verify(sink,overflow ? 512 : n); // Deliberately only AFTER the whole callback.
  if (samples) for (int32 i=0;i<samples;++i)
   require(ol[i]==(overflow ? 0.f : l[i]) && or_[i]==(overflow ? 0.f : r[i]),"audio/failure silence");
  std::printf("PASS: %u payloads, %u drains, %d frames, %s, zero callback effects\n",
              n,drains,samples,overflow ? "explicit capacity failure" : "stable after return");
 };
 run(9,128,true,false,false); // Reviewer's 8+1 SysEx reproducer.
 run(37,128,false,true,false); // Mixed odd SysEx and all three UTF-16 variants.
 for (int i=0;i<3;++i) run(512,128,false,false,false); // Exact bound and reuse.
 run(512,0,false,false,false); // Zero-frame path also resets once per callback.
 run(513,128,false,false,true); // Controlled over-capacity ABI response.
 // Failure remains latched; no remaining result is silently deferred.
 auto old_drains=drains;
 begin();auto status=p->process(d);auto effects=end();
 require(status==kResultFalse && effects==0 && drains==old_drains,"latched failure");
 require(p->setProcessing(false)==kResultOk && p->setActive(false)==kResultOk,"stop failed instance");
 require(p->terminate()==kResultOk && closes==1,"latched processing failure still permits confirmed owner retirement");
 // Containment preserves local returned-note cleanup without consulting the
 // dead Windows result source. First callback and sink-absent retry are audited.
 p=std::make_unique<AP2::Processor>();
 require(p->initialize(&host)==kResultOk && p->activateBus(kEvent,kOutput,0,true)==kResultOk,"IF2 note fixture initialize");
 require(p->setupProcessing(setup)==kResultOk && p->setActive(true)==kResultOk && p->setProcessing(true)==kResultOk,"IF2 note fixture start");
 total=1;note_only=true;terminal_result=false;sink.clear();
 d.numSamples=128;d.numInputs=d.numOutputs=1;d.inputs=&input;d.outputs=&output;d.outputEvents=&sink;
 begin();status=p->process(d);effects=end();
 require(status==kResultOk && effects==0 && sink.getEventCount()==1,"IF2 initial returned Note On");
 const auto completedDrains=drains;const auto completedFailures=failures;
 terminal_result=true;sink.clear();d.outputEvents=nullptr;
 begin();status=p->process(d);effects=end();
 require(status==kResultOk && effects==0 && drains==completedDrains && failures==completedFailures,"IF2 first terminal callback is silent success without dead forwarding");
 require(output.silenceFlags==3 && std::all_of(ol.begin(),ol.end(),[](float x){return x==0;}) && std::all_of(or_.begin(),or_.end(),[](float x){return x==0;}),"IF2 both outputs silent");
 d.outputEvents=&sink;
 begin();status=p->process(d);effects=end();
 Event off{};require(status==kResultOk && effects==0 && sink.getEventCount()==1 && sink.getEvent(0,off)==kResultOk,"IF2 deferred local Note Off");
 require(off.type==Event::kNoteOffEvent && off.busIndex==0 && off.noteOff.channel==15 && off.noteOff.pitch==60 && off.noteOff.noteId==-1234 && off.noteOff.tuning==.25f && off.sampleOffset==0 && off.flags==0xc001 && off.ppqPosition==7.25,"IF2 exact returned note identity and cleanup offset");
 sink.clear();d.numSamples=d.numInputs=d.numOutputs=0;d.inputs=d.outputs=nullptr;
 begin();status=p->process(d);effects=end();
 require(status==kResultFalse && effects==0 && sink.getEventCount()==0 && drains==completedDrains,"exact zero-frame terminal callback fails without duplicating Note Off or draining results");
 require(p->setProcessing(false)==kResultOk && p->setActive(false)==kResultOk && p->terminate()==kResultOk,"IF2 positive contained cleanup");

 // Trace opt-in is sampled once after successful inactive setup. Toggling the
 // test source later cannot create callback storage or phase output lazily.
 char disabled_name[]="/tmp/ap23-phase-disabled-XXXXXX";
 int disabled_fd=mkstemp(disabled_name);require(disabled_fd>=0,"disabled report create");close(disabled_fd);
 setenv("LVB_AP3_REPORT",disabled_name,1);phase_enabled=false;terminal_result=false;note_only=false;
 auto before_queries=phase_queries,before_phase=phase_calls;
 p=std::make_unique<AP2::Processor>();
 require(p->initialize(&host)==kResultOk,"disabled trace initialize");
 require(p->setupProcessing(setup)==kResultOk,"disabled trace setup");
 phase_enabled=true;
 require(p->setupProcessing(setup)==kResultOk,"disabled trace ressetup");
 require(phase_queries==before_queries+1,"phase opt-in sampled once");
 require(p->setActive(true)==kResultOk&&p->setProcessing(true)==kResultOk,"disabled trace start");
 total=0;d.numSamples=1;d.numInputs=d.numOutputs=1;d.inputs=&input;d.outputs=&output;d.outputEvents=nullptr;
 begin();status=p->process(d);effects=end();require(status==kResultOk&&effects==0,"disabled trace callback unchanged");
 require(phase_calls==before_phase,"disabled trace never calls phase ABI");
 require(p->setProcessing(false)==kResultOk&&p->setActive(false)==kResultOk&&p->terminate()==kResultOk,"disabled trace retire");
 {std::ifstream file(disabled_name);std::string text((std::istreambuf_iterator<char>(file)),{});
  require(text.find("ap23_callback_phase")==std::string::npos,"disabled trace has no export");}
 unlink(disabled_name);

 // A bounded trace survives ordinary stop/setup/start reconfiguration, exports
 // only after quiescence, and omits rather than overwrites after capacity.
 char enabled_name[]="/tmp/ap23-phase-enabled-XXXXXX";
 int enabled_fd=mkstemp(enabled_name);require(enabled_fd>=0,"enabled report create");close(enabled_fd);
 setenv("LVB_AP3_REPORT",enabled_name,1);phase_enabled=true;phase_incomplete=false;
 before_queries=phase_queries;before_phase=phase_calls;auto before_ordinary=ordinary_calls;
 p=std::make_unique<AP2::Processor>();require(p->initialize(&host)==kResultOk,"enabled trace initialize");
 require(p->setupProcessing(setup)==kResultOk&&p->setActive(true)==kResultOk&&p->setProcessing(true)==kResultOk,"enabled trace first start");
 auto run_phase=[&]{begin();auto result=p->process(d);auto audit=end();require(result==kResultOk&&audit==0,"traced callback result/effects");};
 run_phase();run_phase();
 require(p->setProcessing(false)==kResultOk&&p->setActive(false)==kResultOk,"enabled trace first stop");
 require(p->setupProcessing(setup)==kResultOk&&p->setActive(true)==kResultOk&&p->setProcessing(true)==kResultOk,"enabled trace reconfiguration");
 for(int i=2;i<130;++i)run_phase();
 {std::ifstream file(enabled_name);std::string text((std::istreambuf_iterator<char>(file)),{});
  require(text.find("ap23_callback_phase")==std::string::npos,"trace exported before quiescent termination");}
 require(phase_queries==before_queries+1&&phase_calls==before_phase+128,"single sample and bounded traced calls");
 require(ordinary_calls==before_ordinary+2,"overflow callbacks use unchanged ordinary ABI");
 require(p->setProcessing(false)==kResultOk&&p->setActive(false)==kResultOk&&p->terminate()==kResultOk,"enabled trace retire");
 {std::ifstream file(enabled_name);std::string text((std::istreambuf_iterator<char>(file)),{});
  size_t count=0,at=0;while((at=text.find("\"event\":\"ap23_callback_phase\"",at))!=std::string::npos){++count;++at;}
  require(count==128,"exact retained trace capacity");
  require(text.find("\"ordinal\":128")!=std::string::npos&&text.find("\"omitted\":2")!=std::string::npos,"ordinal persists and overflow explicit");
  require(text.find("guard-held calls reaching AP23")!=std::string::npos&&text.find("\"cpp_final_stamp\":\"pre-return\"")!=std::string::npos,"trace scope labels");
  require(text.find("\"clock_valid_uses_phase_bits\":true")!=std::string::npos,"complete trace summary");}
 unlink(enabled_name);

 // A failed backend call retains only phases actually reached. It never
 // fabricates a completion predicate, presentation or result-delivery stamp.
 char incomplete_name[]="/tmp/ap23-phase-incomplete-XXXXXX";
 int incomplete_fd=mkstemp(incomplete_name);require(incomplete_fd>=0,"incomplete report create");close(incomplete_fd);
 setenv("LVB_AP3_REPORT",incomplete_name,1);phase_incomplete=true;
 p=std::make_unique<AP2::Processor>();require(p->initialize(&host)==kResultOk&&p->setupProcessing(setup)==kResultOk&&
  p->setActive(true)==kResultOk&&p->setProcessing(true)==kResultOk,"incomplete trace start");
 begin();status=p->process(d);effects=end();require(status==kResultFalse&&effects==0,"incomplete backend failure preserved");
 require(p->setProcessing(false)==kResultOk&&p->setActive(false)==kResultOk&&p->terminate()==kResultOk,"incomplete trace retire");
 {std::ifstream file(incomplete_name);std::string text((std::istreambuf_iterator<char>(file)),{});
  require(text.find("\"phase_reached\":6915")!=std::string::npos&&
    text.find("\"clock_valid\":2307")!=std::string::npos,"incomplete phase/clock masks");
  require(text.find("\"result\":[265,1]")!=std::string::npos,"backend and SDK failure retained");}
 unlink(incomplete_name);unsetenv("LVB_AP3_REPORT");
 std::puts("Native callback payload lifetime, contained terminal silence and exact Note Off cleanup PASS");
}
