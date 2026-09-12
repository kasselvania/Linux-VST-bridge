#pragma once
#include <cstdint>
namespace AP8 {
struct Parameter {uint32_t id;const char16_t* title;const char16_t* units;int32_t steps,flags;double initial;};
struct Bus {uint32_t media,direction,index,channels,type,flags;uint64_t arrangement;const char16_t* name;};
inline constexpr char vendor[]="SDK fixture",version[]="1",subcategories[]="Instrument|Synth",class_name[]="Auxiliary input instrument";
inline constexpr bool effect=false;
inline constexpr Bus buses[]={{0,0,0,2,1,1,3,u"Aux input"},{0,1,0,2,0,1,3,u"Main output"}};
inline constexpr uint8_t identity[48]={};
inline constexpr Parameter parameters[]={{0,u"Gain",u"",0,1,.5}};
#define AP8_PROCESSOR_UID 0x41503138,0,0,1
#define AP8_CONTROLLER_UID 0x41503138,0,0,2
}
