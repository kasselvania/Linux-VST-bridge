#!/usr/bin/env python3
"""Exercise actual Windows mapping/atomic APIs against the delivery header."""
import os,pathlib,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
TEST=r'''
#include "delivery_mailbox.h"
#include <cassert>
#include <filesystem>
#include <iostream>
using namespace linux_vst_bridge;
int main(){
 auto dir=std::filesystem::current_path();auto path=dir/L"ap10.delivery";
 auto f=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);assert(f!=INVALID_HANDLE_VALUE);
 std::vector<uint8_t> initial(33024);std::memcpy(initial.data(),"LVBM",4);ap1::put(initial.data()+4,2,4);ap1::put(initial.data()+8,initial.size(),4);initial[16]=3;
 DWORD written=0;assert(WriteFile(f,initial.data(),DWORD(initial.size()),&written,nullptr)&&written==initial.size());
 auto mapping=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,0,nullptr);assert(mapping);
 auto* v=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,0));assert(v);
 std::array<uint8_t,16> id{};id[0]=3;
 {
  wf0::DeliveryMailbox mailbox(dir.wstring(),id);
  ap1::Frame request{ap1::Process,id,9,std::vector<uint8_t>(56)};
  auto encoded=ap1::encode(request,7);std::memcpy(v+256,encoded.data(),encoded.size());ap1::put(v+68,encoded.size(),4);InterlockedExchange(reinterpret_cast<volatile LONG*>(v+64),1);
  ap1::Frame got{};assert(mailbox.receive(got,7)&&got.sequence==9&&got.session==id&&got.payload==request.payload);
  assert(InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(v+64),0,0)==0);
  ap1::Frame reply{ap1::Done,id,9,std::vector<uint8_t>(40)};mailbox.send(reply,7);
  assert(InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(v+128),0,0)==1);
  auto n=ap1::get(v+132,4);assert(n==96);auto result=ap1::decode(std::vector<uint8_t>(v+16640,v+16640+n),7);assert(result.sequence==9&&result.session==id);
  bool refused=false;try{mailbox.send(reply,7);}catch(...){refused=true;}assert(refused);
  InterlockedExchange(reinterpret_cast<volatile LONG*>(v+128),0);
  InterlockedExchange(reinterpret_cast<volatile LONG*>(v+64),2);assert(!mailbox.receive(got,7));
  ap1::put(v+68,16385,4);InterlockedExchange(reinterpret_cast<volatile LONG*>(v+64),1);
  refused=false;try{mailbox.receive(got,7);}catch(...){refused=true;}assert(refused);
  InterlockedExchange(reinterpret_cast<volatile LONG*>(v+64),0);
 }
 // The new mapping is identity-bound, even when its directory is private.
 id[0]=4;bool refused=false;try{wf0::DeliveryMailbox wrong(dir.wstring(),id);}catch(...){refused=true;}assert(refused);
 UnmapViewOfFile(v);CloseHandle(mapping);CloseHandle(f);assert(DeleteFileW(path.c_str()));
 std::cout<<"Windows mailbox identity, bounds, control and reply ownership passed\n";
}
'''
def main():
 if os.name!='nt':
  print('Windows mapping integration requires Windows; exercised by Windows CI');return
 with tempfile.TemporaryDirectory(prefix='ap10-mailbox-') as d:
  p=pathlib.Path(d);source=p/'test.cpp';source.write_text(TEST);exe=p/'test.exe'
  subprocess.run(['cl','/nologo','/std:c++20','/EHsc','/DNOMINMAX',f'/I{ROOT / "windows-factory-probe/source"}',str(source),f'/Fe:{exe}'],cwd=p,check=True)
  subprocess.run([str(exe)],cwd=p,check=True,timeout=10)
if __name__=='__main__':main()
