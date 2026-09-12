#pragma once
#include <windows.h>
#include <array>
#include "ap1_protocol.h"
#include "../../native-vst3-proxy/include/ap10_sdk_results.h"
namespace linux_vst_bridge::wf0 {
// Independent LVRS v1 diagnostic mapping; no audio/UI/wire layout change.
// One processing-thread writer, first rejection only. Header/session immutable.
// Every slot word is interlocked; inactive slot is complete before commit.
class ResultStatus {
 HANDLE file_=INVALID_HANDLE_VALUE,map_=nullptr;uint8_t* data_=nullptr;
 void close(){if(data_)UnmapViewOfFile(data_);if(map_)CloseHandle(map_);if(file_!=INVALID_HANDLE_VALUE)CloseHandle(file_);data_=nullptr;map_=nullptr;file_=INVALID_HANDLE_VALUE;}
public:
 static constexpr size_t extent=1024,words=31;
 ResultStatus(const std::wstring& directory,const std::array<uint8_t,16>& session){
  try{
   file_=CreateFileW((directory+L"\\ap18.results").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
   if(file_==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_FILE_NOT_FOUND)return; // retained old supervisor
   ap1::require(file_!=INVALID_HANDLE_VALUE,"result status open");LARGE_INTEGER n{};
   ap1::require(GetFileSizeEx(file_,&n)&&n.QuadPart==extent,"result status extent");
   map_=CreateFileMappingW(file_,nullptr,PAGE_READWRITE,0,0,nullptr);ap1::require(map_!=nullptr,"result status mapping");
   data_=static_cast<uint8_t*>(MapViewOfFile(map_,FILE_MAP_ALL_ACCESS,0,0,extent));ap1::require(data_!=nullptr,"result status view");
   ap1::require(std::memcmp(data_,"LVRS",4)==0&&ap1::get(data_+4,4)==1&&ap1::get(data_+8,4)==extent&&ap1::get(data_+12,4)==words&&std::memcmp(data_+16,session.data(),16)==0,"result status identity/version");
  }catch(...){close();throw;}
 }
 ~ResultStatus(){close();}
 ResultStatus(const ResultStatus&)=delete;
 bool published() const noexcept {return data_&&InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(data_+64),0,0)!=0;}
 void publish(const AP10Results::RejectionRecord& r,uint64_t generation,uint64_t epoch,uint64_t sequence,uint64_t position,uint64_t callback,uint64_t notes,uint64_t parameters) noexcept {
  if(!data_||r.reason==AP10Results::Rejection::None)return;
  auto at=[&](size_t n){return reinterpret_cast<volatile LONG64*>(data_+n);};
  if(InterlockedCompareExchange64(at(64),0,0))return;
  // Signed SDK fields retain their exact two's-complement low 32 bits.
  const std::array<uint64_t,words> row{generation,epoch,sequence,position,callback,notes,parameters,
   uint32_t(r.reason),uint32_t(r.frames),r.events,r.points,r.queues,r.bytes,uint32_t(r.event_type),uint32_t(r.bus),uint32_t(r.offset),uint32_t(r.channel),uint32_t(r.declared_channels),uint32_t(r.event_bus_active),r.flags,r.declared_buses,r.payload_type,r.payload_size,r.parameter_id,r.ppq_bits,r.value_bits,uint32_t(r.field_a),uint32_t(r.field_b),r.extra_bits,1,0};
  for(size_t i=0;i<words;++i)InterlockedExchange64(at(512+i*8),LONG64(row[i]));
  InterlockedExchange64(at(64),1);
 }
};
}
