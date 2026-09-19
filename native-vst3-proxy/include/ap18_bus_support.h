#pragma once
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <cstdint>
namespace AP18Buses {
inline constexpr int max_audio_inputs=8,max_audio_outputs=32,max_event_buses=8;
inline constexpr int max_buses=max_audio_inputs+max_audio_outputs+2*max_event_buses;
constexpr int capacity(int media,int direction) {
 return media==0?(direction==0?max_audio_inputs:max_audio_outputs):max_event_buses;
}
// The existing stereo transport has one input lane. It can carry the declared
// main input or an auxiliary input when that is the sole audio input. A main
// plus sidechain layout still needs another lane and remains unsupported.
// Every stereo output retains its SDK identity and its own planar transport lane.
constexpr bool supported(int media,int direction,int index,int type,int audio_inputs,int channels,uint64_t arrangement) {
 using namespace Steinberg::Vst;
 if(media==kAudio&&direction==kOutput)
  return index>=0&&index<max_audio_outputs&&channels==2&&arrangement==SpeakerArr::kStereo;
 return type==kMain || (media==kAudio&&direction==kInput&&index==0&&
                         type==kAux&&audio_inputs==1&&channels==2&&arrangement==SpeakerArr::kStereo);
}
}
