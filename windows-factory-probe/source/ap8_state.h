#pragma once
#include "ap1_protocol.h"
#include "../../vst-state/stream.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include <unordered_set>
namespace linux_vst_bridge::wf0 {
// Payload v2 (flags bit 1): 16-byte ID/availability/value records. Legacy
// 12-byte records remain readable. Unavailable records have no numeric value
// (canonical zero bytes), never an instruction to write zero to a plug-in.
struct SaveRefusal : std::runtime_error {
 uint32_t stage; Steinberg::tresult result;
 SaveRefusal(uint32_t s,Steinberg::tresult r):std::runtime_error("vendor declined state capture"),stage(s),result(r){}
};
inline bool ordinary_refusal(Steinberg::tresult r){return r==Steinberg::kResultFalse||r==Steinberg::kNotImplemented;}
inline void capture_result(Steinberg::tresult r,LVBState::Stream&s,uint32_t stage,bool restoring){
 ap1::require(!s.failed&&s.quiescent(),"state stream bounds/lifetime");
 if(r!=Steinberg::kResultOk){
  if(!restoring&&ordinary_refusal(r))throw SaveRefusal(stage,r);
  throw std::runtime_error("unsafe state SDK result");
 }
}
// Fresh initialization can expose the real editor without persistence. A
// successful stream still synchronizes a separate controller exactly once.
inline bool synchronize_initial(Steinberg::Vst::IEditController&controller,bool separate,Steinberg::tresult result,LVBState::Stream&state){
 using namespace Steinberg;
 ap1::require(!state.failed&&state.quiescent(),"initial state stream bounds/lifetime");
 if(result!=kResultOk){ap1::require(ordinary_refusal(result),"initial state SDK failure");return false;}
 if(separate){state.position=0;auto r=controller.setComponentState(&state);ap1::require(r==kResultOk&&!state.failed&&state.quiescent(),"initial controller synchronization failed");}
 return true;
}
struct ReadbackStatus {uint32_t unavailable=0,first_id=0;uint64_t first_bits=0;};
inline std::vector<uint8_t> commercial_state(Steinberg::Vst::IComponent&component,
 Steinberg::Vst::IEditController&controller,bool separate,const std::vector<uint8_t>*restore,ReadbackStatus*status=nullptr){
 using namespace Steinberg;using namespace ap1;
 auto checked=[](tresult r,LVBState::Stream&s){require(r==kResultOk&&!s.failed&&s.quiescent(),"commercial state SDK call/stream");};
 if(restore){const auto&p=*restore;require(p.size()>=16,"commercial state header");auto a=get(p.data(),4),b=get(p.data()+4,4),n=get(p.data()+8,4),flags=get(p.data()+12,4);
  auto width=(flags&2)?16u:12u;
  require(flags<=3&&n<=8192&&a+b+16+width*n==p.size()&&((flags&1)||b==0),"commercial state extent");
  std::unordered_set<uint32_t> ids;
  for(size_t i=16+a+b;i<p.size();i+=width){auto q=p.data()+i;auto valid=width==16?get(q+4,4):1;double value;std::memcpy(&value,q+width-8,8);
   require(ids.insert(uint32_t(get(q,4))).second&&valid<=1&&(valid?(std::isfinite(value)&&value>=0&&value<=1):get(q+8,8)==0),"state parameter value/identity");}
  LVBState::Stream c({p.begin()+16,p.begin()+16+a});checked(component.setState(&c),c);
  if(separate){c.position=0;checked(controller.setComponentState(&c),c);}
  if(flags&1){LVBState::Stream v({p.begin()+16+a,p.begin()+16+a+b});checked(controller.setState(&v),v);}
 }
 // Capture is read-only. Reapplying component state here makes Serum emit
 // kParamValuesChanged; Bitwig then captures state again, creating a refresh
 // loop. Owner-thread automation is already drained before this barrier.
 // Only an actual restore synchronizes the controller above.
 LVBState::Stream c;capture_result(component.getState(&c),c,1,restore!=nullptr);
 LVBState::Stream v;auto result=controller.getState(&v);
 bool supported=result==kResultOk;
 require(!v.failed&&v.quiescent(),"controller state stream bounds/lifetime");
 if(!supported&&!(result==kNotImplemented&&v.bytes.empty()))capture_result(result,v,2,restore!=nullptr);
 int n=controller.getParameterCount();require(n>=0&&n<=8192,"controller parameter count");
 size_t length=16+c.bytes.size()+v.bytes.size()+size_t(n)*16;require(length<=LVBState::payloadLimit,"complete commercial state bound");
 std::vector<uint8_t> out(length);put(out.data(),c.bytes.size(),4);put(out.data()+4,v.bytes.size(),4);put(out.data()+8,uint32_t(n),4);put(out.data()+12,(supported?1:0)|2,4);
 std::copy(c.bytes.begin(),c.bytes.end(),out.begin()+16);std::copy(v.bytes.begin(),v.bytes.end(),out.begin()+16+c.bytes.size());
 auto*p=out.data()+16+c.bytes.size()+v.bytes.size();
 for(int i=0;i<n;++i){
  Steinberg::Vst::ParameterInfo info{};require(controller.getParameterInfo(i,info)==kResultOk,"state parameter metadata");
  auto value=controller.getParamNormalized(info.id);
  const bool available=std::isfinite(value)&&value>=0&&value<=1;
  if(!available&&status){if(status->unavailable++==0){status->first_id=info.id;std::memcpy(&status->first_bits,&value,8);}}
  put(p,info.id,4);put(p+4,available?1:0,4);
  if(available)std::memcpy(p+8,&value,8);
  p+=16;
 }
 return out;
}
}
