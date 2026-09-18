// Source-owned SCM fixture. Ordinary direct execution never establishes readiness.
#include <winsock2.h>
#include <windows.h>
#include <string>
#include <array>
#include <cstdio>
static void audit(const char* event,DWORD status=0){HANDLE f=CreateFileW(L"C:\\NAD1Fixture\\events.private",FILE_APPEND_DATA,FILE_SHARE_READ,nullptr,OPEN_ALWAYS,FILE_ATTRIBUTE_NORMAL,nullptr);if(f==INVALID_HANDLE_VALUE)return;char row[160];int n=std::snprintf(row,sizeof(row),"%s %lu\n",event,status);DWORD written;WriteFile(f,row,static_cast<DWORD>(n),&written,nullptr);CloseHandle(f);}
static SERVICE_STATUS_HANDLE handle{};
static SERVICE_STATUS status{};
static HANDLE stop_event{};
static constexpr auto service=L"NAD1FixtureService";
static constexpr auto daemon=L"C:\\NAD1Fixture\\NTKDaemon.exe";
static bool marker(const wchar_t* path){return GetFileAttributesW(path)!=INVALID_FILE_ATTRIBUTES;}
static void publish(DWORD state){status.dwServiceType=SERVICE_WIN32_OWN_PROCESS;status.dwCurrentState=state;status.dwControlsAccepted=state==SERVICE_RUNNING?SERVICE_ACCEPT_STOP:0;audit("state",state);SetServiceStatus(handle,&status);}
static DWORD WINAPI control(DWORD code,DWORD,LPVOID,LPVOID){if(code==SERVICE_CONTROL_STOP){if(marker(L"C:\\NAD1Fixture\\refuse-stop")){audit("stop_refused");return ERROR_ACCESS_DENIED;}audit("stop_requested");if(marker(L"C:\\NAD1Fixture\\no-transition"))return NO_ERROR;publish(SERVICE_STOP_PENDING);SetEvent(stop_event);}return NO_ERROR;}
static void WINAPI service_main(DWORD,LPWSTR*){
 audit("service_main");handle=RegisterServiceCtrlHandlerExW(service,control,nullptr);if(!handle){audit("handler_error",GetLastError());return;}
 stop_event=CreateEventW(nullptr,TRUE,FALSE,nullptr);publish(SERVICE_START_PENDING);
 WSADATA wsa{};if(WSAStartup(MAKEWORD(2,2),&wsa)!=0){status.dwWin32ExitCode=1;publish(SERVICE_STOPPED);return;}
 std::array<SOCKET,2> sockets{INVALID_SOCKET,INVALID_SOCKET};
 const bool ready=!marker(L"C:\\NAD1Fixture\\not-ready");
 if(ready){int i=0;for(unsigned short port:{5146,5563}){SOCKET s=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);sockaddr_in addr{};addr.sin_family=AF_INET;addr.sin_addr.s_addr=htonl(INADDR_LOOPBACK);addr.sin_port=htons(port);
  if(s==INVALID_SOCKET||bind(s,reinterpret_cast<sockaddr*>(&addr),sizeof(addr))!=0||listen(s,2)!=0){if(s!=INVALID_SOCKET)closesocket(s);status.dwWin32ExitCode=2;for(auto old:sockets)if(old!=INVALID_SOCKET)closesocket(old);publish(SERVICE_STOPPED);return;}sockets[i++]=s;}}
 publish(SERVICE_RUNNING);
 if(marker(L"C:\\NAD1Fixture\\self-stop")){audit("self_stop");publish(SERVICE_STOP_PENDING);SetEvent(stop_event);}
 while(WaitForSingleObject(stop_event,100)==WAIT_TIMEOUT&&!marker(L"C:\\NAD1Fixture\\release-stop")){}
 if(marker(L"C:\\NAD1Fixture\\delay-stop")){
  for(DWORD i=1;i<=30;i++){status.dwCheckPoint=i;status.dwWaitHint=2000;publish(SERVICE_STOP_PENDING);Sleep(100);}}
 if(marker(L"C:\\NAD1Fixture\\pending-hold")){
  DWORD checkpoint=status.dwCheckPoint;
  while(!marker(L"C:\\NAD1Fixture\\release-stop")){
   if(marker(L"C:\\NAD1Fixture\\pending-progress")){status.dwCheckPoint=++checkpoint;status.dwWaitHint=2000;publish(SERVICE_STOP_PENDING);}
   Sleep(100);
  }
 }
 const bool stopped_process_hold=marker(L"C:\\NAD1Fixture\\stopped-process-hold");
 const bool stopped_listener_hold=marker(L"C:\\NAD1Fixture\\stopped-listener-hold");
 if(stopped_process_hold&&!stopped_listener_hold){for(auto s:sockets)if(s!=INVALID_SOCKET)closesocket(s);sockets.fill(INVALID_SOCKET);WSACleanup();publish(SERVICE_STOPPED);}
 if(stopped_listener_hold)publish(SERVICE_STOPPED);
 if(stopped_process_hold||stopped_listener_hold)while(!marker(L"C:\\NAD1Fixture\\release-stop"))Sleep(100);
 for(auto s:sockets)if(s!=INVALID_SOCKET)closesocket(s);if(!(stopped_process_hold&&!stopped_listener_hold))WSACleanup();CloseHandle(stop_event);
 if(!stopped_process_hold&&!stopped_listener_hold)publish(SERVICE_STOPPED);
}
int wmain(int argc,wchar_t**argv){
 if(argc==2&&std::wstring(argv[1])==L"--self-test")return 0;
 if(argc==2&&std::wstring(argv[1])==L"/s"){
  CreateDirectoryW(L"C:\\NAD1Fixture",nullptr);std::array<wchar_t,32768> own{};GetModuleFileNameW(nullptr,own.data(),static_cast<DWORD>(own.size()));
  if(!CopyFileW(own.data(),daemon,FALSE))return 11;
  SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CREATE_SERVICE);if(!scm)return 12;
  std::wstring command=L"\"";command+=daemon;command+=L"\"";
  SC_HANDLE s=CreateServiceW(scm,service,service,SERVICE_ALL_ACCESS,SERVICE_WIN32_OWN_PROCESS,SERVICE_DEMAND_START,SERVICE_ERROR_NORMAL,command.c_str(),nullptr,nullptr,nullptr,nullptr,nullptr);
  if(!s){CloseServiceHandle(scm);return 13;}CloseServiceHandle(s);CloseServiceHandle(scm);return GetFileAttributesW(L"C:\\NAD1Fixture\\fail-after-register")!=INVALID_FILE_ATTRIBUTES?100:0;
 }
 if(argc==2&&std::wstring(argv[1])==L"--remove"){
  SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CONNECT);if(!scm)return 14;
  SC_HANDLE s=OpenServiceW(scm,service,DELETE|SERVICE_STOP|SERVICE_QUERY_STATUS);if(!s){CloseServiceHandle(scm);return 15;}
  SERVICE_STATUS st{};ControlService(s,SERVICE_CONTROL_STOP,&st);for(int i=0;i<100;i++){QueryServiceStatus(s,&st);if(st.dwCurrentState==SERVICE_STOPPED)break;Sleep(50);}bool ok=DeleteService(s)!=FALSE;CloseServiceHandle(s);CloseServiceHandle(scm);return ok?0:16;
 }
 if(argc!=1)return 64;
 SERVICE_TABLE_ENTRYW table[]={{const_cast<LPWSTR>(service),service_main},{nullptr,nullptr}};
 audit("dispatcher");if(!StartServiceCtrlDispatcherW(table)){DWORD error=GetLastError();audit("dispatcher_error",error);return static_cast<int>(error);}return 0;
}
