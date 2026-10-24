#pragma once
#include "pluginterfaces/vst/ivsthostapplication.h"
#include <array>
#include <cstdint>
namespace LVBHost {
// SDK edge only. Rust validates encoding/display bounds; no inferred DAW name.
struct Name {
  std::array<uint16_t,128> words{};
  uint32_t length=0;
  void capture(Steinberg::FUnknown* context) {
    words.fill(0); length=0;
    if(!context)return;
    Steinberg::Vst::IHostApplication* host=nullptr;
    if(context->queryInterface(Steinberg::Vst::IHostApplication::iid,
                              reinterpret_cast<void**>(&host))!=Steinberg::kResultOk || !host)return;
    Steinberg::Vst::String128 name{};
    auto result=host->getName(name);
    host->release();
    if(result!=Steinberg::kResultOk)return;
    uint32_t count=0;
    while(count<128 && name[count])++count;
    if(!count || count==128)return;
    for(uint32_t i=0;i<count;++i)words[i]=static_cast<uint16_t>(name[i]);
    length=count;
  }
};
}
