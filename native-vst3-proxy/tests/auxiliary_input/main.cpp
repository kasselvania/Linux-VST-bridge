// Actual Processor/SDK bus publication and host default-activation sequence.
#include "processor.h"
#include "ap4_backend.h"
#include "ap10_backend.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <cassert>
#include <cstdio>
using namespace Steinberg;
using namespace Steinberg::Vst;
static unsigned setups=0,activations=0,closes=0;
extern "C" {
uint32_t __wrap_ap9_open(const uint8_t*,uint64_t*h){*h=1;return 0;}
uint32_t __wrap_ap5_report_path(uint64_t,uint8_t*p,uint32_t n){if(n)*p=0;return 0;}
uint32_t __wrap_ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*p,uint32_t n,uint32_t,uint32_t*t){
 assert(n==68&&p[0]==2); // count + two 32-byte bus records
 assert(p[4+20]==0&&p[4+32+20]==1); // auxiliary inactive, main active
 ++setups;t[0]=512;t[1]=t[2]=0;return 0;
}
uint32_t __wrap_ap4_activate(uint64_t,uint32_t,uint32_t){++activations;return 0;}
uint32_t __wrap_ap4_deactivate(uint64_t){return 0;}
uint32_t __wrap_ap3_close(uint64_t){++closes;return 0;}
}
int main(){
 HostApplication host;auto*p=new AP2::Processor;
 assert(p->initialize(&host)==kResultOk);
 assert(p->getBusCount(kAudio,kInput)==1);
 BusInfo aux{},out{};
 assert(p->getBusInfo(kAudio,kInput,0,aux)==kResultOk);
 assert(aux.busType==kAux&&aux.channelCount==2);
 assert(!(aux.flags&BusInfo::kDefaultActive));
 assert(p->getBusInfo(kAudio,kOutput,0,out)==kResultOk);
 assert(out.flags&BusInfo::kDefaultActive);
 assert(p->activateBus(kAudio,kInput,0,true)==kResultFalse);
 assert(p->activateBus(kAudio,kInput,0,false)==kResultOk);
 assert(p->activateBus(kAudio,kOutput,0,true)==kResultOk);
 ProcessSetup setup{kRealtime,kSample32,512,48000};
 assert(p->setupProcessing(setup)==kResultOk);
 assert(p->setActive(true)==kResultOk&&setups==2&&activations==1);
 assert(p->setActive(false)==kResultOk);
 assert(p->terminate()==kResultOk);p->release();assert(closes==1);
 std::puts("AP18 auxiliary input default activation PASS");
}
