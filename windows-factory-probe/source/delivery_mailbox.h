#pragma once
#include <windows.h>
#include <chrono>
#include <thread>
#include "ap1_protocol.h"
namespace linux_vst_bridge::wf0 {
// Negotiated AP10 mailbox v1. Fixed wire bytes; no C++ object crosses the map.
// Socket control remains authenticated by the existing per-session handshake.
class DeliveryMailbox {
 static constexpr size_t bytes=16896,request_offset=256,reply_offset=16640,request_cap=16384,reply_cap=256;
 HANDLE file=INVALID_HANDLE_VALUE,mapping=nullptr;uint8_t* view=nullptr;
 LONG flag(size_t offset)const{return InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(view+offset),0,0);}
 void flag(size_t offset,LONG value){InterlockedExchange(reinterpret_cast<volatile LONG*>(view+offset),value);}
 void close(){if(view)UnmapViewOfFile(view);if(mapping)CloseHandle(mapping);if(file!=INVALID_HANDLE_VALUE)CloseHandle(file);view=nullptr;mapping=nullptr;file=INVALID_HANDLE_VALUE;}
public:
 DeliveryMailbox(const std::wstring& directory,const std::array<uint8_t,16>& session){
  using namespace ap1;
  try{
   file=CreateFileW((directory+L"\\ap10.delivery").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
   require(file!=INVALID_HANDLE_VALUE,"delivery mapping file absent");LARGE_INTEGER size{};
   require(GetFileSizeEx(file,&size)&&size.QuadPart==bytes,"delivery mapping extent");
   mapping=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,0,nullptr);require(mapping!=nullptr,"delivery mapping creation");
   view=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,bytes));require(view!=nullptr,"delivery mapping view");
   require(std::memcmp(view,"LVBM",4)==0&&get(view+4,4)==1&&get(view+8,4)==bytes&&get(view+12,4)==0&&std::memcmp(view+16,session.data(),16)==0,"delivery mapping version/identity");
   require(flag(64)==0&&flag(128)==0,"delivery mapping initially occupied");
  }catch(...){close();throw;}
 }
 ~DeliveryMailbox(){close();}
 DeliveryMailbox(const DeliveryMailbox&)=delete;
 // Idle has no issued-request deadline, as on the existing socket path. The
 // outer owner contains a vanished native peer. Linux waits at most five seconds
 // for an issued request, counts gaps meanwhile, then fails the instance.
 bool receive(ap1::Frame& out,uint16_t minor){
  using namespace ap1;
  for(;;){auto state=flag(64);
   if(state==1){auto n=get(view+68,4);require(n>=header_bytes&&n<=request_cap,"delivery request extent");
    std::vector<uint8_t> data(view+request_offset,view+request_offset+n);out=decode(data,minor);
    require(out.kind==Process,"delivery request kind");flag(64,0);return true;
   }
   if(state==2){flag(64,0);return false;}
   require(state==0,"delivery request flag");
   std::this_thread::sleep_for(std::chrono::microseconds(50));
  }
 }
 void send(const ap1::Frame& frame,uint16_t minor){
  using namespace ap1;require(flag(128)==0,"delivery response occupied");auto data=encode(frame,minor);
  require((frame.kind==Done||frame.kind==Error)&&data.size()<=reply_cap,"delivery response extent/kind");
  std::memcpy(view+reply_offset,data.data(),data.size());put(view+132,data.size(),4);flag(128,1);
 }
};
}
