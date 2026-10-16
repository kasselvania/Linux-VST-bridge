#pragma once
#include <cstdint>
// Synthetic SDK instrument descriptor with one event input; never a published
// commercial identity. 7, 8 and 9 stand for a vendor's hidden MIDI controller
// parameters (modulation wheel, pitch bend, aftertouch), as Kontakt lists them.
namespace AP8 {
inline constexpr char vendor[]="Bridge test fixture",version[]="1.0",subcategories[]="Instrument|Synth";
inline constexpr char class_name[]="AP11 MIDI mapping fixture";
struct Parameter {uint32_t id;const char16_t* title;const char16_t* units;int32_t steps,flags;double initial;};
struct Bus {uint32_t media,direction,index,channels,type,flags;uint64_t arrangement;const char16_t* name;};
inline constexpr bool effect=false;
inline constexpr Bus buses[]={
 {0,1,0,2,0,1,3,u"Output"},
 {1,0,0,16,0,1,0,u"Event Input"}
};
inline constexpr uint8_t identity[48]={};
inline constexpr Parameter parameters[]={
 {0,u"Gain",u"",0,1,0.5},
 {7,u"Modulation Wheel(MSB)",u"",0,0,0},
 {8,u"Pitchbend",u"",0,0,0.5},
 {9,u"Aftertouch",u"",0,0,0}
};
#define AP8_PROCESSOR_UID 0x41503131,0x4d494449,0x4d415020,0x00000001
#define AP8_CONTROLLER_UID 0x41503131,0x4d494449,0x4d415020,0x00000002
}
