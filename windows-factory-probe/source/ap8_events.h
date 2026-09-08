#pragma once
#include "ap1_protocol.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
// Fixed storage owned before activation. SDK pointers never escape process().
namespace linux_vst_bridge::wf0 {
constexpr size_t event_capacity=256;
struct InputEvent { uint32_t offset=0,kind=0,id=0;int16_t channel=0,pitch=0;double value=0;float tuning=0; };
inline size_t decode_events(const std::vector<uint8_t>& p,std::array<InputEvent,event_capacity>& out,uint32_t frames){
 using namespace ap1;
 require(p.size()>=56,"event request header");auto n=get(p.data()+48,4);
 require(n<=event_capacity&&get(p.data()+52,4)==0&&p.size()==56+32*n,"event request extent/reserved");
 for(size_t i=0;i<n;++i){const auto*b=p.data()+56+32*i;auto&e=out[i];
  e.offset=uint32_t(get(b,4));e.kind=uint32_t(get(b+4,4));e.id=uint32_t(get(b+8,4));
  e.channel=int16_t(get(b+12,2));e.pitch=int16_t(get(b+14,2));std::memcpy(&e.value,b+16,8);std::memcpy(&e.tuning,b+24,4);
  require(e.offset<(frames?frames:1)&&e.kind<=2&&get(b+28,4)==0&&std::isfinite(e.value)&&e.value>=0&&e.value<=1,"event values");
  require(e.kind==2?(e.channel==0&&e.pitch==0&&e.tuning==0):(frames&&e.channel>=0&&e.channel<16&&e.pitch>=0&&e.pitch<128&&std::isfinite(e.tuning)),"note extent");
 }return size_t(n);
}
#define AP8_UNKNOWN(Interface) \
 Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id,void**out)override {if(!out)return Steinberg::kInvalidArgument;*out=nullptr;if(Steinberg::FUnknownPrivate::iidEqual(id,Interface::iid)||Steinberg::FUnknownPrivate::iidEqual(id,Steinberg::FUnknown::iid)){*out=static_cast<Interface*>(this);addRef();return Steinberg::kResultOk;}return Steinberg::kNoInterface;} \
 Steinberg::uint32 PLUGIN_API addRef()override{return ++references;} \
 Steinberg::uint32 PLUGIN_API release()override{return --references;} \
 Steinberg::uint32 references=1;
class Notes final:public Steinberg::Vst::IEventList {
public:
 AP8_UNKNOWN(Steinberg::Vst::IEventList)
 std::array<Steinberg::Vst::Event,event_capacity> values{};Steinberg::int32 count=0;
 Steinberg::int32 PLUGIN_API getEventCount()override{return count;}
 Steinberg::tresult PLUGIN_API getEvent(Steinberg::int32 i,Steinberg::Vst::Event&e)override {if(i<0||i>=count)return Steinberg::kInvalidArgument;e=values[size_t(i)];return Steinberg::kResultOk;}
 Steinberg::tresult PLUGIN_API addEvent(Steinberg::Vst::Event&e)override {if(count>=event_capacity)return Steinberg::kResultFalse;values[size_t(count++)]=e;return Steinberg::kResultOk;}
};
class Points final:public Steinberg::Vst::IParamValueQueue {
public:
 AP8_UNKNOWN(Steinberg::Vst::IParamValueQueue)
 Steinberg::Vst::ParamID id=0;const InputEvent* events=nullptr;size_t count=0;
 Steinberg::Vst::ParamID PLUGIN_API getParameterId()override{return id;}
 Steinberg::int32 PLUGIN_API getPointCount()override {int n=0;for(size_t i=0;i<count;++i)n+=events[i].kind==2&&events[i].id==id;return n;}
 Steinberg::tresult PLUGIN_API getPoint(Steinberg::int32 index,Steinberg::int32&offset,Steinberg::Vst::ParamValue&value)override {for(size_t i=0;i<count;++i)if(events[i].kind==2&&events[i].id==id&&index--==0){offset=int(events[i].offset);value=events[i].value;return Steinberg::kResultOk;}return Steinberg::kResultFalse;}
 Steinberg::tresult PLUGIN_API addPoint(Steinberg::int32,Steinberg::Vst::ParamValue,Steinberg::int32&)override{return Steinberg::kNotImplemented;}
};
class Changes final:public Steinberg::Vst::IParameterChanges {
public:
 AP8_UNKNOWN(Steinberg::Vst::IParameterChanges)
 std::array<Points,event_capacity> queues;Steinberg::int32 count=0;
 Steinberg::int32 PLUGIN_API getParameterCount()override{return count;}
 Steinberg::Vst::IParamValueQueue* PLUGIN_API getParameterData(Steinberg::int32 i)override {return i>=0&&i<count?&queues[size_t(i)]:nullptr;}
 Steinberg::Vst::IParamValueQueue* PLUGIN_API addParameterData(const Steinberg::Vst::ParamID&,Steinberg::int32&)override{return nullptr;}
 void load(const InputEvent* values,size_t n,Notes&notes){
  using namespace Steinberg::Vst;count=0;notes.count=0;
  for(size_t i=0;i<n;++i){const auto&v=values[i];if(v.kind==2){int j=0;while(j<count&&queues[size_t(j)].id!=v.id)++j;if(j==count){auto&q=queues[size_t(count++)];q.id=v.id;q.events=values;q.count=n;}continue;}
   Event e{};e.busIndex=0;e.sampleOffset=int(v.offset);e.type=v.kind==0?Event::kNoteOnEvent:Event::kNoteOffEvent;
   if(v.kind==0){e.noteOn.channel=v.channel;e.noteOn.pitch=v.pitch;e.noteOn.velocity=float(v.value);e.noteOn.tuning=v.tuning;e.noteOn.noteId=int32_t(v.id);}
   else {e.noteOff.channel=v.channel;e.noteOff.pitch=v.pitch;e.noteOff.velocity=float(v.value);e.noteOff.tuning=v.tuning;e.noteOff.noteId=int32_t(v.id);}
   notes.addEvent(e);
  }
 }
};
#undef AP8_UNKNOWN
}
