#pragma once
#include <windows.h>
#include <chrono>
#include <thread>
#include "ap1_protocol.h"
namespace linux_vst_bridge::wf0 {
// Negotiated mailbox v2/v3; v3 adds 15 reply-owned diagnostic words. Fixed wire bytes; no C++ object crosses the map.
// Socket control remains authenticated by the existing per-session handshake.
class DeliveryMailbox {
 static constexpr size_t bytes=33024,request_offset=256,reply_offset=16640,request_cap=16384,reply_cap=16384;
 using Delay=LONG (NTAPI*)(BOOLEAN,const LARGE_INTEGER*);Delay delay=nullptr;
 HANDLE file=INVALID_HANDLE_VALUE,mapping=nullptr;uint8_t* view=nullptr;
 LONG flag(size_t offset)const{return InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(view+offset),0,0);}
 void flag(size_t offset,LONG value){InterlockedExchange(reinterpret_cast<volatile LONG*>(view+offset),value);}
 void close(){if(view)UnmapViewOfFile(view);if(mapping)CloseHandle(mapping);if(file!=INVALID_HANDLE_VALUE)CloseHandle(file);view=nullptr;mapping=nullptr;file=INVALID_HANDLE_VALUE;}
public:
 uint32_t version=0;
 uint64_t sleep50_ns=0,delay50_ns=0;
 void pause(){LARGE_INTEGER interval{};interval.QuadPart=-500;ap1::require(delay(FALSE,&interval)>=0,"delivery wait failed");}
 DeliveryMailbox(const std::wstring& directory,const std::array<uint8_t,16>& session){
  using namespace ap1;
  try{
   delay=reinterpret_cast<Delay>(GetProcAddress(GetModuleHandleW(L"ntdll.dll"),"NtDelayExecution"));require(delay!=nullptr,"precise delay unavailable");
   file=CreateFileW((directory+L"\\ap10.delivery").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
   require(file!=INVALID_HANDLE_VALUE,"delivery mapping file absent");LARGE_INTEGER size{};
   require(GetFileSizeEx(file,&size)&&size.QuadPart==bytes,"delivery mapping extent");
   mapping=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,0,nullptr);require(mapping!=nullptr,"delivery mapping creation");
   view=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,bytes));require(view!=nullptr,"delivery mapping view");
   version=uint32_t(get(view+4,4));
   require(std::memcmp(view,"LVBM",4)==0&&(version==2||version==3)&&get(view+8,4)==bytes&&get(view+12,4)==0&&std::memcmp(view+16,session.data(),16)==0,"delivery mapping version/identity");
   require(flag(64)==0&&flag(128)==0,"delivery mapping initially occupied");
   auto probe=[&](bool precise){auto start=std::chrono::steady_clock::now();for(int i=0;i<32;++i){if(precise)pause();else std::this_thread::sleep_for(std::chrono::microseconds(50));}return uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count()/32);};
   sleep50_ns=probe(false);delay50_ns=probe(true);
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
   pause();
  }
 }
 void send(const ap1::Frame& frame,uint16_t minor,const std::array<uint64_t,15>* diagnostic=nullptr){
  using namespace ap1;require(flag(128)==0,"delivery response occupied");auto data=encode(frame,minor);
  require((frame.kind==Done||frame.kind==Error)&&data.size()<=reply_cap,"delivery response extent/kind");
  std::memcpy(view+reply_offset,data.data(),data.size());put(view+132,data.size(),4);
  if(version==3){
   auto row=diagnostic?*diagnostic:std::array<uint64_t,15>{};
   if(diagnostic){LARGE_INTEGER stamp{};QueryPerformanceCounter(&stamp);row[7]=uint64_t(stamp.QuadPart);}
   for(size_t i=0;i<row.size();++i)put(view+136+i*8,row[i],8);
  }
  flag(128,1);
 }
};
}
