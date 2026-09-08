#pragma once
#include "ap10_sdk_results.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
namespace AP10Results {
// Lives on the native processor. All SDK calls use the current callback sinks.
// Stop/failure only requests cleanup; no host process pointer is retained.
class Output {
 // Match the pending queue's 512-event bound. Each submitted payload gets an
 // aligned slot, independent of the reusable drain packet. Construct/touch
 // this storage with the processor, before activation; never allocate in RT.
 alignas(TChar) std::array<std::array<uint8_t,ap10_event_payload_capacity>,512> payloads{};
 size_t payload_count=0;
 static_assert(ap10_event_payload_capacity%alignof(TChar)==0);
 std::array<Event,512> notes{};size_t note_count=0;
 static bool match(const Event&on,const Event&off){return on.busIndex==off.busIndex&&on.noteOn.noteId==off.noteOff.noteId&&(off.noteOff.noteId!=-1||(on.noteOn.channel==off.noteOff.channel&&on.noteOn.pitch==off.noteOff.pitch));}
public:
 ap10_results_t packet{};
 // Called once at process entry under its guard, never between drains or at
 // process exit. Host EventLists may shallow-copy pointers and consume them
 // after process returns. Storage remains intact until the next callback.
 void beginCallback(){payload_count=0;}
 bool release_requested=false;
 uint64_t events=0,points=0,unrequested_events=0,unrequested_points=0,rejected=0,cleanup_sent=0,cleanup_unavailable=0;
 size_t active_notes()const{return note_count;}
 template<class Active> bool release(ProcessData&data,Active active){
  if(!release_requested)return true;
  if(!note_count){release_requested=false;return true;}
  if(!data.outputEvents){cleanup_unavailable+=note_count;return false;}
  bool ok=true;size_t i=0;
  while(i<note_count){auto on=notes[i];if(active(on.busIndex)!=1){++cleanup_unavailable;++i;ok=false;continue;}
   Event off{};off.busIndex=on.busIndex;off.sampleOffset=0;off.ppqPosition=on.ppqPosition;off.flags=on.flags;off.type=Event::kNoteOffEvent;
   off.noteOff={on.noteOn.channel,on.noteOn.pitch,0,on.noteOn.noteId,on.noteOn.tuning};
   bool accepted=false;try{accepted=data.outputEvents->addEvent(off)==kResultOk;}catch(...){accepted=false;}
   if(!accepted){++rejected;++i;ok=false;continue;}
   ++cleanup_sent;for(size_t j=i+1;j<note_count;++j)notes[j-1]=notes[j];--note_count;
  }
  release_requested=note_count!=0;return ok;
 }
 template<class Active,class Known> bool deliver(ProcessData&data,Active active,Known known){
  for(uint32_t i=0;i<packet.events;++i){const auto&r=packet.event[i];auto disposition=active(r.bus);
   if(disposition<0){++rejected;return false;}
   if(!disposition||!data.outputEvents){++unrequested_events;continue;}
   auto owned=r;owned.payload_offset=0;
   const uint8_t* payload=payloads[0].data();
   if(r.payload_size){
    if(payload_count==payloads.size()||r.payload_size>ap10_event_payload_capacity||
       packet.bytes>ap10_payload_capacity||r.payload_offset>packet.bytes||
       r.payload_size>packet.bytes-r.payload_offset){++rejected;return false;}
    auto& slot=payloads[payload_count++];
    std::memcpy(slot.data(),packet.payload+r.payload_offset,r.payload_size);
    payload=slot.data();
   }
   auto e=sdk(owned,payload);if(e.type==Event::kNoteOnEvent&&note_count==notes.size()){++rejected;return false;}
   bool accepted=false;try{accepted=data.outputEvents->addEvent(e)==kResultOk;}catch(...){accepted=false;}
   if(!accepted){++rejected;return false;}++events;
   if(e.type==Event::kNoteOnEvent)notes[note_count++]=sdk(r,packet.payload);
   if(e.type==Event::kNoteOffEvent)for(size_t j=0;j<note_count;++j)if(match(notes[j],e)){for(size_t k=j+1;k<note_count;++k)notes[k-1]=notes[k];--note_count;break;}
  }
  for(uint32_t i=0;i<packet.points;++i){const auto&p=packet.point[i];if(!known(p.id)){++rejected;return false;}
   if(!data.outputParameterChanges){++unrequested_points;continue;}
   bool accepted=false;try{int32 index=0;auto*q=data.outputParameterChanges->addParameterData(p.id,index);accepted=q&&q->addPoint(p.offset,p.value,index)==kResultOk;}catch(...){accepted=false;}
   if(!accepted){++rejected;return false;}++points;
  }return true;
 }
};
}
