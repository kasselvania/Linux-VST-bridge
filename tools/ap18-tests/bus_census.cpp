#include "bus_census.h"
#include "stereo_negotiation.h"
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
 tresult PLUGIN_API process(ProcessData& data)override{
  assert(data.numOutputs==32&&data.numSamples==1);
  for(int b=0;b<32;++b)for(int c=0;c<2;++c)data.outputs[b].channelBuffers32[c][0]=float(2*b+c);
  return kResultOk;
 }
 tresult PLUGIN_API activateBus(MediaType m,BusDirection d,int32 i,TBool on)override{
  if(m==kAudio&&d==kOutput)enabled[size_t(i)]=on!=0;
  return AudioEffect::activateBus(m,d,i,on);
 }
};
struct StereoNegotiationPlugin:AudioEffect {
 tresult request_result=kResultOk;SpeakerArrangement current=SpeakerArr::kMono;int calls=0;
 tresult PLUGIN_API initialize(FUnknown* h)override{auto r=AudioEffect::initialize(h);addAudioInput(u"In",SpeakerArr::kMono);addAudioOutput(u"Out",SpeakerArr::kMono);return r;}
 tresult PLUGIN_API getBusInfo(MediaType m,BusDirection d,int32 i,BusInfo& b)override{auto r=AudioEffect::getBusInfo(m,d,i,b);if(r==kResultOk&&m==kAudio)b.channelCount=current==SpeakerArr::kStereo?2:1;return r;}
 tresult PLUGIN_API getBusArrangement(BusDirection,int32,SpeakerArrangement& arrangement)override{arrangement=current;return kResultOk;}
 tresult PLUGIN_API setBusArrangements(SpeakerArrangement* in,int32 ni,SpeakerArrangement* out,int32 no)override{
  assert(ni==1&&no==1&&in&&out&&in[0]==out[0]);++calls;current=in[0];return current==SpeakerArr::kStereo?request_result:kResultOk;
 }
};
int main(){
 {
  HostApplication host;StereoNegotiationPlugin plugin;assert(plugin.initialize(&host)==kResultOk);
  const auto applied=apply_stereo_main_pair(plugin,plugin);
  assert(applied.result==kResultOk&&applied.verified&&applied.readback.stereo_main_pair_readback());
  assert(plugin.current==SpeakerArr::kStereo&&plugin.calls==1&&plugin.terminate()==kResultOk);
 }
 {
  HostApplication host;StereoNegotiationPlugin plugin;plugin.request_result=kResultFalse;assert(plugin.initialize(&host)==kResultOk);
  const auto applied=apply_stereo_main_pair(plugin,plugin);
  assert(applied.result==kResultFalse&&!applied.verified&&applied.readback.stereo_main_pair_readback());
  assert(plugin.terminate()==kResultOk);
 }
 for(auto request_result:{kResultOk,kResultFalse}){
  HostApplication host;StereoNegotiationPlugin plugin;plugin.request_result=request_result;assert(plugin.initialize(&host)==kResultOk);
  std::vector<std::string> stages;std::vector<StereoBusSnapshot> snapshots;StereoNegotiationResult requested{},restored{};
  auto result=run_stereo_negotiation(plugin,plugin,
   [&](const char* stage,const StereoBusSnapshot& row){stages.emplace_back(stage);snapshots.push_back(row);},
   [&](const StereoNegotiationResult& row){requested=row;},[&](const StereoNegotiationResult& row){restored=row;},[](const char*){});
  assert((stages==std::vector<std::string>{"before","after","restored"}));assert(snapshots.size()==3);
  assert(snapshots[0].buses[0].arrangement==SpeakerArr::kMono&&snapshots[0].buses[1].info.channelCount==1);
  assert(snapshots[1].stereo_readback()&&snapshots[2].buses[0].arrangement==SpeakerArr::kMono&&snapshots[2].buses[1].info.channelCount==1);
  assert(result.request_result==request_result&&requested.request_result==request_result&&requested.readback_stereo&&requested.layout_changed);
  assert(result.accepted==(request_result==kResultOk)&&result.restore_attempted&&result.restored&&restored.restored&&plugin.calls==2);
  assert(plugin.current==SpeakerArr::kMono&&plugin.terminate()==kResultOk);
 }
 {
  HostApplication host;MultiOutput plugin;assert(plugin.initialize(&host)==kResultOk);
  BusLayout layout;layout.read(plugin,plugin,false);assert(layout.size==34&&layout.counts[1]==32);
  layout.negotiate(plugin);layout.activate(plugin,true);
  for(bool enabled:plugin.enabled)assert(enabled);
  std::array<float,64> samples{};float* channels[64];for(size_t i=0;i<64;++i)channels[i]=&samples[i];float* inactive[]={nullptr,nullptr};
  std::array<AudioBusBuffers,32> buffers{};layout.map_outputs(buffers,channels,inactive);
  assert(buffers[0].channelBuffers32==channels&&buffers[0].numChannels==2);
  for(size_t i=1;i<32;++i)assert(buffers[i].numChannels==2&&buffers[i].channelBuffers32==channels+2*i&&buffers[i].silenceFlags==0);
  ProcessData data{};data.numOutputs=32;data.numSamples=1;data.outputs=buffers.data();
  assert(plugin.process(data)==kResultOk);for(size_t ch=0;ch<64;++ch)assert(samples[ch]==float(ch));
  std::vector<uint8_t> contract(28+32*layout.size);ap1::put(contract.data()+20,1,4);ap1::put(contract.data()+24,layout.size,4);
  std::array<int,4> indices{};
  for(size_t i=0;i<layout.size;++i){auto& b=layout.buses[i];auto* p=contract.data()+28+32*i;
   ap1::put(p,b.info.mediaType,4);ap1::put(p+4,b.info.direction,4);ap1::put(p+8,indices[b.info.mediaType*2+b.info.direction]++,4);
   ap1::put(p+12,b.effective_channels,4);ap1::put(p+16,b.info.busType,4);ap1::put(p+20,b.active?1:0,4);ap1::put(p+24,b.arrangement,8);
  }
  layout.contract(contract);ap1::put(contract.data()+28+32*31+20,0,4);
  layout.contract(contract);layout.map_outputs(buffers,channels,inactive);assert(buffers[31].channelBuffers32==inactive);
  ap1::put(contract.data()+28+32*31+20,2,4);
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
