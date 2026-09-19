#include "bus_census.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <cassert>
#include <vector>
#include <string>
using namespace Steinberg;using namespace Steinberg::Vst;using namespace linux_vst_bridge::wf0;
namespace ap1=linux_vst_bridge::ap1;
struct Plugin:AudioEffect {
 int initial=0,operational=0;bool negotiated=false;int active=0,processing=0,process_calls=0;
 tresult PLUGIN_API initialize(FUnknown* h)override{auto r=AudioEffect::initialize(h);addAudioInput(u"Sidechain",SpeakerArr::kStereo,kAux);addAudioOutput(u"Out",SpeakerArr::kStereo);addEventInput(u"Midi In",16);addEventOutput(u"Midi Out",initial);return r;}
 tresult PLUGIN_API getBusInfo(MediaType m,BusDirection d,int32 i,BusInfo& b)override{auto r=AudioEffect::getBusInfo(m,d,i,b);if(r==kResultOk&&m==kEvent&&d==kOutput)b.channelCount=negotiated?operational:initial;return r;}
 tresult PLUGIN_API setBusArrangements(SpeakerArrangement* in,int32 ni,SpeakerArrangement* out,int32 no)override{assert(ni==1&&no==1&&in[0]==SpeakerArr::kStereo&&out[0]==SpeakerArr::kStereo);negotiated=true;return kResultOk;}
 tresult PLUGIN_API setActive(TBool on)override{active+=on?1:-1;return kResultOk;}
 tresult PLUGIN_API setProcessing(TBool on)override{processing+=on?1:-1;return kResultOk;}
 tresult PLUGIN_API process(ProcessData&)override{++process_calls;assert(false);return kResultFalse;}
};
struct MultiOutput:AudioEffect {
 std::array<bool,32> enabled{};
 tresult PLUGIN_API initialize(FUnknown* h)override{
  auto r=AudioEffect::initialize(h);
  for(int i=0;i<32;++i)addAudioOutput(u"Output",SpeakerArr::kStereo,kMain);
  addEventInput(u"Notes",16);addEventOutput(u"Notes",16);return r;
 }
 tresult PLUGIN_API setBusArrangements(SpeakerArrangement*,int32 ni,SpeakerArrangement* out,int32 no)override{
  assert(ni==0&&no==32);for(int i=0;i<no;++i)assert(out[i]==SpeakerArr::kStereo);return kResultOk;
 }
 tresult PLUGIN_API activateBus(MediaType m,BusDirection d,int32 i,TBool on)override{
  if(m==kAudio&&d==kOutput)enabled[size_t(i)]=on!=0;
  return AudioEffect::activateBus(m,d,i,on);
 }
};
int main(){
 {
  HostApplication host;MultiOutput plugin;assert(plugin.initialize(&host)==kResultOk);
  BusLayout layout;layout.read(plugin,plugin,false);assert(layout.size==34&&layout.counts[1]==32);
  layout.negotiate(plugin);layout.activate(plugin,true);
  assert(plugin.enabled[0]);for(size_t i=1;i<32;++i)assert(!plugin.enabled[i]);
  float left=1,right=2;float* channels[]={&left,&right};float* inactive[]={nullptr,nullptr};
  std::array<AudioBusBuffers,32> buffers{};layout.map_outputs(buffers,channels,inactive);
  assert(buffers[0].channelBuffers32==channels&&buffers[0].numChannels==2);
  for(size_t i=1;i<32;++i)assert(buffers[i].numChannels==2&&buffers[i].channelBuffers32==inactive&&buffers[i].silenceFlags==3);
  std::vector<uint8_t> contract(28+32*layout.size);ap1::put(contract.data()+20,1,4);ap1::put(contract.data()+24,layout.size,4);
  std::array<int,4> indices{};
  for(size_t i=0;i<layout.size;++i){auto& b=layout.buses[i];auto* p=contract.data()+28+32*i;
   ap1::put(p,b.info.mediaType,4);ap1::put(p+4,b.info.direction,4);ap1::put(p+8,indices[b.info.mediaType*2+b.info.direction]++,4);
   ap1::put(p+12,b.effective_channels,4);ap1::put(p+16,b.info.busType,4);ap1::put(p+20,b.active?1:0,4);ap1::put(p+24,b.arrangement,8);
  }
  layout.contract(contract);ap1::put(contract.data()+28+32*31+20,1,4);
  bool refused=false;try{layout.contract(contract);}catch(const std::exception&){refused=true;}assert(refused);
  layout.activate(plugin,false);for(bool on:plugin.enabled)assert(!on);
  assert(plugin.terminate()==kResultOk);
 }
 for(auto counts: {std::pair{0,0},std::pair{0,16},std::pair{7,7}}){
  HostApplication host;Plugin plugin;plugin.initial=counts.first;plugin.operational=counts.second;assert(plugin.initialize(&host)==kResultOk);
  auto initial=EventBusCensus::capture(plugin,plugin,false);assert(initial.info.channelCount==counts.first&&!initial.activated);
  std::vector<EventBusCensus> rows;std::vector<std::string> stages;
  run_event_bus_census(plugin,plugin,[&](const char* stage,const EventBusCensus& row){stages.emplace_back(stage);rows.push_back(row);},[](const char*){},[](tresult result,const char*){assert(result==kResultOk);});
  assert((stages==std::vector<std::string>{"arrangements","setup","bus_active","active","processing"}));
  assert(rows.size()==5&&initial.info.channelCount==counts.first);
  for(size_t i=0;i<rows.size();++i){auto& row=rows[i];assert(row.count==1&&row.result==kResultOk&&row.info.channelCount==counts.second);assert(row.info.mediaType==kEvent&&row.info.direction==kOutput&&row.info.busType==kMain);assert(row.activated==(i>=2));assert(row.audio_size==2&&row.audio[0].arrangement==SpeakerArr::kStereo&&row.audio[1].arrangement==SpeakerArr::kStereo);}
  assert(plugin.active==0&&plugin.processing==0&&plugin.process_calls==0);assert(plugin.terminate()==kResultOk);
 }
}
