#!/usr/bin/env python3
"""Run the paired Windows pump against real sockets, events and mapped slots."""
import os
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
TEST = r'''
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <atomic>
#include <chrono>
#include <thread>
#include <array>
#include <algorithm>
#include <filesystem>
#include <cstdlib>
#include <new>
#include <cassert>
#include <iostream>
static thread_local bool count_allocations=false,count_network=false;
static thread_local size_t allocations=0,deallocations=0,network_calls=0;
void* operator new(size_t n){if(count_allocations)++allocations;if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}
void operator delete(void*p)noexcept{if(count_allocations)++deallocations;std::free(p);}
void operator delete(void*p,size_t)noexcept{if(count_allocations)++deallocations;std::free(p);}
void* operator new[](size_t n){if(count_allocations)++allocations;if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}
void operator delete[](void*p)noexcept{if(count_allocations)++deallocations;std::free(p);}
void operator delete[](void*p,size_t)noexcept{if(count_allocations)++deallocations;std::free(p);}
static int observed_send(SOCKET fd,const char*p,int n,int flags){if(count_network)++network_calls;return ::send(fd,p,n,flags);}
static int observed_recv(SOCKET fd,char*p,int n,int flags){if(count_network)++network_calls;return ::recv(fd,p,n,flags);}
static int observed_select(int n,fd_set*r,fd_set*w,fd_set*e,const timeval*t){if(count_network)++network_calls;return ::select(n,r,w,e,t);}
static int observed_shutdown(SOCKET fd,int how){if(count_network)++network_calls;return ::shutdown(fd,how);}
#define send observed_send
#define recv observed_recv
#define select observed_select
#define shutdown observed_shutdown
#include "notification_transport.h"
#undef send
#undef recv
#undef select
#undef shutdown
#include "delivery_mailbox.h"
using namespace linux_vst_bridge;
using namespace ap1;
using Clock=std::chrono::steady_clock;
template<class F>void until(F f){auto end=Clock::now()+std::chrono::seconds(3);while(!f()){assert(Clock::now()<end);Sleep(1);}}
struct Listener {
 SOCKET fd=INVALID_SOCKET;uint16_t port=0;
 Listener(){fd=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);assert(fd!=INVALID_SOCKET);sockaddr_in a{};a.sin_family=AF_INET;a.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  assert(bind(fd,reinterpret_cast<sockaddr*>(&a),sizeof(a))==0&&listen(fd,1)==0);int n=sizeof(a);assert(getsockname(fd,reinterpret_cast<sockaddr*>(&a),&n)==0);port=ntohs(a.sin_port);}
 SOCKET accept(){auto peer=::accept(fd,nullptr,nullptr);assert(peer!=INVALID_SOCKET);DWORD timeout=4000;assert(setsockopt(peer,SOL_SOCKET,SO_RCVTIMEO,reinterpret_cast<const char*>(&timeout),sizeof(timeout))==0);return peer;}
 SOCKET connect(){auto peer=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);assert(peer!=INVALID_SOCKET);sockaddr_in a{};a.sin_family=AF_INET;a.sin_addr.s_addr=htonl(INADDR_LOOPBACK);a.sin_port=htons(port);assert(::connect(peer,reinterpret_cast<sockaddr*>(&a),sizeof(a))==0);return peer;}
 ~Listener(){if(fd!=INVALID_SOCKET)closesocket(fd);}
};
struct Peer {
 SOCKET fd=INVALID_SOCKET;
 void transfer(uint8_t*p,size_t n,bool out){while(n){auto k=out ? ::send(fd,reinterpret_cast<const char*>(p),int(n),0) : ::recv(fd,reinterpret_cast<char*>(p),int(n),0);assert(k>0);p+=k;n-=size_t(k);}}
 void write(Frame frame,bool fragmented=false){auto bytes=encode(frame,15);if(fragmented){transfer(bytes.data(),3,true);Sleep(5);transfer(bytes.data()+3,bytes.size()-3,true);}else transfer(bytes.data(),bytes.size(),true);}
 Frame read(){std::vector<uint8_t>bytes(header_bytes);transfer(bytes.data(),bytes.size(),false);auto n=payload_length(bytes.data(),15);bytes.resize(header_bytes+n);if(n)transfer(bytes.data()+header_bytes,n,false);return decode(bytes,15);}
 void wake(size_t count=1){uint8_t value=1;while(count--)transfer(&value,1,true);}
 void receive_wake(){uint8_t value=0;transfer(&value,1,false);assert(value==1);}
 ~Peer(){if(fd!=INVALID_SOCKET)closesocket(fd);}
};
Frame offer(const Listener& listener,const std::array<uint8_t,16>& id){Frame f{Hello,id,0,std::vector<uint8_t>(44)};std::memcpy(f.payload.data(),"LVBW",4);put(f.payload.data()+4,1,4);put(f.payload.data()+8,listener.port,2);for(int i=0;i<32;++i)f.payload[size_t(i)+12]=uint8_t(61+i);return f;}
void authenticate(Peer& peer,const Frame& offered){auto h=peer.read();assert(h.kind==Hello&&h.session==offered.session&&h.sequence==0&&h.payload.size()==40);assert(get(h.payload.data(),4)==1&&get(h.payload.data()+4,4)==1&&std::memcmp(h.payload.data()+8,offered.payload.data()+12,32)==0);peer.write({Hello,offered.session,0,{}});}
struct View {
 HANDLE file=INVALID_HANDLE_VALUE,mapping=nullptr;uint8_t*p=nullptr;
 View(const std::filesystem::path& path,const std::array<uint8_t,16>& id){file=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);assert(file!=INVALID_HANDLE_VALUE);mapping=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,33024,nullptr);assert(mapping);p=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_ALL_ACCESS,0,0,33024));assert(p);std::memset(p,0,33024);std::memcpy(p,"LVBM",4);put(p+4,3,4);put(p+8,33024,4);std::memcpy(p+16,id.data(),16);}
 LONG flag(size_t at){return InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(p+at),0,0);}
 void flag(size_t at,LONG v){InterlockedExchange(reinterpret_cast<volatile LONG*>(p+at),v);}
 ~View(){UnmapViewOfFile(p);CloseHandle(mapping);CloseHandle(file);}
};
void normal_and_cancel(const std::filesystem::path& directory){
 std::array<uint8_t,16>id{};id[0]=31;View view(directory/L"ap10.delivery",id);
 Listener control_listener,notification_listener;Peer host{control_listener.connect()};auto native_control=control_listener.accept();auto offered=offer(notification_listener,id);
 std::atomic<bool>finished{false},native_ready{false};
 std::thread native([&]{Peer control{native_control},notification{notification_listener.accept()};authenticate(notification,offered);notification.wake(3);
  control.write({Start,id,1,std::vector<uint8_t>(8)},true);auto started=control.read();assert(started.kind==Started&&started.sequence==1);
  for(uint64_t sequence=10;sequence<13;++sequence){
   assert(view.flag(64)==0&&view.flag(128)==0);auto request=encode({Process,id,sequence,std::vector<uint8_t>(8360)},15);std::memcpy(view.p+256,request.data(),request.size());put(view.p+68,request.size(),4);
   if(sequence==10)Sleep(20);view.flag(64,1);notification.wake(2);
   until([&]{return view.flag(128)==1;});auto n=get(view.p+132,4);auto result=decode({view.p+16640,view.p+16640+n},15);assert(result.kind==Done&&result.sequence==sequence&&result.session==id&&result.payload.size()==10312);view.flag(128,0);notification.receive_wake();
  }
  control.write({GetState,id,20,{}});view.flag(64,2);notification.wake();notification.receive_wake();
  native_ready.store(true,std::memory_order_release);until([&]{return finished.load(std::memory_order_acquire);});
 });
 {
  wf0::NotificationTransport transport(host.fd,offered,id);wf0::DeliveryMailbox mailbox(directory.wstring(),id,true);
  Frame request{};request.payload.reserve(8360);Frame reply{Started,id,1,std::vector<uint8_t>(8)};
  count_allocations=count_network=true;transport.read(request,true);assert(request.kind==Start&&request.sequence==1);transport.write(reply,false);count_allocations=count_network=false;
  reply.kind=Done;reply.payload.resize(10312);
  for(uint64_t sequence=10;sequence<13;++sequence){
   count_allocations=count_network=true;assert(mailbox.receive(request,15,nullptr,&transport));assert(request.sequence==sequence&&request.payload.size()==8360);reply.sequence=sequence;mailbox.send(reply,15);transport.signal_reply();count_allocations=count_network=false;
  }
  count_allocations=count_network=true;assert(!mailbox.receive(request,15,nullptr,&transport));transport.read(request,true);count_allocations=count_network=false;
  assert(request.kind==GetState&&request.sequence==20);assert(allocations==0&&deallocations==0&&network_calls==0);until([&]{return native_ready.load(std::memory_order_acquire);});
  bool cancelled=false;size_t wait_network_calls=0;
  std::thread waiting([&]{count_network=true;try{mailbox.receive(request,15,nullptr,&transport);}catch(const std::exception&){cancelled=true;}count_network=false;wait_network_calls=network_calls;});
  Sleep(20);auto begin=Clock::now();transport.cancel();waiting.join();assert(cancelled&&wait_network_calls==0&&Clock::now()-begin<std::chrono::milliseconds(500));
  transport.stop();assert(Clock::now()-begin<std::chrono::seconds(1));
  // Storage stays mapped until the waiter and transport pump have both ended.
 }
 finished.store(true,std::memory_order_release);native.join();
}
void peer_loss_and_offer_refusal(){
 std::array<uint8_t,16>id{};id[0]=32;Listener control_listener,notification_listener;Peer host{control_listener.connect()};Peer control{control_listener.accept()};auto offered=offer(notification_listener,id);
 for(int field=0;field<4;++field){auto invalid=offered;if(field==0)invalid.session[0]^=1;if(field==1)invalid.payload[4]^=1;if(field==2)invalid.payload[10]=1;if(field==3)invalid.payload.resize(43);bool refused=false;try{wf0::NotificationTransport wrong(host.fd,invalid,id);}catch(const std::exception&){refused=true;}assert(refused);}
 std::atomic<bool>disconnect{false};std::thread peer([&]{Peer notification{notification_listener.accept()};authenticate(notification,offered);until([&]{return disconnect.load(std::memory_order_acquire);});});
 wf0::NotificationTransport transport(host.fd,offered,id);Frame frame{};frame.payload.reserve(8360);bool refused=false;size_t calls=0;
 std::thread waiting([&]{count_network=true;try{transport.read(frame,true);}catch(const std::exception&){refused=true;}count_network=false;calls=network_calls;});
 Sleep(20);auto begin=Clock::now();disconnect.store(true,std::memory_order_release);waiting.join();peer.join();assert(refused&&calls==0&&transport.failed()&&Clock::now()-begin<std::chrono::milliseconds(500));transport.stop();
}
int main(){WSADATA data{};assert(WSAStartup(MAKEWORD(2,2),&data)==0);auto directory=std::filesystem::current_path();normal_and_cancel(directory);peer_loss_and_offer_refusal();assert(DeleteFileW((directory/L"ap10.delivery").c_str()));WSACleanup();std::cout<<"Windows paired pump: bounded event delivery, prepared render storage, no render socket I/O, cancellation and peer loss PASS\n";}
'''


def main():
    if os.name != "nt":
        print("Windows notification integration requires Windows; exercised by Windows CI")
        return
    with tempfile.TemporaryDirectory(prefix="lvb-notifications-") as directory:
        path = pathlib.Path(directory)
        source = path / "test.cpp"
        source.write_text(TEST)
        executable = path / "test.exe"
        subprocess.run(
            ["cl", "/nologo", "/std:c++20", "/EHsc", "/DNOMINMAX",
             f'/I{ROOT / "windows-factory-probe/source"}', str(source),
             f"/Fe:{executable}", "ws2_32.lib"], cwd=path, check=True,
        )
        subprocess.run([str(executable)], cwd=path, check=True, timeout=20)


if __name__ == "__main__":
    main()
