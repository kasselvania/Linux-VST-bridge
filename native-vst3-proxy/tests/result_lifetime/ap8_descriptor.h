#pragma once
#include <cstdint>
// Synthetic SDK effect descriptor; never a published commercial identity.
namespace AP8 {
struct Parameter {uint32_t id;const char16_t* title;const char16_t* units;int32_t steps,flags;double initial;};
struct Bus {uint32_t media,direction,index,channels,type,flags;uint64_t arrangement;const char16_t* name;};
inline constexpr bool effect=true;
inline constexpr Bus buses[]={
 {0,0,0,2,0,1,3,u"Input"}, {0,1,0,2,0,1,3,u"Output"},
 {1,1,0,16,0,0,0,u"Returned events"}
};
inline constexpr uint8_t identity[48]={};
inline constexpr Parameter parameters[]={{0,u"Gain",u"",0,1,0.5}};
#define AP8_PROCESSOR_UID 0x41503130,0x4c494645,0x54494d45,0x00000001
#define AP8_CONTROLLER_UID 0x41503130,0x4c494645,0x54494d45,0x00000002
}
