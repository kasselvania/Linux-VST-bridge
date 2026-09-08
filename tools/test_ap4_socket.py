#!/usr/bin/env python3
"""Actual Windows Socket body: Winsock on CI, POSIX syscall shim locally."""
import pathlib
import subprocess
import tempfile
import os

ROOT = pathlib.Path(__file__).resolve().parents[1]

def main():
    source = (ROOT / 'windows-factory-probe/source/mapped_processing.cpp').read_text()
    body = source[source.index('struct Socket {'):source.index('struct Handle{')]
    includes = r'''
#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <sys/socket.h>
#include <sys/select.h>
#include <sys/ioctl.h>
#include <netinet/in.h>
#include <unistd.h>
#include <cerrno>
using SOCKET=int;
constexpr int INVALID_SOCKET=-1, SOCKET_ERROR=-1, WSAEWOULDBLOCK=EWOULDBLOCK;
int closesocket(int s){return close(s);}
int WSAGetLastError(){return errno;}
int ioctlsocket(int s,unsigned long c,unsigned long* p){return ioctl(s,c,p);}
int winselect(int,fd_set* r,fd_set* w,fd_set* e,timeval* t){return select(FD_SETSIZE,r,w,e,t);}
#define select winselect
#endif
#include "ap1_protocol.h"
#include <chrono>
#include <thread>
#include <iostream>
using namespace linux_vst_bridge::ap1;
'''
    tests = r'''
int main(){
#ifdef _WIN32
 WSADATA data{};require(WSAStartup(MAKEWORD(2,2),&data)==0,"WSAStartup");
#endif
 for(int fast=0;fast<2;++fast)for(int scenario=0;scenario<5;++scenario){
  SOCKET listener=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);
  sockaddr_in a{};a.sin_family=AF_INET;a.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  require(bind(listener,reinterpret_cast<sockaddr*>(&a),sizeof(a))==0,"bind");
#ifdef _WIN32
  int size=sizeof(a);
#else
  socklen_t size=sizeof(a);
#endif
  require(getsockname(listener,reinterpret_cast<sockaddr*>(&a),&size)==0,"name");
  require(listen(listener,1)==0,"listen");
  Socket sender;sender.eager=fast!=0;sender.value=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);
  require(connect(sender.value,reinterpret_cast<sockaddr*>(&a),sizeof(a))==0,"connect");
  Socket receiver;receiver.eager=fast!=0;receiver.value=accept(listener,nullptr,nullptr);closesocket(listener);
  unsigned long nonblock=1;require(ioctlsocket(sender.value,FIONBIO,&nonblock)==0,"sender nonblock");require(ioctlsocket(receiver.value,FIONBIO,&nonblock)==0,"nonblock");
  auto started=std::chrono::steady_clock::now();bool failed=false;
  std::thread peer([&]{
   if(scenario==0){
    std::this_thread::sleep_for(std::chrono::milliseconds(6100));
    sender.write(Frame{Close,{},1,{}});sender.write(Frame{Close,{},2,{}});
   }else if(scenario==1){shutdown(sender.value,2);}
   else if(scenario==2){char byte='L';send(sender.value,&byte,1,0);}
   else if(scenario==3){auto b=encode(Frame{Hello,{},0,{1,2,3}});send(sender.value,reinterpret_cast<const char*>(b.data()),int(header_bytes),0);}
  });
  try{
   auto f=receiver.receive(scenario!=4);
   require(scenario==0&&f.kind==Close&&f.sequence==1,"first command");
   require(receiver.receive(true).sequence==2,"next command");
  }catch(const std::exception&){failed=true;}
  peer.join();
  auto seconds=std::chrono::duration<double>(std::chrono::steady_clock::now()-started).count();
  require(failed==(scenario!=0),"idle versus failure result");
  if(scenario==0)require(seconds>=6&&seconds<9,"idle survival");
  if(scenario==1)require(seconds<2,"disconnect detection");
  if(scenario>=2)require(seconds>=4.5&&seconds<8,"bounded message/reply deadline");
  std::cout<<"socket scenario "<<scenario<<" passed\n";
 }
}
'''
    with tempfile.TemporaryDirectory(prefix='ap4-socket-') as temp:
        root = pathlib.Path(temp)
        unit = root / 'socket.cpp'
        unit.write_text(includes + body + tests)
        executable = root / ('socket.exe' if os.name == 'nt' else 'socket')
        command = (['cl', '/nologo', '/std:c++20', '/EHsc', '/W4',
                    '/I' + str(ROOT/'windows-factory-probe/source'), str(unit),
                    '/Fe:' + str(executable), '/link', 'ws2_32.lib'] if os.name == 'nt' else
                   ['c++', '-std=c++20', '-pthread', '-I', str(ROOT/'windows-factory-probe/source'), str(unit), '-o', str(executable)])
        subprocess.run(command, cwd=root, check=True)
        subprocess.run([str(executable)], cwd=root, check=True, timeout=60)

if __name__ == '__main__':
    main()
