#include "bus_census.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <cassert>
#include <vector>
#include <string>
using namespace Steinberg;using namespace Steinberg::Vst;using namespace linux_vst_bridge::wf0;
struct Plugin:AudioEffect {
 int initial=0,operational=0;bool negotiated=false;int active=0,processing=0,process_calls=0;
 tresult PLUGIN_API initialize(FUnknown* h)override{auto r=AudioEffect::initialize(h);addAudioInput(u"Sidechain",SpeakerArr::kStereo,kAux);addAudioOutput(u"Out",SpeakerArr::kStereo);addEventInput(u"Midi In",16);addEventOutput(u"Midi Out",initial);return r;}
 tresult PLUGIN_API getBusInfo(MediaType m,BusDirection d,int32 i,BusInfo& b)override{auto r=AudioEffect::getBusInfo(m,d,i,b);if(r==kResultOk&&m==kEvent&&d==kOutput)b.channelCount=negotiated?operational:initial;return r;}
 tresult PLUGIN_API setBusArrangements(SpeakerArrangement* in,int32 ni,SpeakerArrangement* out,int32 no)override{assert(ni==1&&no==1&&in[0]==SpeakerArr::kStereo&&out[0]==SpeakerArr::kStereo);negotiated=true;return kResultOk;}
 tresult PLUGIN_API setActive(TBool on)override{active+=on?1:-1;return kResultOk;}
 tresult PLUGIN_API setProcessing(TBool on)override{processing+=on?1:-1;return kResultOk;}
 tresult PLUGIN_API process(ProcessData&)override{++process_calls;assert(false);return kResultFalse;}
};
int main(){
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
