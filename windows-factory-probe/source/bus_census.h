#pragma once
#include "bus_layout.h"
#include <exception>
#include <thread>
namespace linux_vst_bridge::wf0 {
// Inspection-only raw snapshots. Never substitutes a count into processing or
// changes the original BusLayout; fixed SDK data, no vendor payloads.
struct EventBusCensus {
 Steinberg::int32 count=0;
 Steinberg::tresult result=Steinberg::kResultFalse;
 Steinberg::Vst::BusInfo info{};
 bool activated=false;
 struct Audio {int direction=0,index=0;Steinberg::tresult result=Steinberg::kResultFalse;Steinberg::Vst::SpeakerArrangement arrangement=0;};
 std::array<Audio,16> audio{};size_t audio_size=0;
 static EventBusCensus capture(Steinberg::Vst::IComponent& c,Steinberg::Vst::IAudioProcessor& p,bool active){
  using namespace Steinberg::Vst;EventBusCensus row;row.activated=active;
  row.count=c.getBusCount(kEvent,kOutput);ap1::require(row.count>=0&&row.count<=8,"event census count bound");
  if(row.count)row.result=c.getBusInfo(kEvent,kOutput,0,row.info);
  for(int d=0;d<2;++d){auto n=c.getBusCount(kAudio,d);ap1::require(n>=0&&n<=8,"audio census count bound");
   for(int i=0;i<n;++i){auto& a=row.audio[row.audio_size++];a.direction=d;a.index=i;a.result=p.getBusArrangement(d,i,a.arrangement);}}
  return row;
 }
};
// Six finite observations on one initialized instance. setProcessing and its
// snapshot run on the processing thread, with the native owner quiescent.
// Failure after activation must unwind before the caller can release objects.
template<class Emit,class Step,class Check>
void run_event_bus_census(Steinberg::Vst::IComponent& c,Steinberg::Vst::IAudioProcessor& p,Emit emit,Step step,Check check){
 using namespace Steinberg;using namespace Steinberg::Vst;
 BusLayout layout;layout.read(c,p);bool active=false,buses=false;std::exception_ptr error;
 try{
  step("censusSetBusArrangements");layout.negotiate(p);check(kResultOk,"censusSetBusArrangements");
  emit("arrangements",EventBusCensus::capture(c,p,false));
  ProcessSetup setup{};setup.processMode=kRealtime;setup.symbolicSampleSize=kSample32;setup.maxSamplesPerBlock=512;setup.sampleRate=48000.;
  step("censusSetupProcessing");check(p.setupProcessing(setup),"censusSetupProcessing");emit("setup",EventBusCensus::capture(c,p,false));
  step("censusActivateBuses");buses=true;layout.activate(c,true);
  // Observe this bus explicitly even if the vendor did not default-activate it.
  ap1::require(layout.counts[3]==1,"census requires one event output");
  check(c.activateBus(kEvent,kOutput,0,true),"censusActivateBuses");emit("bus_active",EventBusCensus::capture(c,p,true));
  step("censusSetActive");active=true;check(c.setActive(true),"censusSetActive");emit("active",EventBusCensus::capture(c,p,true));
  EventBusCensus processing;std::exception_ptr worker_error;tresult processing_result=kNotInitialized;
  step("censusSetProcessing");
  std::thread worker([&]{
   try{processing_result=p.setProcessing(true);ap1::require(processing_result==kResultOk||processing_result==kNotImplemented,"census setProcessing refused");processing=EventBusCensus::capture(c,p,true);}
   catch(...){worker_error=std::current_exception();}
   // No object release if stopped processing cannot be established.
   try{auto result=p.setProcessing(false);if(result!=kResultOk&&result!=kNotImplemented)std::terminate();}catch(...){std::terminate();}
  });worker.join();if(worker_error)std::rethrow_exception(worker_error);
  check(processing_result,"censusSetProcessing");emit("processing",processing);
 }catch(...){error=std::current_exception();}
 try{if(active){step("censusSetInactive");check(c.setActive(false),"censusSetInactive");}if(buses){step("censusDeactivateBuses");layout.activate(c,false);check(kResultOk,"censusDeactivateBuses");}}
 catch(...){std::terminate();} // incomplete SDK retirement remains contained
 if(error)std::rethrow_exception(error);
}
}
