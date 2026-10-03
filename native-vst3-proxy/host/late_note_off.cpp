// Independent SDK consumer of an installed instrument. Never decodes vendor
// state or accesses the bridge protocol. Output stays in the private fixture.
#include "../../vst-state/stream.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/module.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include <array>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <thread>
#include <vector>
using namespace Steinberg;
using namespace Steinberg::Vst;
using Clock=std::chrono::steady_clock;
static void need(bool v,const char* why){if(!v)throw std::runtime_error(why);}
static void ok(tresult v,const char* why){need(v==kResultOk,why);}
int main(int argc,char** argv){try{
 need(argc==4||argc==5,"usage: late-note-off-host BUNDLE OUTPUT_PREFIX -1661|0 [BLOCKS]");
 const int offset=std::stoi(argv[3]);need(offset==-1661||offset==0,"declared release offset");
 constexpr int frames=512;
 const int count=argc==5?std::stoi(argv[4]):720;
 need(count>=720&&count<=112500,"capture duration outside 720..112500 blocks");
 const std::string prefix=argv[2];
 std::string error;auto module=VST3::Hosting::Module::create(argv[1],error);
 need(bool(module),"module load");auto host=owned(new HostApplication);
 module->getFactory().setHostContext(host);auto classes=module->getFactory().classInfos();
 need(classes.size()==2,"exact component/controller factory");
 auto component=module->getFactory().createInstance<IComponent>(classes[0].ID());
 need(bool(component),"component");ok(component->initialize(host),"initialize");
 FUnknownPtr<IAudioProcessor> processor(component);need(bool(processor),"processor");
 auto controller=module->getFactory().createInstance<IEditController>(classes[1].ID());
 need(bool(controller),"controller");ok(controller->initialize(host),"controller initialize");
 FUnknownPtr<IConnectionPoint> cp(component),cc(controller);need(cp&&cc,"connections");
 ok(cp->connect(cc),"component connect");ok(cc->connect(cp),"controller connect");
 struct Retirement {
  IComponent* component;IEditController* controller;IAudioProcessor* processor;
  IConnectionPoint* cp;IConnectionPoint* cc;bool pending=true;
  ~Retirement(){if(pending){processor->setProcessing(false);component->setActive(false);
   cc->disconnect(cp);cp->disconnect(cc);component->terminate();controller->terminate();}}
 } retirement{component,controller,processor,cp,cc};
 need(component->getBusCount(kAudio,kInput)==0&&component->getBusCount(kAudio,kOutput)==1&&
      component->getBusCount(kEvent,kInput)==1,"instrument buses");
 LVBState::Stream initial;ok(component->getState(&initial),"initial state");
 initial.position=0;ok(controller->setComponentState(&initial),"initial control state");
 SpeakerArrangement stereo=SpeakerArr::kStereo;
 ok(processor->setBusArrangements(nullptr,0,&stereo,1),"stereo output");
 ProcessSetup setup{kRealtime,kSample32,frames,48000.};
 ok(processor->setupProcessing(setup),"setup");
 ok(component->activateBus(kEvent,kInput,0,true),"notes active");
 ok(component->activateBus(kAudio,kOutput,0,true),"output active");
 ok(component->setActive(true),"activate");
 // The runner verifies selected bridge delay independently. The SDK reports
 // bridge plus vendor latency; a commercial instrument need not report 512.
 const auto reportedLatency=processor->getLatencySamples();
 std::vector<std::array<float,2>> samples(size_t(count*frames));
 struct Row{tresult result{};uint64_t started{},duration{};};std::vector<Row> rows(size_t(count),Row{});
 uint64_t audioBegin=0;
 std::exception_ptr failure;
 std::thread audio([&]{try{
  std::array<std::array<float,frames>,2> out{};float* planes[]{out[0].data(),out[1].data()};
  AudioBusBuffers output{};output.numChannels=2;output.channelBuffers32=planes;
  EventList notes(1);ProcessData data{};data.processMode=kRealtime;data.symbolicSampleSize=kSample32;
  data.numSamples=frames;data.numOutputs=1;data.outputs=&output;data.inputEvents=&notes;
  ok(processor->setProcessing(true),"start");auto begin=Clock::now();
  audioBegin=std::chrono::duration_cast<std::chrono::nanoseconds>(begin.time_since_epoch()).count();
  for(int b=0;b<count;++b){
   notes.clear();
   const int pattern=b%720;
   if(pattern==8||pattern==120||pattern==360||pattern==472){
    const bool on=pattern==8||pattern==360;const int pitch=pattern<360?60:67,id=pattern<360?42:43;
    Event e{};e.busIndex=0;e.sampleOffset=pattern==120?offset:0;
    e.type=on?Event::kNoteOnEvent:Event::kNoteOffEvent;
    if(on)e.noteOn={0,int16(pitch),0.f,.75f,0,id};else e.noteOff={0,int16(pitch),.25f,id,0.f};
    ok(notes.addEvent(e),"note input");
   }
   std::this_thread::sleep_until(begin+std::chrono::nanoseconds(uint64_t(b)*frames*1000000000ULL/48000));
   auto at=Clock::now();rows[size_t(b)].result=processor->process(data);
   rows[size_t(b)].duration=std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-at).count();
   rows[size_t(b)].started=std::chrono::duration_cast<std::chrono::nanoseconds>(at-begin).count();
   for(int i=0;i<frames;++i)samples[size_t(b*frames+i)]={out[0][size_t(i)],out[1][size_t(i)]};
  }
  ok(processor->setProcessing(false),"stop");
 }catch(...){failure=std::current_exception();}});audio.join();
 if(failure)std::rethrow_exception(failure);
 LVBState::Stream after;const auto state=component->getState(&after);
 const auto inactive=component->setActive(false);
 const auto disconnectController=cc->disconnect(cp),disconnectComponent=cp->disconnect(cc);
 const auto retired=component->terminate(),controllerRetired=controller->terminate();
 retirement.pending=false;cc=nullptr;cp=nullptr;processor=nullptr;component=nullptr;controller=nullptr;
 ok(disconnectController,"controller disconnect");ok(disconnectComponent,"component disconnect");
 ok(controllerRetired,"controller terminate");
 std::ofstream raw(prefix+".f32le",std::ios::binary);
 raw.write(reinterpret_cast<const char*>(samples.data()),std::streamsize(samples.size()*sizeof(samples[0])));
 raw.flush();need(bool(raw),"captured output write");
 uint64_t rejected=0,nonfinite=0;
 for(const auto&r:rows)rejected+=r.result!=kResultOk;
 for(const auto&s:samples)for(float v:s)nonfinite+=!std::isfinite(v);
 std::cout<<"{\"event\":\"late_note_off\",\"offset\":"<<offset<<",\"frames\":"<<frames
  <<",\"blocks\":"<<count<<",\"reported_latency_samples\":"<<reportedLatency<<",\"native_processor_id\":\""<<classes[0].ID().toString()
  <<"\",\"native_controller_id\":\""<<classes[1].ID().toString()<<"\",\"rejected_callbacks\":"<<rejected
  <<",\"nonfinite_samples\":"<<nonfinite<<",\"state_result\":"<<state<<",\"state_bytes\":"<<after.bytes.size()
  <<",\"deactivate_result\":"<<inactive<<",\"terminate_result\":"<<retired
  <<",\"audio_begin_monotonic_ns\":"<<audioBegin<<",\"timing\":[";
 for(int b=0;b<count;++b){const auto&r=rows[size_t(b)];if(b)std::cout<<',';
  std::cout<<'['<<r.result<<','<<r.started<<','<<r.duration<<']';}
 std::cout<<"]}"<<std::endl;
 return rejected||nonfinite||state!=kResultOk||inactive!=kResultOk||retired!=kResultOk?1:0;
}catch(const std::exception&e){std::cerr<<e.what()<<'\n';return 2;}}
