// Source-owned installer instrumentation. No vendor/account/input/SDK behavior.
#include <windows.h>
#include <cstdio>
#include <string>
#include <vector>

static void event(const char* kind, const wchar_t* role, DWORD code) {
    std::printf("IS1_FIXTURE_V1 kind=%s pid=%lu role=%ls code=%lu tick=%llu\n",kind,GetCurrentProcessId(),role,code,GetTickCount64());
    std::fflush(stdout);
}
static std::wstring image() { wchar_t p[32768];DWORD n=GetModuleFileNameW(nullptr,p,32768);return std::wstring(p,n); }
static PROCESS_INFORMATION spawn(const wchar_t* role,DWORD code,DWORD delay,const std::wstring& executable=L"") {
    std::wstring cmd=L"\""+(executable.empty()?image():executable)+L"\" --child "+role+L" "+std::to_wstring(code)+L" "+std::to_wstring(delay);
    STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};
    if(!CreateProcessW(nullptr,cmd.data(),nullptr,nullptr,FALSE,0,nullptr,nullptr,&si,&pi)){event("spawn_error",role,GetLastError());return {};}
    event("spawned",role,pi.dwProcessId);CloseHandle(pi.hThread);return pi;
}
static DWORD join(PROCESS_INFORMATION p) {
    if(!p.hProcess)return 125;
    if(WaitForSingleObject(p.hProcess,15000)!=WAIT_OBJECT_0){TerminateProcess(p.hProcess,124);CloseHandle(p.hProcess);return 124;}
    DWORD code=126;GetExitCodeProcess(p.hProcess,&code);CloseHandle(p.hProcess);return code;
}
static bool installed() {
    CreateDirectoryW(L"C:\\Program Files\\IS1 Fixture",nullptr);
    if(!CopyFileW(image().c_str(),L"C:\\Program Files\\IS1 Fixture\\application.exe",FALSE))return false;
    HKEY key{};if(RegCreateKeyExW(HKEY_LOCAL_MACHINE,L"Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\IS1Fixture",0,nullptr,0,KEY_SET_VALUE,nullptr,&key,nullptr)!=ERROR_SUCCESS)return false;
    const wchar_t location[]=L"C:\\Program Files\\IS1 Fixture";
    const wchar_t name[]=L"IS1 source-owned fixture";
    RegSetValueExW(key,L"InstallLocation",0,REG_SZ,reinterpret_cast<const BYTE*>(location),sizeof(location));
    RegSetValueExW(key,L"DisplayName",0,REG_SZ,reinterpret_cast<const BYTE*>(name),sizeof(name));
    RegFlushKey(key);RegCloseKey(key);return true;
}
int wmain(int argc,wchar_t** argv) {
    if(argc==5 && std::wstring(argv[1])==L"--child") {
        auto role=argv[2];DWORD code=wcstoul(argv[3],nullptr,10),delay=wcstoul(argv[4],nullptr,10);
        event("ready",role,0);
        if(std::wstring(role)==L"service_dependency"){
            SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CREATE_SERVICE);
            if(!scm){event("service_open_error",role,GetLastError());return 72;}
            SC_HANDLE service=CreateServiceW(scm,L"IS1MissingImage",L"IS1 fixture",SERVICE_START|DELETE,SERVICE_WIN32_OWN_PROCESS,SERVICE_DEMAND_START,SERVICE_ERROR_NORMAL,L"C:\\IS1-missing-service.exe",nullptr,nullptr,nullptr,nullptr,nullptr);
            DWORD e=0;if(!service)e=GetLastError();else {if(!StartServiceW(service,0,nullptr))e=GetLastError();DeleteService(service);CloseServiceHandle(service);}CloseServiceHandle(scm);
            event("service_start_result",role,e);return e?73:74;
        }
        Sleep(delay);
        if(std::wstring(role)==L"updater")code=join(spawn(L"relaunch",0,500));
        event("exit",role,code);return static_cast<int>(code);
    }
    if(argc==2 && std::wstring(argv[1])==L"--self-test")return join(spawn(L"payload",0,1))==0?0:1;
    // Fixed source-owned case mailbox: fixture input, never a manager launch flag.
    FILE* f=nullptr;_wfopen_s(&f,(image()+L".case").c_str(),L"rb");if(!f)return 120;
    char b[64]{};const size_t n=fread(b,1,sizeof(b)-1,f);fclose(f);std::string test(b,n);
    while(!test.empty()&&(test.back()=='\n'||test.back()=='\r'))test.pop_back();
    event("ready",L"bootstrapper",0);
    if(join(spawn(L"prerequisite",0,200))!=0)return 121;
    if(test=="success"){if(!installed())return 122;return static_cast<int>(join(spawn(L"payload",0,200)));}
    if(test=="payload_failure"){join(spawn(L"payload",37,200));return 2;}
    if(test=="service_failure"){installed();join(spawn(L"service_dependency",0,100));return 2;}
    if(test=="postlaunch_failure"){installed();join(spawn(L"postinstall_launch",41,200,L"C:\\Program Files\\IS1 Fixture\\application.exe"));return 2;}
    if(test=="outer_first"){auto p=spawn(L"payload",0,2000);if(p.hProcess)CloseHandle(p.hProcess);return 0;}
    if(test=="failure_while_alive"||test=="cancel_after_failure"){join(spawn(L"payload",39,200));event("holding",L"bootstrapper",0);Sleep(test=="cancel_after_failure"?180000:2000);return 2;}
    if(test=="short_lived"){join(spawn(L"payload",43,0));return 2;}
    if(test=="handoff"){auto p=spawn(L"updater",0,1200);if(p.hProcess)CloseHandle(p.hProcess);return 0;}
    if(test=="same_names"){auto a=spawn(L"payload",0,500);auto c=spawn(L"payload",47,700);join(a);join(c);return 2;}
    if(test=="noise"){std::fprintf(stderr,"Xalia System.InvalidOperationException: process already exited\n");installed();return 0;}
    if(test=="nothing_installed")return 0;
    return 123;
}
