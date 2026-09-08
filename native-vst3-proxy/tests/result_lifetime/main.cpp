// Full production Processor::process() + pinned SDK EventList (shallow copy).
// Only the backend C ABI is controlled to force deterministic multiple drains;
// this test makes no new Windows, mailbox, musical or physical-latency claim.
#include "processor.h"
#include "ap10_backend.h"
#include "ap4_backend.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <cstdio>
#include <cstdlib>
#include <dlfcn.h>
#include <memory>

using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
void require(bool ok, const char* why) {
 if (!ok) { std::fprintf(stderr,"FAIL: %s\n",why); std::exit(1); }
}
uint32_t total=0, cursor=0, drains=0, failures=0, closes=0, seed=0;
bool sysex_only=false, odd=false;
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
uint32_t __wrap_ap9_open(const uint8_t*,uint64_t* h) {*h=1;return 0;}
uint32_t __wrap_ap5_report_path(uint64_t,uint8_t* p,uint32_t n) {if(n)*p=0;return 0;}
uint32_t __wrap_ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*,uint32_t,uint32_t,uint32_t* traits) {
 traits[0]=512;traits[1]=traits[2]=0;return 0;
}
uint32_t __wrap_ap4_activate(uint64_t,uint32_t,uint32_t) {return 0;}
uint32_t __wrap_ap4_deactivate(uint64_t) {return 0;}
uint32_t __wrap_ap3_transition(uint64_t,uint32_t) {return 0;}
uint32_t __wrap_ap3_close(uint64_t) {++closes;return 0;}
uint32_t __wrap_ap10_fail_results(uint64_t) {++failures;return 0;}
uint32_t __wrap_ap10_process(uint64_t,uint32_t n,const ap8_event_t*,uint32_t,
 const ap10_context_t*,uint64_t,const float* l,const float* r,float* ol,float* or_,
 uint64_t* silence,ap7_delivery_t* delivery) {
 frames=int32(n);cursor=drains=0;*silence=0;*delivery={};delivery->delivered_frames=n;
 std::copy_n(l,n,ol);std::copy_n(r,n,or_);return 0;
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
 require(p->terminate()!=kResultOk && closes==1,"failed close remains explicit and releases owner");
 std::puts("Native callback payload lifetime and bounded failure PASS");
}
