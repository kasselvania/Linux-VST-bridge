// Windows half of LC1: real MappedSession/Timeline and SDK processing owner.
// The other end is queued::lc1_tests, not a second implementation of the client.
#include "mapped_processing.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <cassert>
#include <filesystem>
#include <iostream>
using namespace Steinberg;using namespace Steinberg::Vst;
using namespace linux_vst_bridge::wf0;
struct Fixture final:AudioEffect {
 unsigned blocks=0,starts=0,stops=0;
 tresult PLUGIN_API initialize(FUnknown* h) override {auto r=AudioEffect::initialize(h);addAudioInput(u"In",SpeakerArr::kStereo);addAudioOutput(u"Out",SpeakerArr::kStereo);return r;}
 tresult PLUGIN_API setProcessing(TBool active) override {if(active)++starts;else ++stops;return kResultOk;}
 tresult PLUGIN_API process(ProcessData& d) override {
  assert(d.numSamples==256&&d.numInputs==1&&d.numOutputs==1);
  for(int c=0;c<2;++c)for(int i=0;i<d.numSamples;++i)d.outputs[0].channelBuffers32[c][i]=d.inputs[0].channelBuffers32[c][i]*.5f;
  ++blocks;return kResultOk;
 }
};
int wmain(int argc,wchar_t** argv){
 if(argc!=3){std::cerr<<"LC1 requires the native test's directory and session\n";return 2;}
 try{
  HostApplication host;Fixture plugin;assert(plugin.initialize(&host)==kResultOk);
  EventWriter events(1048576);HostCallbackSink callbacks(&events,GetCurrentThreadId());
  std::wstring wid(argv[2]);std::string id(wid.begin(),wid.end());
  MappedSession session(argv[1],id,events,true,true,true,true,true);
  session.lc1_seed();session.ready();
  auto result=run_offline_processing(plugin,plugin,callbacks,events,&session);
  assert(result.success&&result.quiescent&&result.retirement_ready);
  assert(plugin.blocks==4&&plugin.starts==2&&plugin.stops==2);
  session.finish(true);assert(plugin.terminate()==kResultOk);
  return 0;
 }catch(const std::exception& e){std::cerr<<"LC1 Windows failure: "<<e.what()<<'\n';return 1;}
}
