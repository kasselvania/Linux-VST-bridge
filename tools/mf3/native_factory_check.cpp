// Source-owned native package check; no component, controller, audio or session.
#include "pluginterfaces/base/ipluginbase.h"
#include <dlfcn.h>
#include <iostream>
#include <string>
using namespace Steinberg;
int main(int argc,char**argv){
 if(argc!=4)return 2;
 void* image=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);if(!image){std::cerr<<dlerror();return 3;}
 auto entry=reinterpret_cast<bool(*)(void*)>(dlsym(image,"ModuleEntry"));
 auto exit=reinterpret_cast<bool(*)()>(dlsym(image,"ModuleExit"));
 auto get=reinterpret_cast<IPluginFactory*(*)()>(dlsym(image,"GetPluginFactory"));
 if(!entry||!exit||!get||!entry(image))return 4;
 auto* factory=get();if(!factory||factory->countClasses()!=2)return 5;
 bool processor=false,controller=false;
 for(int32 i=0;i<2;++i){PClassInfo info{};if(factory->getClassInfo(i,&info)!=kResultOk)return 6;
  std::string id;const char*hex="0123456789abcdef";
  for(unsigned char c:info.cid){id+=hex[c>>4];id+=hex[c&15];}
  processor|=id==argv[2]&&std::string(info.category)=="Audio Module Class";
  controller|=id==argv[3]&&std::string(info.category)=="Component Controller Class";
 }
 factory->release();if(!exit())return 7;dlclose(image);
 if(!processor||!controller)return 8;
 std::cout<<"Exact generated native processor/controller factory identities: passed\n";
}
