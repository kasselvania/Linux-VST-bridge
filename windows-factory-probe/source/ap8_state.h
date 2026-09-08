#pragma once
#include "ap1_protocol.h"
#include "../../vst-state/stream.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include <iomanip>
#include <sstream>
namespace linux_vst_bridge::wf0 {
// Version 5 State payload: bounded component bytes, optional controller bytes,
// and actual controller readback. Values are metadata, never a restore recipe.
inline std::vector<uint8_t> commercial_state(Steinberg::Vst::IComponent&component,
 Steinberg::Vst::IEditController&controller,bool separate,const std::vector<uint8_t>*restore){
 using namespace Steinberg;using namespace ap1;
 auto checked=[](tresult r,LVBState::Stream&s){require(r==kResultOk&&!s.failed&&s.quiescent(),"commercial state SDK call/stream");};
 if(restore){const auto&p=*restore;require(p.size()>=16,"commercial state header");auto a=get(p.data(),4),b=get(p.data()+4,4),n=get(p.data()+8,4),flags=get(p.data()+12,4);
  require(flags<=1&&n<=8192&&a+b+16+12*n==p.size()&&(flags==1||b==0),"commercial state extent");
  LVBState::Stream c({p.begin()+16,p.begin()+16+a});checked(component.setState(&c),c);
  if(separate){c.position=0;checked(controller.setComponentState(&c),c);}
  if(flags){LVBState::Stream v({p.begin()+16+a,p.begin()+16+a+b});checked(controller.setState(&v),v);}
 }
 // Capture is read-only. Reapplying component state here makes Serum emit
 // kParamValuesChanged; Bitwig then captures state again, creating a refresh
 // loop. Owner-thread automation is already drained before this barrier.
 // Only an actual restore synchronizes the controller above.
 LVBState::Stream c;checked(component.getState(&c),c);
 LVBState::Stream v;auto result=controller.getState(&v);
 bool supported=result==kResultOk;
 require((supported||result==kNotImplemented||result==kResultFalse)&&!v.failed&&v.quiescent(),"controller getState failure");
 require(supported||v.bytes.empty(),"unsupported controller state wrote data");
 int n=controller.getParameterCount();require(n>=0&&n<=8192,"controller parameter count");
 size_t length=16+c.bytes.size()+v.bytes.size()+size_t(n)*12;require(length<=LVBState::payloadLimit,"complete commercial state bound");
 std::vector<uint8_t> out(length);put(out.data(),c.bytes.size(),4);put(out.data()+4,v.bytes.size(),4);put(out.data()+8,uint32_t(n),4);put(out.data()+12,supported?1:0,4);
 std::copy(c.bytes.begin(),c.bytes.end(),out.begin()+16);std::copy(v.bytes.begin(),v.bytes.end(),out.begin()+16+c.bytes.size());
 auto*p=out.data()+16+c.bytes.size()+v.bytes.size();
 for(int i=0;i<n;++i){
  Steinberg::Vst::ParameterInfo info{};require(controller.getParameterInfo(i,info)==kResultOk,"state parameter metadata");
  auto value=controller.getParamNormalized(info.id);
  if(!std::isfinite(value)||value<0||value>1){
   // This is the owner-thread state barrier, not the audio callback. Retain the
   // exact offending scalar rather than hiding it behind a correlation error.
   std::ostringstream detail;
   detail<<"state parameter readback: id="<<info.id<<" value="<<std::setprecision(17)<<value;
   throw std::runtime_error(detail.str());
  }
  put(p,info.id,4);std::memcpy(p+4,&value,8);p+=12;
 }
 return out;
}
}
