#pragma once
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <cstdint>
namespace AP18Buses {
// The existing stereo transport has one input lane. It can carry the declared
// main input or an auxiliary input when that is the sole audio input. A main
// plus sidechain layout still needs another lane and remains unsupported.
constexpr bool supported(int media,int direction,int index,int type,int audio_inputs,int channels,uint64_t arrangement) {
 using namespace Steinberg::Vst;
 return type==kMain || (media==kAudio&&direction==kInput&&index==0&&
                         type==kAux&&audio_inputs==1&&channels==2&&arrangement==SpeakerArr::kStereo);
}
}
