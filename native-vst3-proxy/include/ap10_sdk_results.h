#pragma once
#include "ap10_results.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include <array>
#include <bit>
#include <cmath>
#include <cstring>
#include <algorithm>
#include <type_traits>
namespace AP10Results {
using namespace Steinberg;
using namespace Steinberg::Vst;
// Diagnostic-only, fixed scalar snapshot. No payloads, pointers or ABI changes.
enum class Rejection : uint32_t { None, EventCapacity, NegativeBus, UndeclaredBus,
 BusStorage, NegativeEventOffset, EventExtent, NonFinitePPQ, UnsupportedEvent,
 InvalidEventField, EventChannel, EventPayload, PayloadCapacity, NullPayload,
 PayloadAlignment, QueueCapacity, PointCapacity, NegativePointOffset, PointExtent,
 NonFiniteValue, ValueBelowZero, ValueAboveOne };
struct RejectionRecord {
 Rejection reason; int32_t frames; uint32_t events,points,queues,bytes;
 int32_t event_type,bus,offset,channel,declared_channels,event_bus_active;
 uint32_t flags,declared_buses,payload_type,parameter_id;
 uint64_t payload_size,ppq_bits,value_bits; int32_t field_a,field_b; uint64_t extra_bits;
};
static_assert(std::is_trivial_v<RejectionRecord> && std::is_standard_layout_v<RejectionRecord>);
inline bool normalized(double v){return std::isfinite(v)&&v>=0&&v<=1;}
// Decode the closed value representation; pointers borrow only this packet.
inline Event sdk(const ap10_event_result_t&r,const uint8_t*payload){
 Event e{};e.busIndex=r.bus;e.sampleOffset=r.offset;e.ppqPosition=r.ppq;e.flags=r.flags;e.type=r.kind;
 auto f=std::bit_cast<float>(uint32_t(r.value)),t=std::bit_cast<float>(uint32_t(r.extra));
 const auto* text=reinterpret_cast<const TChar*>(payload+r.payload_offset);
 switch(e.type){
 case Event::kNoteOnEvent:e.noteOn={int16(r.b),int16(r.c),t,f,int32(r.d),r.a};break;
 case Event::kNoteOffEvent:e.noteOff={int16(r.b),int16(r.c),f,r.a,t};break;
 case Event::kDataEvent:e.data={r.payload_size,r.d,payload+r.payload_offset};break;
 case Event::kPolyPressureEvent:e.polyPressure={int16(r.b),int16(r.c),f,r.a};break;
 case Event::kNoteExpressionValueEvent:e.noteExpressionValue={r.d,r.a,std::bit_cast<double>(r.value)};break;
 case Event::kNoteExpressionIntValueEvent:e.noteExpressionIntValue={r.d,r.a,r.value};break;
 case Event::kNoteExpressionTextEvent:e.noteExpressionText={r.d,r.a,r.payload_size/2-1,text};break;
 case Event::kChordEvent:e.chord={int16(r.a),int16(r.b),int16(r.c),uint16(r.payload_size/2-1),text};break;
 case Event::kScaleEvent:e.scale={int16(r.a),int16(r.c),uint16(r.payload_size/2-1),text};break;
 case Event::kLegacyMIDICCOutEvent:e.midiCCOut={uint8(r.a),int8(r.b),int8(r.c),int8(r.d)};break;
 }
 return e;
}
inline bool append(ap10_results_t&out,const Event&e,int32 frames,Rejection*why=nullptr){
 auto reject=[&](Rejection r){if(why)*why=r;return false;};
 if(out.events==ap10_event_capacity)return reject(Rejection::EventCapacity);
 if(e.busIndex<0)return reject(Rejection::NegativeBus);
 if(e.busIndex>=8)return reject(Rejection::BusStorage);
 if(e.sampleOffset<0)return reject(Rejection::NegativeEventOffset);
 if(e.sampleOffset>=std::max(frames,1))return reject(Rejection::EventExtent);
 if(!std::isfinite(e.ppqPosition))return reject(Rejection::NonFinitePPQ);
 ap10_event_result_t r{};r.offset=e.sampleOffset;r.bus=e.busIndex;r.ppq=e.ppqPosition;r.flags=e.flags;r.kind=e.type;
 const void* bytes=nullptr;uint32_t size=0;bool text=false;
 auto note=[&](int16 channel,int16 pitch,float value,float tuning){r.b=channel;r.c=pitch;r.value=std::bit_cast<uint32_t>(value);r.extra=std::bit_cast<uint32_t>(tuning);return channel>=0&&channel<16&&pitch>=0&&pitch<128&&normalized(value)&&std::isfinite(tuning);};
 switch(e.type){
 case Event::kNoteOnEvent:r.a=e.noteOn.noteId;r.d=uint32_t(e.noteOn.length);if(!note(e.noteOn.channel,e.noteOn.pitch,e.noteOn.velocity,e.noteOn.tuning))return reject(Rejection::InvalidEventField);break;
 case Event::kNoteOffEvent:r.a=e.noteOff.noteId;if(!note(e.noteOff.channel,e.noteOff.pitch,e.noteOff.velocity,e.noteOff.tuning))return reject(Rejection::InvalidEventField);break;
 case Event::kPolyPressureEvent:r.a=e.polyPressure.noteId;if(!note(e.polyPressure.channel,e.polyPressure.pitch,e.polyPressure.pressure,0))return reject(Rejection::InvalidEventField);break;
 case Event::kDataEvent:r.d=e.data.type;bytes=e.data.bytes;size=e.data.size;if(r.d!=DataEvent::kMidiSysEx)return reject(Rejection::InvalidEventField);break;
 case Event::kNoteExpressionValueEvent:r.a=e.noteExpressionValue.noteId;r.d=e.noteExpressionValue.typeId;r.value=std::bit_cast<uint64_t>(e.noteExpressionValue.value);if(!normalized(e.noteExpressionValue.value))return reject(Rejection::InvalidEventField);break;
 case Event::kNoteExpressionIntValueEvent:r.a=e.noteExpressionIntValue.noteId;r.d=e.noteExpressionIntValue.typeId;r.value=e.noteExpressionIntValue.value;break;
 case Event::kNoteExpressionTextEvent:r.a=e.noteExpressionText.noteId;r.d=e.noteExpressionText.typeId;bytes=e.noteExpressionText.text;if(e.noteExpressionText.textLen>255)return reject(Rejection::EventPayload);size=e.noteExpressionText.textLen*2;text=true;break;
 case Event::kChordEvent:r.a=e.chord.root;r.b=e.chord.bassNote;r.c=e.chord.mask;bytes=e.chord.text;size=e.chord.textLen*2;text=true;if(r.a<0||r.a>127||r.b<0||r.b>127)return reject(Rejection::InvalidEventField);break;
 case Event::kScaleEvent:r.a=e.scale.root;r.c=e.scale.mask;bytes=e.scale.text;size=e.scale.textLen*2;text=true;if(r.a<0||r.a>127)return reject(Rejection::InvalidEventField);break;
 case Event::kLegacyMIDICCOutEvent:r.a=e.midiCCOut.controlNumber;r.b=e.midiCCOut.channel;r.c=e.midiCCOut.value;r.d=uint32_t(e.midiCCOut.value2);if(r.b<0||r.b>15||r.c<0||r.c>127||r.d>127)return reject(Rejection::InvalidEventField);break;
 default:return reject(Rejection::UnsupportedEvent);
 }
 uint32_t length=size+(text?2:0);
 if(length>ap10_event_payload_capacity)return reject(Rejection::EventPayload);
 if(out.bytes+length>ap10_payload_capacity)return reject(Rejection::PayloadCapacity);
 if(size&&!bytes)return reject(Rejection::NullPayload);
 // All payload starts stay 2-byte aligned so UTF-16 pointers are valid.
 if(length){auto start=(out.bytes+1)&~1u;if(start+length>ap10_payload_capacity)return reject(Rejection::PayloadAlignment);if(start>out.bytes)out.payload[out.bytes]=0;r.payload_offset=start;r.payload_size=length;if(size)std::memcpy(out.payload+start,bytes,size);if(text)std::memset(out.payload+start+size,0,2);out.bytes=start+length;}
 out.event[out.events++]=r;return true;
}
#define AP10_UNKNOWN(Interface) \
 tresult PLUGIN_API queryInterface(const TUID id,void**out)override{if(!out)return kInvalidArgument;*out=nullptr;if(FUnknownPrivate::iidEqual(id,Interface::iid)||FUnknownPrivate::iidEqual(id,FUnknown::iid)){*out=static_cast<Interface*>(this);addRef();return kResultOk;}return kNoInterface;} \
 uint32 PLUGIN_API addRef()override{return ++references;} \
 uint32 PLUGIN_API release()override{return --references;}uint32 references=1;
class Collector final:public IEventList,public IParameterChanges {
 struct PointQueue final:public IParamValueQueue {
  AP10_UNKNOWN(IParamValueQueue)
  Collector*owner=nullptr;ParamID id=0;
  ParamID PLUGIN_API getParameterId()override{return id;}
  int32 PLUGIN_API getPointCount()override{int32 n=0;for(uint32_t i=0;i<owner->values.points;++i)n+=owner->values.point[i].id==id;return n;}
  tresult PLUGIN_API getPoint(int32 index,int32&offset,ParamValue&value)override{if(index<0)return kInvalidArgument;for(uint32_t i=0;i<owner->values.points;++i){auto&p=owner->values.point[i];if(p.id==id&&index--==0){offset=p.offset;value=p.value;return kResultOk;}}return kResultFalse;}
  tresult PLUGIN_API addPoint(int32 offset,ParamValue value,int32&index)override{
   if(owner->failed)return kResultFalse;
   Rejection reason=Rejection::None;
   if(offset<0)reason=Rejection::NegativePointOffset;
   else if(offset>=std::max(owner->frames,1))reason=Rejection::PointExtent;
   else if(!std::isfinite(value))reason=Rejection::NonFiniteValue;
   else if(value<0)reason=Rejection::ValueBelowZero;
   else if(value>1)reason=Rejection::ValueAboveOne;
   else if(owner->values.points==ap10_point_capacity)reason=Rejection::PointCapacity;
   if(reason!=Rejection::None){owner->reject_point(reason,id,offset,value);return kResultFalse;}
   index=getPointCount();owner->values.point[owner->values.points++]={offset,id,value};return kResultOk;
  }
 };
 std::array<PointQueue,ap10_point_capacity> queues{};int32 count=0;
public:
 ap10_results_t values{};int32 frames=0;bool failed=false;
 RejectionRecord rejection{};
 // -1 means the caller has not supplied activation metadata.
 std::array<int32_t,8> bus_active{-1,-1,-1,-1,-1,-1,-1,-1};
 RejectionRecord context()const noexcept {
  RejectionRecord r{};r.frames=frames;r.events=values.events;r.points=values.points;
  r.queues=uint32_t(count);r.bytes=values.bytes;r.event_type=r.bus=r.channel=r.declared_channels=r.event_bus_active=-1;r.declared_buses=buses;return r;
 }
 void retain(RejectionRecord r,Rejection reason)noexcept {if(rejection.reason==Rejection::None){r.reason=reason;rejection=r;}failed=true;}
 void reject_point(Rejection reason,ParamID id,int32 offset,double value)noexcept {
  auto r=context();r.parameter_id=id;r.offset=offset;r.value_bits=std::bit_cast<uint64_t>(value);retain(r,reason);
 }
 RejectionRecord event_context(const Event&e)const noexcept {
  auto r=context();r.event_type=e.type;r.bus=e.busIndex;r.offset=e.sampleOffset;r.flags=e.flags;r.ppq_bits=std::bit_cast<uint64_t>(e.ppqPosition);
  if(e.busIndex>=0&&e.busIndex<8&&uint32_t(e.busIndex)<buses){r.declared_channels=channels[e.busIndex];r.event_bus_active=bus_active[e.busIndex];}
  switch(e.type){
  case Event::kNoteOnEvent:r.channel=e.noteOn.channel;r.field_a=e.noteOn.pitch;r.field_b=e.noteOn.length;r.value_bits=std::bit_cast<uint32_t>(e.noteOn.velocity);r.extra_bits=std::bit_cast<uint32_t>(e.noteOn.tuning);break;
  case Event::kNoteOffEvent:r.channel=e.noteOff.channel;r.field_a=e.noteOff.pitch;r.value_bits=std::bit_cast<uint32_t>(e.noteOff.velocity);r.extra_bits=std::bit_cast<uint32_t>(e.noteOff.tuning);break;
  case Event::kPolyPressureEvent:r.channel=e.polyPressure.channel;r.field_a=e.polyPressure.pitch;r.value_bits=std::bit_cast<uint32_t>(e.polyPressure.pressure);break;
  case Event::kLegacyMIDICCOutEvent:r.channel=e.midiCCOut.channel;r.field_a=e.midiCCOut.controlNumber;r.field_b=e.midiCCOut.value;r.extra_bits=uint32_t(e.midiCCOut.value2);break;
  case Event::kDataEvent:r.payload_type=e.data.type;r.payload_size=e.data.size;break;
  case Event::kNoteExpressionTextEvent:r.payload_type=e.noteExpressionText.typeId;r.payload_size=uint64_t(e.noteExpressionText.textLen)*2+2;break;
  case Event::kChordEvent:r.field_a=e.chord.root;r.field_b=e.chord.bassNote;r.payload_size=uint64_t(e.chord.textLen)*2+2;break;
  case Event::kScaleEvent:r.field_a=e.scale.root;r.payload_size=uint64_t(e.scale.textLen)*2+2;break;
  case Event::kNoteExpressionValueEvent:r.field_a=e.noteExpressionValue.noteId;r.payload_type=e.noteExpressionValue.typeId;r.value_bits=std::bit_cast<uint64_t>(e.noteExpressionValue.value);break;
  case Event::kNoteExpressionIntValueEvent:r.field_a=e.noteExpressionIntValue.noteId;r.payload_type=e.noteExpressionIntValue.typeId;r.value_bits=e.noteExpressionIntValue.value;break;
  default:break;
  }return r;
 }
 std::array<int32,8> channels{};uint32_t buses=0;
 void reset(int32 n){values.events=values.points=values.bytes=0;count=0;frames=n;failed=false;rejection={};}
 tresult PLUGIN_API queryInterface(const TUID id,void**out)override{if(!out)return kInvalidArgument;*out=nullptr;if(FUnknownPrivate::iidEqual(id,IEventList::iid)||FUnknownPrivate::iidEqual(id,FUnknown::iid))*out=static_cast<IEventList*>(this);else if(FUnknownPrivate::iidEqual(id,IParameterChanges::iid))*out=static_cast<IParameterChanges*>(this);else return kNoInterface;addRef();return kResultOk;}
 uint32 references=1;uint32 PLUGIN_API addRef()override{return ++references;}uint32 PLUGIN_API release()override{return --references;}
 int32 PLUGIN_API getEventCount()override{return int32(values.events);}
 tresult PLUGIN_API getEvent(int32 i,Event&e)override{if(i<0||uint32(i)>=values.events)return kInvalidArgument;e=sdk(values.event[i],values.payload);return kResultOk;}
 tresult PLUGIN_API addEvent(Event&e)override{
  if(failed)return kResultFalse;
  const auto before=event_context(e);Rejection reason=Rejection::None;
  if(e.busIndex<0)reason=Rejection::NegativeBus;
  else if(uint32(e.busIndex)>=buses)reason=Rejection::UndeclaredBus;
  else if(!append(values,e,frames,&reason)){}
  else {auto&r=values.event[values.events-1];if((r.kind<=1||r.kind==3||r.kind==65535)&&r.b>=channels[r.bus])reason=Rejection::EventChannel;}
  if(reason!=Rejection::None){retain(before,reason);return kResultFalse;}return kResultOk;
 }
 int32 PLUGIN_API getParameterCount()override{return count;}
 IParamValueQueue* PLUGIN_API getParameterData(int32 i)override{return i>=0&&i<count?&queues[i]:nullptr;}
 IParamValueQueue* PLUGIN_API addParameterData(const ParamID&id,int32&index)override{
  if(failed)return nullptr;
  for(int32 i=0;i<count;++i)if(queues[i].id==id){index=i;return &queues[i];}
  if(count==int32(ap10_point_capacity)){reject_point(Rejection::QueueCapacity,id,0,0);return nullptr;}
  index=count;auto&q=queues[count++];q.id=id;q.owner=this;return &q;
 }
};
#undef AP10_UNKNOWN
}
