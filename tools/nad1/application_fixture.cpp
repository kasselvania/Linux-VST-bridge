// Source-owned application: verifies the SCM prerequisite, never starts it.
#include <windows.h>
#include <winsvc.h>
#include <string>
static bool marker(const wchar_t* name){return GetFileAttributesW(name)!=INVALID_FILE_ATTRIBUTES;}
int wmain(int argc,wchar_t**argv){
 if(argc==2&&std::wstring(argv[1])==L"--self-test")return 0;
 if(argc!=2||std::wstring(argv[1])!=L"--disable-gpu")return 64;
 SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CONNECT);if(!scm)return 10;
 SC_HANDLE service=OpenServiceW(scm,L"NAD1FixtureService",SERVICE_QUERY_STATUS);if(!service){CloseServiceHandle(scm);return 11;}
 SERVICE_STATUS_PROCESS status{};DWORD used=0;
 bool ready=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&status),sizeof(status),&used)&&status.dwCurrentState==SERVICE_RUNNING;
 CloseServiceHandle(service);CloseServiceHandle(scm);if(!ready)return 12;
 HANDLE file=CreateFileW(L"C:\\NAD1Fixture\\application-started",GENERIC_WRITE,FILE_SHARE_READ,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);
 if(file==INVALID_HANDLE_VALUE)return 13;CloseHandle(file);
 if(marker(L"C:\\NAD1Fixture\\hold-application"))Sleep(60000);else Sleep(500);
 return marker(L"C:\\NAD1Fixture\\fail-application")?7:0;
}
