#pragma once
#include <windows.h>
#include <array>
#include <cstdint>
#include <exception>
#include "ap1_protocol.h"
namespace linux_vst_bridge::wf0 {
// AP12 fault-status v1, independent of mailbox/protocol versions. Three lanes:
// native transport, Windows delivery, Windows UI owner. No C++ objects on wire.
// Every shared word uses interlocked access. Publish the inactive slot before
// its counter; an interrupted write leaves the last complete slot readable.
class FaultStatus {
 static constexpr size_t bytes=1024;
 HANDLE file=INVALID_HANDLE_VALUE,mapping=nullptr;uint8_t* view=nullptr;
 std::array<uint64_t,3> counters{};
 uint64_t frequency=0;
 void close(){if(view)UnmapViewOfFile(view);if(mapping)CloseHandle(mapping);if(file!=INVALID_HANDLE_VALUE)CloseHandle(file);view=nullptr;mapping=nullptr;file=INVALID_HANDLE_VALUE;}
 uint64_t load(size_t at)const{return uint64_t(InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(view+at),0,0));}
 void store(size_t at,uint64_t v){InterlockedExchange64(reinterpret_cast<volatile LONG64*>(view+at),LONG64(v));}
public:
 // Local rows have one owning thread each. Owner scopes may nest/reenter.
 struct Row {uint64_t generation=0,epoch=0,sequence=0,position=0,stage=0,detail=0;};
 std::array<Row,3> rows{};
 FaultStatus(const std::wstring& dir,const std::array<uint8_t,16>& session){
  try{
   file=CreateFileW((dir+L"\\ap12.status").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
   // Legacy diagnostic clients predate this independently versioned extension.
   if(file==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_FILE_NOT_FOUND)return;
   ap1::require(file!=INVALID_HANDLE_VALUE,"fault status open");LARGE_INTEGER n{};
   ap1::require(GetFileSizeEx(file,&n)&&n.QuadPart==bytes,"fault status extent");
   mapping=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,0,nullptr);ap1::require(mapping!=nullptr,"fault status mapping");
   view=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,bytes));ap1::require(view!=nullptr,"fault status view");
   ap1::require(std::memcmp(view,"LVFS",4)==0&&ap1::get(view+4,4)==1&&ap1::get(view+8,4)==bytes&&ap1::get(view+12,4)==0&&std::memcmp(view+16,session.data(),16)==0,"fault status identity/version");
   ap1::require(QueryPerformanceFrequency(&n)&&n.QuadPart>0,"fault status clock");frequency=uint64_t(n.QuadPart);
  }catch(...){close();throw;}
 }
 ~FaultStatus(){close();}
 FaultStatus(const FaultStatus&)=delete;
 void publish(size_t lane,Row row){
  if(!view)return;
  // Native owns generation. Read only a stable published generation, bounded.
  for(int i=0;i<3;++i){auto c=load(64);if(!c)break;auto g=load(128+(c&1)*128);if(load(64)==c){row.generation=g;break;}}
  rows[lane]=row;const auto next=++counters[lane];
  const size_t base=64+lane*320,slot=base+64+(next&1)*128;
  LARGE_INTEGER t{};QueryPerformanceCounter(&t);
  const std::array<uint64_t,10> data{row.generation,row.epoch,row.sequence,row.position,row.stage,row.detail,uint64_t(t.QuadPart),frequency,GetCurrentThreadId(),GetCurrentProcessId()};
  for(size_t i=0;i<data.size();++i)store(slot+i*8,data[i]);store(base,next);
 }
 void stage(size_t lane,uint64_t stage,uint64_t detail=0){auto r=rows[lane];r.stage=stage;r.detail=detail;publish(lane,r);}
 struct Scope {
  FaultStatus* status;size_t lane;Row previous;int exceptions=std::uncaught_exceptions();
  Scope(FaultStatus* s,size_t l,uint64_t stage,uint64_t detail=0):status(s),lane(l),previous(s?s->rows[l]:Row{}){if(s)s->stage(l,stage,detail);}
  ~Scope(){if(status&&std::uncaught_exceptions()==exceptions)status->publish(lane,previous);}
 };
};
}
