#pragma once
#include <winsock2.h>
#include <windows.h>
#include <chrono>
#include <atomic>
#include <thread>
#include "ap1_protocol.h"
#include "notification_transport.h"
#include "../../tools/direct-audio-handoff/linux_wait.h"
namespace linux_vst_bridge::wf0 {
// Mailbox v4 has independent AUDIO and prepared lifecycle words; v3 adds diagnostics. Fixed wire bytes; no C++ object crosses the map.
// Socket control remains authenticated by the existing per-session handshake.
class DeliveryMailbox {
 static constexpr size_t bytes=33024,request_offset=256,reply_offset=16640,request_cap=16384,reply_cap=16384;
 lvb::linux_wait::Runtime direct_wait;
 using Delay=LONG (NTAPI*)(BOOLEAN,const LARGE_INTEGER*);Delay delay=nullptr;
 HANDLE file=INVALID_HANDLE_VALUE,mapping=nullptr;uint8_t* view=nullptr;
 std::vector<uint8_t> wire;
 LONG flag(size_t offset)const{return InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(view+offset),0,0);}
 void flag(size_t offset,LONG value){InterlockedExchange(reinterpret_cast<volatile LONG*>(view+offset),value);}
 volatile LONG* word(size_t offset)const{return reinterpret_cast<volatile LONG*>(view+offset);}
 void wake(size_t offset)noexcept{InterlockedIncrement(word(offset));direct_wait.wake(word(offset));}
 void close(){if(view)UnmapViewOfFile(view);if(mapping)CloseHandle(mapping);if(file!=INVALID_HANDLE_VALUE)CloseHandle(file);view=nullptr;mapping=nullptr;file=INVALID_HANDLE_VALUE;}
 static bool minor_capable(bool notified){return notified;}
public:
 uint32_t version=0;
 uint64_t sleep50_ns=0,delay50_ns=0;
 void pause(){ap1::require(delay!=nullptr,"legacy polling adapter absent");LARGE_INTEGER interval{};interval.QuadPart=-500;ap1::require(delay(FALSE,&interval)>=0,"delivery wait failed");}
 DeliveryMailbox(const std::wstring& directory,const std::array<uint8_t,16>& session,bool notified=false){
  using namespace ap1;
  try{
   wire.reserve(reply_cap);
   if(!notified){delay=reinterpret_cast<Delay>(GetProcAddress(GetModuleHandleW(L"ntdll.dll"),"NtDelayExecution"));require(delay!=nullptr,"precise delay unavailable");}
   file=CreateFileW((directory+L"\\ap10.delivery").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);
   require(file!=INVALID_HANDLE_VALUE,"delivery mapping file absent");LARGE_INTEGER size{};
   require(GetFileSizeEx(file,&size)&&size.QuadPart==bytes,"delivery mapping extent");
   mapping=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,0,nullptr);require(mapping!=nullptr,"delivery mapping creation");
   view=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,bytes));require(view!=nullptr,"delivery mapping view");
   version=uint32_t(get(view+4,4));
   require(std::memcmp(view,"LVBM",4)==0&&(version==2||version==3||version==4)&&get(view+8,4)==bytes&&get(view+12,4)==0&&std::memcmp(view+16,session.data(),16)==0,"delivery mapping version/identity");
   require(flag(64)==0&&flag(128)==0,"delivery mapping initially occupied");
   if(version==4){require(minor_capable(notified)&&flag(32)==0&&flag(36)==0,"direct delivery preparation");
    require(direct_wait.bind(),"Wine/Linux shared wait capability unavailable");
    lvb::linux_wait::Deadline now{};require(direct_wait.monotonic(now),"direct monotonic capability unavailable");}
   auto probe=[&](bool precise){auto start=std::chrono::steady_clock::now();for(int i=0;i<32;++i){if(precise)pause();else std::this_thread::sleep_for(std::chrono::microseconds(50));}return uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count()/32);};
   if(!notified){sleep50_ns=probe(false);delay50_ns=probe(true);}
  }catch(...){close();throw;}
 }
 ~DeliveryMailbox(){close();}
 DeliveryMailbox(const DeliveryMailbox&)=delete;
 void cancel(LONG code)noexcept{if(version!=4||!view)return;InterlockedCompareExchange(word(36),code?code:1,0);wake(40);wake(44);}
 void lifecycle_ready(){ap1::require(version==4&&flag(36)==0&&flag(32)==0,"direct lifecycle handoff occupied");flag(32,1);wake(40);}
 static void peer_failed(void* context)noexcept{static_cast<DeliveryMailbox*>(context)->cancel(1);}
 // Idle has no issued-request deadline, as on the existing socket path. The
 // outer owner contains a vanished native peer. Linux waits at most five seconds
 // for an issued request, counts gaps meanwhile, then fails the instance.
 bool receive(ap1::Frame& out,uint16_t minor,const std::atomic<bool>* cancelled=nullptr,NotificationTransport* notifications=nullptr){
  using namespace ap1;
  require(version==4||minor<15||notifications,"paired notification transport absent");
  if(version==4){
   require(minor==15,"direct delivery protocol differs");
   lvb::linux_wait::Deadline now{};require(direct_wait.monotonic(now),"direct monotonic clock");
   auto end=lvb::linux_wait::after(now,5000000000LL);
   for(;;){
    const LONG observed=flag(40);
    require(flag(36)==0&&(!cancelled||!cancelled->load(std::memory_order_acquire)),"direct owner/peer cancelled");
    const LONG state=flag(64);require(state==0||state==1,"direct request flag");
    if(state==1){const auto n=get(view+68,4);require(n>=header_bytes&&n<=request_cap&&n-header_bytes<=out.payload.capacity(),"direct request extent");
     decode_into(view+request_offset,size_t(n),minor,out);require(out.kind==Process,"direct request kind");flag(64,0);return true;}
    const LONG control=flag(32);require(control==0||control==1,"direct control flag");
    if(control){flag(32,0);return false;}
    require(direct_wait.monotonic(now),"direct monotonic clock");
    if(now.sec>end.sec||(now.sec==end.sec&&now.nsec>=end.nsec))end=lvb::linux_wait::after(now,5000000000LL); // idle: no issued ticket
    const auto result=direct_wait.wait(word(40),observed,end);
    require(result==0||result==-4||result==-11||result==-110,"direct shared wait failed");
   }
  }
  for(;;){require(!cancelled||!cancelled->load(std::memory_order_acquire),"owner service failed");auto state=flag(64);
   if(state==1){auto n=get(view+68,4);require(n>=header_bytes&&n<=request_cap&&n-header_bytes<=out.payload.capacity(),"delivery request extent");
    decode_into(view+request_offset,size_t(n),minor,out);
    require(out.kind==Process,"delivery request kind");flag(64,0);return true;
   }
   if(state==2){flag(64,0);if(notifications)notifications->signal_reply();return false;}
   require(state==0,"delivery request flag");
   if(notifications)notifications->wait_request();else pause();
  }
 }
 void send(const ap1::Frame& frame,uint16_t minor,const std::array<uint64_t,15>* diagnostic=nullptr){
  using namespace ap1;require(flag(128)==0,"delivery response occupied");encode_into(frame,minor,wire);
  require((frame.kind==Done||frame.kind==Error)&&wire.size()<=reply_cap,"delivery response extent/kind");
  std::memcpy(view+reply_offset,wire.data(),wire.size());put(view+132,wire.size(),4);
  if(version>=3){
   auto row=diagnostic?*diagnostic:std::array<uint64_t,15>{};
   if(diagnostic){LARGE_INTEGER stamp{};QueryPerformanceCounter(&stamp);row[7]=uint64_t(stamp.QuadPart);}
   for(size_t i=0;i<row.size();++i)put(view+136+i*8,row[i],8);
  }
  require(version!=4||flag(36)==0,"direct completion cancelled");
  flag(128,1);if(version==4)wake(44);
 }
};
}
