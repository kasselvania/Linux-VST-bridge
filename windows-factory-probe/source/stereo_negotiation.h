#pragma once
#include "ap1_protocol.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <array>
#include <cstring>

namespace linux_vst_bridge::wf0 {

struct StereoBusSnapshot {
 struct Bus {
  Steinberg::Vst::BusDirection direction=Steinberg::Vst::kInput;
  Steinberg::tresult info_result=Steinberg::kResultFalse;
  Steinberg::Vst::BusInfo info{};
  Steinberg::tresult arrangement_result=Steinberg::kResultFalse;
  Steinberg::Vst::SpeakerArrangement arrangement=0;
 };
 std::array<Steinberg::int32,2> counts{};
 std::array<Bus,2> buses{};

 static StereoBusSnapshot capture(Steinberg::Vst::IComponent& component,
                                  Steinberg::Vst::IAudioProcessor& processor) {
  using namespace Steinberg;using namespace Steinberg::Vst;
  StereoBusSnapshot row;
  for(int direction=0;direction<2;++direction){
   row.counts[size_t(direction)]=component.getBusCount(kAudio,direction);
   auto& bus=row.buses[size_t(direction)];bus.direction=BusDirection(direction);
   if(row.counts[size_t(direction)]==1){
    bus.info_result=component.getBusInfo(kAudio,direction,0,bus.info);
    bus.arrangement_result=processor.getBusArrangement(direction,0,bus.arrangement);
   }
  }
  return row;
 }

 bool exact_topology()const {
  using namespace Steinberg;using namespace Steinberg::Vst;
  if(counts[0]!=1||counts[1]!=1)return false;
  for(int direction=0;direction<2;++direction){const auto& bus=buses[size_t(direction)];
   if(bus.info_result!=kResultOk||bus.arrangement_result!=kResultOk||
      bus.info.mediaType!=kAudio||bus.info.direction!=direction)return false;
  }
  return true;
 }
 bool stereo_readback()const {
  if(!exact_topology())return false;
  for(const auto& bus:buses)if(bus.info.channelCount!=2||bus.arrangement!=Steinberg::Vst::SpeakerArr::kStereo)return false;
  return true;
 }
};

inline bool same_stereo_snapshot(const StereoBusSnapshot& left,const StereoBusSnapshot& right){
 if(left.counts!=right.counts)return false;
 for(size_t i=0;i<left.buses.size();++i){const auto& a=left.buses[i];const auto& b=right.buses[i];
  if(a.direction!=b.direction||a.info_result!=b.info_result||a.arrangement_result!=b.arrangement_result||a.arrangement!=b.arrangement)return false;
  if(a.info_result==Steinberg::kResultOk&&
     (a.info.mediaType!=b.info.mediaType||a.info.direction!=b.info.direction||a.info.channelCount!=b.info.channelCount||
      a.info.busType!=b.info.busType||a.info.flags!=b.info.flags||std::memcmp(a.info.name,b.info.name,sizeof(a.info.name))!=0))return false;
 }
 return true;
}

struct StereoNegotiationResult {
 Steinberg::tresult request_result=Steinberg::kResultFalse;
 bool readback_stereo=false;
 bool accepted=false;
 bool layout_changed=false;
 bool restore_attempted=false;
 Steinberg::tresult restore_result=Steinberg::kResultFalse;
 bool restored=false;
};

// Diagnostic only: request one exact stereo input/output pair while initialized
// and inactive. Even a false result can adapt a layout, so read back first and
// restore the exact original arrangement whenever any reported bus fact changes.
template<class EmitSnapshot,class EmitRequest,class EmitRestore,class Step>
StereoNegotiationResult run_stereo_negotiation(Steinberg::Vst::IComponent& component,
                                                Steinberg::Vst::IAudioProcessor& processor,
                                                EmitSnapshot emit_snapshot,EmitRequest emit_request,
                                                EmitRestore emit_restore,Step step){
 using namespace Steinberg;using namespace Steinberg::Vst;
 StereoNegotiationResult result;
 const auto before=StereoBusSnapshot::capture(component,processor);emit_snapshot("before",before);
 ap1::require(before.exact_topology(),"stereo diagnostic requires one exact audio input and output");
 SpeakerArrangement input=SpeakerArr::kStereo,output=SpeakerArr::kStereo;
 step("stereoSetBusArrangements");
 result.request_result=processor.setBusArrangements(&input,1,&output,1);
 const auto after=StereoBusSnapshot::capture(component,processor);emit_snapshot("after",after);
 result.readback_stereo=after.stereo_readback();
 result.accepted=result.request_result==kResultOk&&result.readback_stereo;
 result.layout_changed=!same_stereo_snapshot(before,after);
 emit_request(result);
 result.restore_attempted=result.layout_changed;
 StereoBusSnapshot restored=after;
 if(result.restore_attempted){
  input=before.buses[0].arrangement;output=before.buses[1].arrangement;
  step("stereoRestoreBusArrangements");
  result.restore_result=processor.setBusArrangements(&input,1,&output,1);
  restored=StereoBusSnapshot::capture(component,processor);emit_snapshot("restored",restored);
 }
 result.restored=same_stereo_snapshot(before,restored);
 emit_restore(result);
 ap1::require(result.restored,"stereo diagnostic original layout not restored");
 return result;
}

}
