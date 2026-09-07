#pragma once
#include "ap10_backend.h"
#include "pluginterfaces/gui/iplugview.h"
#include "ap8_descriptor.h"
#include "../../vst-state/stream.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
namespace AP8 {
class Controller final:public Steinberg::Vst::EditController {
 struct Timer final:Steinberg::Linux::ITimerHandler {
  Controller* parent;std::atomic<Steinberg::uint32> refs{1};explicit Timer(Controller*p):parent(p){}
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id,void**out)override{if(!out)return Steinberg::kInvalidArgument;*out=nullptr;if(Steinberg::FUnknownPrivate::iidEqual(id,Steinberg::Linux::ITimerHandler::iid)||Steinberg::FUnknownPrivate::iidEqual(id,Steinberg::FUnknown::iid)){*out=static_cast<Steinberg::Linux::ITimerHandler*>(this);addRef();return Steinberg::kResultOk;}return Steinberg::kNoInterface;}
  Steinberg::uint32 PLUGIN_API addRef()override{return ++refs;}
  Steinberg::uint32 PLUGIN_API release()override{auto n=--refs;if(!n)delete this;return n;}
  void PLUGIN_API onTimer()override{if(parent&&parent->connected_)parent->request("AP10.poll");}
 };
public:
 ~Controller()override{stopTimer();}
 Steinberg::tresult PLUGIN_API terminate()override{stopTimer();return EditController::terminate();}
 static Steinberg::FUnknown* create(void*){try{return static_cast<Steinberg::Vst::IEditController*>(new Controller);}catch(...){return nullptr;}}
 Steinberg::tresult PLUGIN_API initialize(Steinberg::FUnknown*host)override {
  auto r=EditController::initialize(host);if(r!=Steinberg::kResultOk)return r;owner_=std::this_thread::get_id();
  if(host&&host->queryInterface(Steinberg::Linux::IRunLoop::iid,reinterpret_cast<void**>(&loop_))==Steinberg::kResultOk&&loop_){
   timer_=new Timer(this);if(loop_->registerTimer(timer_,20)!=Steinberg::kResultOk){timer_->release();timer_=nullptr;loop_->release();loop_=nullptr;}
  }
  for(const auto&p:AP8::parameters)parameters.addParameter(p.title,p.units,p.steps,p.initial,p.flags,p.id);
  return Steinberg::kResultOk;
 }
 Steinberg::tresult PLUGIN_API connect(Steinberg::Vst::IConnectionPoint*peer)override {
  auto r=EditController::connect(peer);if(r==Steinberg::kResultOk){connected_=true;
   auto*m=allocateMessage();if(m){m->setMessageID("AP10.capabilities");m->getAttributes()->setInt("notifications",timer_?1:0);sendMessage(m);m->release();}
   request();}return r;
 }
 Steinberg::tresult PLUGIN_API disconnect(Steinberg::Vst::IConnectionPoint*peer)override {connected_=false;return EditController::disconnect(peer);}
 Steinberg::tresult PLUGIN_API setComponentState(Steinberg::IBStream*s)override {
  if(owner_!=std::this_thread::get_id())return Steinberg::kResultFalse;
  try {std::vector<uint8_t>b;if(!LVBState::readEnvelope(s,b)||!apply(b))return Steinberg::kResultFalse;
   if(connected_)return request();
   return Steinberg::kResultOk;
  }catch(...){return Steinberg::kResultFalse;}
 }
 Steinberg::tresult PLUGIN_API notify(Steinberg::Vst::IMessage*m)override {
  if(!m||owner_!=std::this_thread::get_id())return Steinberg::kResultFalse;
  if(!std::strcmp(m->getMessageID(),"AP10.restart")){Steinberg::int64 flags=0;if(!componentHandler||m->getAttributes()->getInt("flags",flags)!=Steinberg::kResultOk||flags<=0||(flags&~10))return Steinberg::kResultFalse;return componentHandler->restartComponent(Steinberg::int32(flags));}
  if(std::strcmp(m->getMessageID(),"AP8.readback"))return EditController::notify(m);
  const void*p=nullptr;Steinberg::uint32 n=0;
  if(m->getAttributes()->getBinary("state",p,n)!=Steinberg::kResultOk||n>LVBState::payloadLimit+LVBState::overhead)return Steinberg::kResultFalse;
  try {const auto*b=static_cast<const uint8_t*>(p);return apply({b,b+n})?Steinberg::kResultOk:Steinberg::kResultFalse;}catch(...){return Steinberg::kResultFalse;}
 }
 // The vendor controller bytes are included with the complete component bundle.
 Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream*s)override {uint8_t b[8]={'L','V','B','C',2,0,0,0};return LVBState::transfer(s,b,8,true)?Steinberg::kResultOk:Steinberg::kResultFalse;}
 Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream*s)override {uint8_t b[8]{},expected[8]={'L','V','B','C',2,0,0,0};return LVBState::transfer(s,b,8,false)&&std::equal(b,b+8,expected)?Steinberg::kResultOk:Steinberg::kResultFalse;}
private:
 bool apply(const std::vector<uint8_t>&b){
  if(ap8_validate(identity,b.data(),static_cast<uint32_t>(b.size())))return false;
  auto read=[](const uint8_t*p){uint32_t n=0;for(unsigned i=0;i<4;++i)n|=uint32_t(p[i])<<(8*i);return n;};
  const auto*p=b.data()+104;auto n=read(p+8);if(n!=std::size(AP8::parameters))return false;
  p+=16+read(p)+read(p+4);
  for(uint32_t i=0;i<n;++i,p+=12){double value;std::memcpy(&value,p+4,8);auto id=read(p);if(!parameters.getParameter(id)||EditController::setParamNormalized(id,value)!=Steinberg::kResultOk)return false;}
  return true;
 }
 Steinberg::tresult request(const char*id="AP8.readback"){auto*m=allocateMessage();if(!m)return Steinberg::kResultFalse;m->setMessageID(id);auto r=sendMessage(m);m->release();return r;}
 void stopTimer(){if(timer_){timer_->parent=nullptr;if(loop_)loop_->unregisterTimer(timer_);timer_->release();timer_=nullptr;}if(loop_){loop_->release();loop_=nullptr;}}
 Steinberg::Linux::IRunLoop* loop_=nullptr;Timer* timer_=nullptr;
 std::thread::id owner_;bool connected_=false;
};
}
