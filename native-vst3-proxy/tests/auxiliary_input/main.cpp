// Real native Processor/SDK callback; transport boundary checks exact samples.
#include "processor.h"
#include "ap4_backend.h"
#include "ap10_backend.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include <algorithm>
#include <array>
#include <cassert>
#include <cstdio>
#include <cstring>
#include <string>
using namespace Steinberg;
using namespace Steinberg::Vst;
static unsigned setups=0,activations=0,closes=0,processes=0;
static bool input_enabled=true;
static std::array<float,32> expected_left{},expected_right{};
static uint64_t expected_silence=3;
extern "C" {
uint32_t __wrap_ap9_open(const uint8_t*,uint64_t*h){*h=1;return 0;}
uint32_t __wrap_ap5_report_path(uint64_t,uint8_t*p,uint32_t n){if(n)*p=0;return 0;}
uint32_t __wrap_ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*p,uint32_t n,uint32_t,uint32_t*t){
 assert(n==68&&p[0]==2);assert(p[4+16]==kAux);
 assert(p[4+20]==unsigned(input_enabled)&&p[4+32+20]==1);
 ++setups;t[0]=512;t[1]=t[2]=0;return 0;
}
uint32_t __wrap_ap4_activate(uint64_t,uint32_t,uint32_t){++activations;return 0;}
uint32_t __wrap_ap4_deactivate(uint64_t){return 0;}
uint32_t __wrap_ap3_transition(uint64_t,uint32_t){return 0;}
uint32_t __wrap_ap10_take_results(uint64_t,ap10_results_t*p){std::memset(p,0,sizeof(*p));return 0;}
uint32_t __wrap_ap13_process(uint64_t,uint32_t n,const ap8_event_t*e,uint32_t count,const ap10_context_t*,uint64_t silence,const float*l,const float*r,float*ol,float*orr,uint64_t*out,ap7_delivery_t*d,uint64_t entered){
 assert(entered&&count==2&&e[0].kind==0&&e[1].kind==2);
 assert(e[0].offset==(n?7u:0u)&&e[1].offset==(n?11u:0u));
 assert(n==0||n==32);
 if(n){assert(silence==expected_silence);for(unsigned i=0;i<n;++i){assert(l[i]==expected_left[i]);assert(r[i]==expected_right[i]);}}
 std::copy_n(l,n,ol);std::copy_n(r,n,orr);*out=silence;*d={};d->delivered_frames=n;++processes;return 0;
}
uint32_t __wrap_ap3_close(uint64_t){++closes;return 0;}
}
int main(){
 HostApplication host;auto*p=new AP2::Processor;assert(p->initialize(&host)==kResultOk);
 BusInfo aux{},output{};assert(p->getBusCount(kAudio,kInput)==1);
 assert(p->getBusInfo(kAudio,kInput,0,aux)==kResultOk);
 assert(aux.mediaType==kAudio&&aux.direction==kInput&&aux.busType==kAux&&aux.channelCount==2);
 assert(std::u16string(reinterpret_cast<const char16_t*>(aux.name))==u"Aux input");
 assert(aux.flags&BusInfo::kDefaultActive);SpeakerArrangement arrangement=0;
 assert(p->getBusArrangement(kInput,0,arrangement)==kResultOk&&arrangement==SpeakerArr::kStereo);
 assert(p->getBusInfo(kAudio,kOutput,0,output)==kResultOk&&output.busType==kMain);
 assert(p->activateBus(kAudio,kInput,0,true)==kResultOk);
 assert(p->activateBus(kAudio,kOutput,0,true)==kResultOk);
 ProcessSetup setup{kRealtime,kSample32,512,48000};assert(p->setupProcessing(setup)==kResultOk);
 assert(p->setActive(true)==kResultOk&&setups==2&&activations==1);assert(p->setProcessing(true)==kResultOk);
 // Guarded buffers with unused tails: no-source -> routed source -> source gone.
 std::array<float,36> left{},right{},ol{},orr{};left.fill(99);right.fill(99);ol.fill(99);orr.fill(99);
 float* in[]={left.data()+1,right.data()+1};float* out[]={ol.data()+1,orr.data()+1};
 AudioBusBuffers ib{},ob{};ib.numChannels=ob.numChannels=2;ib.channelBuffers32=in;ob.channelBuffers32=out;
 EventList notes;Event note{};note.type=Event::kNoteOnEvent;note.sampleOffset=7;note.noteOn={0,60,0,.5f,32,1};assert(notes.addEvent(note)==kResultOk);
 ParameterChanges parameters;int32 qi=0,pi=0;parameters.addParameterData(0,qi)->addPoint(11,.75,pi);
 ProcessData d{};d.processMode=kRealtime;d.symbolicSampleSize=kSample32;d.numSamples=32;d.numInputs=d.numOutputs=1;d.inputs=&ib;d.outputs=&ob;d.inputEvents=&notes;d.inputParameterChanges=&parameters;
 auto run=[&]{std::copy(expected_left.begin(),expected_left.end(),in[0]);std::copy(expected_right.begin(),expected_right.end(),in[1]);const auto l=left,r=right;assert(p->process(d)==kResultOk);assert(left==l&&right==r);for(size_t i=0;i<32;++i){assert(out[0][i]==expected_left[i]);assert(out[1][i]==expected_right[i]);}for(auto*b:{&ol,&orr}){assert((*b)[0]==99);for(size_t i=33;i<36;++i)assert((*b)[i]==99);}};
 ib.silenceFlags=3;run();
 for(unsigned i=0;i<32;++i){expected_left[i]=float(i+1)/64;expected_right[i]=-float(i+1)/128;}expected_silence=0;ib.silenceFlags=0;run();
 // Preserve the existing correction of false silence hints without dropping audio.
 ib.silenceFlags=3;run();
 expected_left.fill(0);expected_right.fill(0);expected_silence=3;run();
 auto before=processes;ib.channelBuffers32=nullptr;assert(p->process(d)!=kResultOk&&processes==before);ib.channelBuffers32=in;
 in[1]=nullptr;assert(p->process(d)!=kResultOk&&processes==before);in[1]=right.data()+1;
 ib.numChannels=1;assert(p->process(d)!=kResultOk&&processes==before);ib.numChannels=2;
 ib.silenceFlags=4;assert(p->process(d)!=kResultOk&&processes==before);ib.silenceFlags=3;
 in[1]=in[0];assert(p->process(d)!=kResultOk&&processes==before);in[1]=right.data()+1;
 out[0]=in[1];assert(p->process(d)!=kResultOk&&processes==before);out[0]=ol.data()+1;
 // Inactive SDK bus may omit all channel pointers; stale nonzero input is ignored.
 assert(p->setProcessing(false)==kResultOk);assert(p->setActive(false)==kResultOk);
 input_enabled=false;assert(p->activateBus(kAudio,kInput,0,false)==kResultOk);assert(p->setActive(true)==kResultOk);assert(p->setProcessing(true)==kResultOk);
 left.fill(.8f);right.fill(-.4f);ib.channelBuffers32=nullptr;assert(p->process(d)==kResultOk);
 for(unsigned i=0;i<32;++i)assert(out[0][i]==0&&out[1][i]==0);
 // Zero-frame event/parameter flush remains independent of input storage.
 notes.clear();note.sampleOffset=0;notes.addEvent(note);parameters.clearQueue();parameters.addParameterData(0,qi)->addPoint(0,.5,pi);
 d.numSamples=0;d.numInputs=d.numOutputs=0;d.inputs=d.outputs=nullptr;assert(p->process(d)==kResultOk);
 assert(p->setProcessing(false)==kResultOk);assert(p->setActive(false)==kResultOk);
 assert(p->terminate()==kResultOk);p->release();assert(closes==1);
 std::puts("AP18 native sole auxiliary samples/silence/guards/events PASS");
}
