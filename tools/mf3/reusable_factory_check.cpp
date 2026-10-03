// Independent SDK consumer: two identical engine images with different data.
#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include <dlfcn.h>
#include <cstring>
#include <iostream>
#include <string>
#include <stdexcept>
using namespace Steinberg;
struct Module {
 void* image=nullptr;IPluginFactory* factory=nullptr;bool(*exit)()=nullptr;
 explicit Module(const char* path) {
  image=dlopen(path,RTLD_NOW|RTLD_LOCAL);if(!image)throw std::runtime_error(dlerror());
  auto entry=reinterpret_cast<bool(*)(void*)>(dlsym(image,"ModuleEntry"));
  exit=reinterpret_cast<bool(*)()>(dlsym(image,"ModuleExit"));
  auto get=reinterpret_cast<IPluginFactory*(*)()>(dlsym(image,"GetPluginFactory"));
  if(!entry||!exit||!get||!entry(image))throw std::runtime_error("module entry");
  factory=get();
 }
 ~Module(){if(factory)factory->release();if(exit)exit();if(image)dlclose(image);}
};
std::string hex(const TUID id){std::string out;for(unsigned char b:std::string(id,16)){out+="0123456789abcdef"[b>>4];out+="0123456789abcdef"[b&15];}return out;}
void check(Module& m,const char* expected,const char* name,double value){
 if(!m.factory||m.factory->countClasses()!=2)throw std::runtime_error("class count");
 PClassInfo processor{},controller{};
 if(m.factory->getClassInfo(0,&processor)||m.factory->getClassInfo(1,&controller)||
    hex(processor.cid)!=expected||std::strcmp(processor.name,name))throw std::runtime_error("metadata identity");
 Vst::IEditController* edit=nullptr;
 if(m.factory->createInstance(controller.cid,Vst::IEditController::iid.toTUID(),reinterpret_cast<void**>(&edit))||!edit)throw std::runtime_error("controller creation");
 if(edit->initialize(nullptr)||edit->getParameterCount()!=1||edit->getParamNormalized(7)!=value)throw std::runtime_error("parameter projection");
 Vst::ParameterInfo parameter{};
 if(edit->getParameterInfo(0,parameter)||parameter.id!=7||parameter.title[0]!='L')throw std::runtime_error("parameter metadata");
 if(edit->terminate())throw std::runtime_error("controller termination");edit->release();
 Vst::IComponent* component=nullptr;
 if(m.factory->createInstance(processor.cid,Vst::IComponent::iid.toTUID(),reinterpret_cast<void**>(&component))||!component)throw std::runtime_error("component creation");
 TUID cid{};if(component->getControllerClassId(cid)||std::memcmp(cid,controller.cid,16))throw std::runtime_error("controller association");
 if(component->initialize(nullptr))throw std::runtime_error("component initialization");
 const bool effect=std::strcmp(name,"Unseen Effect")==0;
 if(component->getBusCount(Vst::kAudio,Vst::kInput)!=(effect?1:0)||
    component->getBusCount(Vst::kAudio,Vst::kOutput)!=1)throw std::runtime_error("bus counts");
 Vst::BusInfo bus{};
 if(component->getBusInfo(Vst::kAudio,Vst::kOutput,0,bus)||bus.channelCount!=2||
    bus.busType!=Vst::kMain||bus.name[0]!='O')throw std::runtime_error("bus metadata");
 if(component->terminate())throw std::runtime_error("component termination");
 component->release();
}
int main(int argc,char**argv){try{
 if(argc==2){Module invalid(argv[1]);return invalid.factory?2:0;}
 if(argc!=6)return 2;
 Module first(argv[1]),second(argv[3]);
 check(first,argv[2],"Unseen Instrument",std::stod(argv[5]));
 check(second,argv[4],"Unseen Effect",0.6);
 // Reading the first factory again catches accidental cross-module globals.
 check(first,argv[2],"Unseen Instrument",std::stod(argv[5]));
 std::cout<<"Independent factories, buses, controller values and stable component/controller pairing passed\n";return 0;
 }catch(const std::exception&e){std::cerr<<e.what()<<'\n';return 1;}}
