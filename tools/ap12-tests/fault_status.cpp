// Independent-process reader observes production status while an SDK producer
// is permanently stopped at delivery boundaries. No destructor/trace dump runs.
#include "fault_status.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <cassert>
#include <filesystem>
#include <iostream>
#include <thread>
using namespace linux_vst_bridge;
using namespace Steinberg;
using namespace Steinberg::Vst;
struct Producer final : AudioEffect {
 bool block=false;
 tresult PLUGIN_API process(ProcessData&) override {if(block)Sleep(INFINITE);return kResultOk;}
};
uint64_t load(uint8_t* v,size_t offset){return uint64_t(InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(v+offset),0,0));}
int wmain(int argc,wchar_t** argv){
 std::array<uint8_t,16> sid{};sid.fill(12);
 if(argc==3){
  const int stop=_wtoi(argv[2]);wf0::FaultStatus status(argv[1],sid);
  std::thread delivery([&]{
   status.publish(1,{0,1,9,0,1,0});if(stop==1)Sleep(INFINITE);
   status.stage(1,2);if(stop==2)Sleep(INFINITE);
   Producer sdk;sdk.block=stop==3;ProcessData data{};data.numSamples=256;
   status.stage(1,3);assert(sdk.process(data)==kResultOk);
   status.stage(1,4);if(stop==4)Sleep(INFINITE);
   status.stage(1,5);if(stop==5)Sleep(INFINITE);
   status.stage(1,6);Sleep(INFINITE);
  });
  wf0::FaultStatus::Scope owner(&status,2,22);Sleep(INFINITE);
  return 99;
 }
 auto root=std::filesystem::temp_directory_path()/(L"ap12-fault-"+std::to_wstring(GetCurrentProcessId()));
 std::filesystem::create_directory(root);
 wchar_t exe[32768]{};assert(GetModuleFileNameW(nullptr,exe,32768)>0);
 for(int stop=1;stop<=6;++stop){
  auto dir=root/std::to_wstring(stop);std::filesystem::create_directory(dir);auto path=dir/L"ap12.status";
  HANDLE f=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);assert(f!=INVALID_HANDLE_VALUE);
  std::array<uint8_t,1024> data{};std::memcpy(data.data(),"LVFS",4);ap1::put(data.data()+4,1,4);ap1::put(data.data()+8,1024,4);std::memcpy(data.data()+16,sid.data(),16);
  // Native lane already published generation 4, first epoch/request, pre-note.
  ap1::put(data.data()+64,1,8);ap1::put(data.data()+256,4,8);
  DWORD n=0;assert(WriteFile(f,data.data(),DWORD(data.size()),&n,nullptr)&&n==data.size());
  HANDLE m=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,0,nullptr);assert(m);auto* v=static_cast<uint8_t*>(MapViewOfFile(m,FILE_MAP_READ|FILE_MAP_WRITE,0,0,0));assert(v);
  std::wstring command=L"\""+std::wstring(exe)+L"\" \""+dir.wstring()+L"\" "+std::to_wstring(stop);
  STARTUPINFOW startup{};startup.cb=sizeof(startup);PROCESS_INFORMATION child{};
  assert(CreateProcessW(exe,command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW,nullptr,nullptr,&startup,&child));
  auto end=GetTickCount64()+5000;std::array<uint64_t,10> row{};bool observed=false;
  while(GetTickCount64()<end){
   auto c=load(v,384);if(c){const size_t slot=448+(c&1)*128;for(size_t i=0;i<row.size();++i)row[i]=load(v,slot+i*8);
    if(load(v,384)==c&&row[4]==uint64_t(stop)){observed=true;break;}}
   Sleep(1);
  }
  assert(observed&&row[0]==4&&row[1]==1&&row[2]==9&&row[3]==0&&row[6]>0&&row[7]>0);
  assert(WaitForSingleObject(child.hProcess,0)==WAIT_TIMEOUT);
  assert(TerminateProcess(child.hProcess,91));assert(WaitForSingleObject(child.hProcess,3000)==WAIT_OBJECT_0);
  // No normal producer shutdown; status remains readable outside that process.
  auto c=load(v,384);assert(load(v,448+(c&1)*128+4*8)==uint64_t(stop));
  CloseHandle(child.hThread);CloseHandle(child.hProcess);UnmapViewOfFile(v);CloseHandle(m);CloseHandle(f);
 }
 std::filesystem::remove_all(root);
 std::cout<<"SDK producer and independent owner: six unfinished pre-note stages retained before/after containment\n";
}
