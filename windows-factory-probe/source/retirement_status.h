#pragma once
#include <windows.h>
#include <array>
#include "ap1_protocol.h"
namespace linux_vst_bridge::wf0 {
// LVRT v1 is final-retirement authority, separate from optional diagnostics.
// One UI-owner writer. No vendor pointers or payloads cross this boundary.
class RetirementStatus {
 HANDLE file_=INVALID_HANDLE_VALUE,map_=nullptr;uint8_t* data_=nullptr;
 void close(){if(data_)UnmapViewOfFile(data_);if(map_)CloseHandle(map_);if(file_!=INVALID_HANDLE_VALUE)CloseHandle(file_);}
public:
 static constexpr uint64_t complete=127; // stopped, joined, inactive, detached,
 // parent absent, bridge close acknowledged, no pending state
 static bool selected(){
  wchar_t value[96]{};auto n=GetEnvironmentVariableW(L"LVB_VENDOR_RETIREMENT",value,96);
  if(!n)return false;
  ap1::require(n<96&&std::wstring(value)==L"process_scoped_vendor_retirement","unsupported vendor retirement");return true;
 }
 RetirementStatus(const std::wstring& directory,const std::array<uint8_t,16>& session){
  try{
   file_=CreateFileW((directory+L"\\ap18.retirement").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
   ap1::require(file_!=INVALID_HANDLE_VALUE,"retirement status required");LARGE_INTEGER n{};
   ap1::require(GetFileSizeEx(file_,&n)&&n.QuadPart==256,"retirement status extent");
   map_=CreateFileMappingW(file_,nullptr,PAGE_READWRITE,0,0,nullptr);ap1::require(map_!=nullptr,"retirement status mapping");
   data_=static_cast<uint8_t*>(MapViewOfFile(map_,FILE_MAP_ALL_ACCESS,0,0,256));ap1::require(data_!=nullptr,"retirement status view");
   ap1::require(std::memcmp(data_,"LVRT",4)==0&&ap1::get(data_+4,4)==1&&ap1::get(data_+8,4)==256&&ap1::get(data_+12,4)==8&&std::memcmp(data_+16,session.data(),16)==0,"retirement status identity/version");
  }catch(...){close();throw;}
 }
 ~RetirementStatus(){close();}
 RetirementStatus(const RetirementStatus&)=delete;
 void publish(uint64_t flags,uint64_t epoch,uint64_t sequence,uint64_t position,uint64_t generation){
  ap1::require(flags==complete,"retirement milestones incomplete");
  auto at=[&](size_t n){return reinterpret_cast<volatile LONG64*>(data_+n);};
  ap1::require(!InterlockedCompareExchange64(at(64),0,0),"retirement already committed");
  const std::array<uint64_t,8> row{flags,epoch,sequence,position,generation,0,0,0};
  for(size_t i=0;i<row.size();++i)InterlockedExchange64(at(192+i*8),LONG64(row[i]));
  InterlockedExchange64(at(64),1);
 }
};
}
