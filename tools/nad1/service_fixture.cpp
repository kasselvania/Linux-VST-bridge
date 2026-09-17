// Source-owned SCM fixture. Ordinary direct execution never establishes readiness.
#include <winsock2.h>
#include <windows.h>
#include <string>
#include <array>
static SERVICE_STATUS_HANDLE handle{};
static SERVICE_STATUS status{};
static HANDLE stop_event{};
static constexpr auto service=L"NAD1FixtureService";
static constexpr auto daemon=L"C:\\NAD1Fixture\\NTKDaemon.exe";
static void publish(DWORD state){status.dwServiceType=SERVICE_WIN32_OWN_PROCESS;status.dwCurrentState=state;status.dwControlsAccepted=state==SERVICE_RUNNING?SERVICE_ACCEPT_STOP:0;SetServiceStatus(handle,&status);}
static DWORD WINAPI control(DWORD code,DWORD,LPVOID,LPVOID){if(code==SERVICE_CONTROL_STOP){publish(SERVICE_STOP_PENDING);SetEvent(stop_event);}return NO_ERROR;}
static void WINAPI service_main(DWORD,LPWSTR*){
 handle=RegisterServiceCtrlHandlerExW(service,control,nullptr);if(!handle)return;
 stop_event=CreateEventW(nullptr,TRUE,FALSE,nullptr);publish(SERVICE_START_PENDING);
 WSADATA wsa{};if(WSAStartup(MAKEWORD(2,2),&wsa)!=0){status.dwWin32ExitCode=1;publish(SERVICE_STOPPED);return;}
 std::array<SOCKET,2> sockets{INVALID_SOCKET,INVALID_SOCKET};
 const bool ready=GetFileAttributesW(L"C:\\NAD1Fixture\\not-ready") == INVALID_FILE_ATTRIBUTES;
 if(ready){int i=0;for(unsigned short port:{5146,5563}){SOCKET s=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);sockaddr_in addr{};addr.sin_family=AF_INET;addr.sin_addr.s_addr=htonl(INADDR_LOOPBACK);addr.sin_port=htons(port);
  if(s==INVALID_SOCKET||bind(s,reinterpret_cast<sockaddr*>(&addr),sizeof(addr))!=0||listen(s,2)!=0){if(s!=INVALID_SOCKET)closesocket(s);status.dwWin32ExitCode=2;for(auto old:sockets)if(old!=INVALID_SOCKET)closesocket(old);publish(SERVICE_STOPPED);return;}sockets[i++]=s;}}
 publish(SERVICE_RUNNING);WaitForSingleObject(stop_event,90000);
 for(auto s:sockets)if(s!=INVALID_SOCKET)closesocket(s);WSACleanup();CloseHandle(stop_event);publish(SERVICE_STOPPED);
}
int wmain(int argc,wchar_t**argv){
 if(argc==2&&std::wstring(argv[1])==L"--self-test")return 0;
 if(argc==2&&std::wstring(argv[1])==L"/s"){
  CreateDirectoryW(L"C:\\NAD1Fixture",nullptr);std::array<wchar_t,32768> own{};GetModuleFileNameW(nullptr,own.data(),static_cast<DWORD>(own.size()));
  if(!CopyFileW(own.data(),daemon,FALSE))return 11;
  SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CREATE_SERVICE);if(!scm)return 12;
  std::wstring command=L"\"";command+=daemon;command+=L"\"";
  SC_HANDLE s=CreateServiceW(scm,service,service,SERVICE_ALL_ACCESS,SERVICE_WIN32_OWN_PROCESS,SERVICE_DEMAND_START,SERVICE_ERROR_NORMAL,command.c_str(),nullptr,nullptr,nullptr,nullptr,nullptr);
  if(!s){CloseServiceHandle(scm);return 13;}CloseServiceHandle(s);CloseServiceHandle(scm);return 0;
 }
 if(argc==2&&std::wstring(argv[1])==L"--remove"){
  SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CONNECT);if(!scm)return 14;
  SC_HANDLE s=OpenServiceW(scm,service,DELETE|SERVICE_STOP|SERVICE_QUERY_STATUS);if(!s){CloseServiceHandle(scm);return 15;}
  SERVICE_STATUS st{};ControlService(s,SERVICE_CONTROL_STOP,&st);for(int i=0;i<100;i++){QueryServiceStatus(s,&st);if(st.dwCurrentState==SERVICE_STOPPED)break;Sleep(50);}bool ok=DeleteService(s)!=FALSE;CloseServiceHandle(s);CloseServiceHandle(scm);return ok?0:16;
 }
 if(argc!=1)return 64;
 SERVICE_TABLE_ENTRYW table[]={{const_cast<LPWSTR>(service),service_main},{nullptr,nullptr}};
 if(!StartServiceCtrlDispatcherW(table))return static_cast<int>(GetLastError());return 0;
}
