// Source-owned application: verifies the SCM prerequisite, never starts it.
#include <windows.h>
#include <winsvc.h>
#include <string>
#include "child_memory.h"
static constexpr wchar_t callback_pipe[]=L"\\\\.\\pipe\\LVBSourceOwnedLoginReturn";
static constexpr wchar_t callback_value[]=L"native-access:source-owned-fixture";
static bool callback_primary(){
 HANDLE pipe=CreateNamedPipeW(callback_pipe,PIPE_ACCESS_INBOUND,PIPE_TYPE_BYTE|PIPE_READMODE_BYTE|PIPE_NOWAIT,1,64,64,0,nullptr);
 if(pipe==INVALID_HANDLE_VALUE)return false;
 HANDLE ready=CreateFileW(L"C:\\NAD1Fixture\\callback-ready",GENERIC_WRITE,FILE_SHARE_READ,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);
 if(ready==INVALID_HANDLE_VALUE){CloseHandle(pipe);return false;}CloseHandle(ready);
 // Only the generated fixture has a wall-clock safety bound.
 auto deadline=GetTickCount64()+45000;bool received=false;
 while(GetTickCount64()<deadline){
  ConnectNamedPipe(pipe,nullptr);DWORD available=0;
  if(PeekNamedPipe(pipe,nullptr,0,nullptr,&available,nullptr)&&available){char b[16]{};DWORD n=0;received=ReadFile(pipe,b,sizeof(b),&n,nullptr)&&n==8&&std::string(b,n)=="returned";break;}
  Sleep(50);
 }
 DisconnectNamedPipe(pipe);CloseHandle(pipe);
 if(received){HANDLE f=CreateFileW(L"C:\\NAD1Fixture\\callback-received",GENERIC_WRITE,FILE_SHARE_READ,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);if(f==INVALID_HANDLE_VALUE)return false;CloseHandle(f);Sleep(1000);}
 return received;
}
static bool marker(const wchar_t* name){return GetFileAttributesW(name)!=INVALID_FILE_ATTRIBUTES;}
int wmain(int argc,wchar_t**argv){
 if(argc==2&&std::wstring(argv[1])==L"--self-test")return 0;
 if(argc==2&&std::wstring(argv[1])==L"--memory-child")return 65;
 if(argc==2&&std::wstring(argv[1])==L"--memory-control")return child_memory_probe(L"C:\\NAD1Fixture\\memory-control.json")?0:72;
 if(argc==3&&std::wstring(argv[1])==L"--disable-gpu"&&std::wstring(argv[2])==callback_value){
  HANDLE pipe=CreateFileW(callback_pipe,GENERIC_WRITE,0,nullptr,OPEN_EXISTING,0,nullptr);if(pipe==INVALID_HANDLE_VALUE)return 15;
  DWORD n=0;bool ok=WriteFile(pipe,"returned",8,&n,nullptr)&&n==8;CloseHandle(pipe);return ok?0:16;
 }
 if(argc!=2||std::wstring(argv[1])!=L"--disable-gpu")return 64;
 SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CONNECT);if(!scm)return 10;
 SC_HANDLE service=OpenServiceW(scm,L"NAD1FixtureService",SERVICE_QUERY_STATUS);if(!service){CloseServiceHandle(scm);return 11;}
 SERVICE_STATUS_PROCESS status{};DWORD used=0;
 bool ready=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&status),sizeof(status),&used)&&status.dwCurrentState==SERVICE_RUNNING;
 CloseServiceHandle(service);CloseServiceHandle(scm);if(!ready)return 12;
 HANDLE file=CreateFileW(L"C:\\NAD1Fixture\\application-started",GENERIC_WRITE,FILE_SHARE_READ,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);
 if(file==INVALID_HANDLE_VALUE)return 13;CloseHandle(file);
 if(marker(L"C:\\NAD1Fixture\\await-callback"))return callback_primary()?0:17;
 if(marker(L"C:\\NAD1Fixture\\probe-child-memory")&&!child_memory_probe(L"C:\\NAD1Fixture\\memory-service.json"))return 72;
 if(marker(L"C:\\NAD1Fixture\\await-explicit-exit")){
  // A fixture safety timeout is not a production application lifetime policy.
  auto deadline=GetTickCount64()+60000;
  while(!marker(L"C:\\NAD1Fixture\\explicit-exit")){
   if(GetTickCount64()>deadline)return 14;
   Sleep(50);
  }
 }else if(marker(L"C:\\NAD1Fixture\\hold-application"))Sleep(60000);else Sleep(500);
 return marker(L"C:\\NAD1Fixture\\fail-application")?7:0;
}
