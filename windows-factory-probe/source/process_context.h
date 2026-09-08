#pragma once
#include "ap1_protocol.h"
#include "pluginterfaces/vst/ivstprocesscontext.h"
namespace linux_vst_bridge::wf0 {
inline bool decode_context(const uint8_t* b,Steinberg::Vst::ProcessContext& c,double rate){
 using namespace ap1;auto present=get(b,4);require(present<=1&&get(b+92,4)==0,"context presence/reserved");c={};if(!present)return false;
 c.state=uint32_t(get(b+4,4));require((c.state&~0x2bf0eu)==0,"context flags");
 auto real=[&](int o){double v;std::memcpy(&v,b+o,8);return v;};
 c.sampleRate=real(8);require(c.sampleRate==rate,"context sample rate differs");
 c.projectTimeSamples=int64_t(get(b+16,8));c.systemTime=int64_t(get(b+24,8));c.continousTimeSamples=int64_t(get(b+32,8));
 c.projectTimeMusic=real(40);c.barPositionMusic=real(48);c.cycleStartMusic=real(56);c.cycleEndMusic=real(64);c.tempo=real(72);
 c.timeSigNumerator=int32_t(get(b+80,4));c.timeSigDenominator=int32_t(get(b+84,4));c.samplesToNextClock=int32_t(get(b+88,4));
 require(!(c.state&0x200)||std::isfinite(c.projectTimeMusic),"context music");require(!(c.state&0x800)||std::isfinite(c.barPositionMusic),"context bar");
 require(!(c.state&0x1000)||(std::isfinite(c.cycleStartMusic)&&std::isfinite(c.cycleEndMusic)&&c.cycleEndMusic>c.cycleStartMusic),"context cycle");
 require(!(c.state&0x400)||(std::isfinite(c.tempo)&&c.tempo>0),"context tempo");require(!(c.state&0x2000)||(c.timeSigNumerator>0&&c.timeSigDenominator>0),"context signature");return true;
}
}
