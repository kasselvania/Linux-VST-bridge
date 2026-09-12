#pragma once
#include <windows.h>
#include <array>
#include <cstdint>
#include "ap1_protocol.h"
namespace linux_vst_bridge::wf0 {
// IF1 v1 sibling mapping. Only the UI/controller owner writes producer 2.
// Native and supervisor own separate slots. Commit follows complete scalar copy.
class TerminalStatus {
 HANDLE file_=INVALID_HANDLE_VALUE,mapping_=nullptr;uint8_t* view_=nullptr;
 bool completed_=false,pending_=false;uint64_t status_=0,domain_=0;
 friend struct TerminalStatusTestAccess;
 uint64_t load(size_t at)const{return uint64_t(InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(view_+at),0,0));}
 void store(size_t at,uint64_t v){InterlockedExchange64(reinterpret_cast<volatile LONG64*>(view_+at),LONG64(v));}
public:
 TerminalStatus(const std::wstring& directory,const std::array<uint8_t,16>& session){
  file_=CreateFileW((directory+L"\\if1.terminal").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
  if(file_==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_FILE_NOT_FOUND)return; // old immutable native
  try{
   LARGE_INTEGER n{};ap1::require(file_!=INVALID_HANDLE_VALUE&&GetFileSizeEx(file_,&n)&&n.QuadPart==2048,"IF1 extent");
   mapping_=CreateFileMappingW(file_,nullptr,PAGE_READWRITE,0,0,nullptr);ap1::require(mapping_!=nullptr,"IF1 mapping");
   view_=static_cast<uint8_t*>(MapViewOfFile(mapping_,FILE_MAP_READ|FILE_MAP_WRITE,0,0,2048));ap1::require(view_!=nullptr,"IF1 view");
   ap1::require(std::memcmp(view_,"LVIF",4)==0&&ap1::get(view_+4,4)==1&&ap1::get(view_+8,4)==2048&&ap1::get(view_+12,4)==0&&std::memcmp(view_+16,session.data(),16)==0,"IF1 identity");
  }catch(...){close();throw;}
 }
 ~TerminalStatus(){close();}
 TerminalStatus(const TerminalStatus&)=delete;
 void close(){if(view_)UnmapViewOfFile(view_);if(mapping_)CloseHandle(mapping_);if(file_!=INVALID_HANDLE_VALUE)CloseHandle(file_);view_=nullptr;mapping_=nullptr;file_=INVALID_HANDLE_VALUE;}
 bool editor_fatal(uint64_t status,uint64_t domain=2){
  return editor_fatal_after_copy(status,domain,[]{});
 }
private:
 template<class AfterCopy> bool editor_fatal_after_copy(uint64_t status,uint64_t domain,AfterCopy after_copy){
  if(!view_||completed_)return true;
  if(!pending_){status_=status;domain_=domain;pending_=true;}
  if(load(64)){completed_=true;return true;}
  std::array<uint64_t,24> row{};bool complete=false;
  for(unsigned attempt=0;attempt<3;++attempt){auto c=load(1024);if(!c)return false;auto at=1088+(c&1)*256;for(size_t i=0;i<row.size();++i)row[i]=load(at+i*8);after_copy();if(load(1024)==c){complete=true;break;}}
  if(!complete)return false; // a later owner call retries the same first failure
  row[14]=2;row[15]=status_;row[18]=2;row[19]=domain_;
  for(size_t i=0;i<row.size();++i)store(384+i*8,row[i]);
  InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(view_+64),2,0);
  completed_=true;return true;
 }
};
}
