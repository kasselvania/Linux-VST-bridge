// Closed NAD1 SCM boundary; included after the existing exact-file hash owner.
// No caller-selected service, binary location, endpoint or installer arguments.
#include <winsvc.h>
#ifdef NAD1_GENERATED_SERVICE
static constexpr auto nad1_service=L"NAD1FixtureService";
static constexpr auto nad1_image=L"C:\\NAD1Fixture\\NTKDaemon.exe";
static constexpr auto nad1_installer=L"C:\\NAD1Fixture\\Setup.exe";
#else
static constexpr auto nad1_service=L"NTKDaemonService";
static constexpr auto nad1_image=L"C:\\Program Files\\Common Files\\Native Instruments\\NTK\\NTKDaemon.exe";
static constexpr auto nad1_installer=L"C:\\Program Files\\Native Instruments\\Native Access\\resources\\daemon\\win\\NTKDaemon 1.32.0 Setup PC.exe";
#endif
static int nad1_service_request(const std::vector<std::wstring>& r) {
    if(r.size()!=5 || r[0]!=L"NAD1_SERVICE_V1" || !hex(r[1],32) || !hex(r[2],64)
       || (r[3]!=L"query" && r[3]!=L"start" && r[3]!=L"stop" && r[3]!=L"install") || !hex(r[4],64)) return 140;
    if(r[3]==L"install") {
        HANDLE image=CreateFileW(nad1_installer,GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);
        if(image==INVALID_HANDLE_VALUE)return 141;
        std::string expected(r[4].begin(),r[4].end());bool exact=hash(image)==expected;
        if(!exact){CloseHandle(image);return 142;}
        std::wstring command=L"\"";command+=nad1_installer;command+=L"\" /s";
        STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};
        if(!CreateProcessW(nad1_installer,command.data(),nullptr,nullptr,FALSE,CREATE_SUSPENDED,nullptr,nullptr,&si,&pi)){CloseHandle(image);return 143;}
        std::array<wchar_t,32768> path{};DWORD length=static_cast<DWORD>(path.size());
        bool bound=QueryFullProcessImageNameW(pi.hProcess,0,path.data(),&length)!=FALSE;
        HANDLE actual=bound?CreateFileW(path.data(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr):INVALID_HANDLE_VALUE;
        bound=actual!=INVALID_HANDLE_VALUE&&same_file(image,actual);if(actual!=INVALID_HANDLE_VALUE)CloseHandle(actual);
        BY_HANDLE_FILE_INFORMATION info{};auto created=time_of(pi.hProcess);
        bound=bound&&created&&GetFileInformationByHandle(image,&info);
        if(!bound){TerminateProcess(pi.hProcess,151);WaitForSingleObject(pi.hProcess,5000);CloseHandle(image);CloseHandle(pi.hThread);CloseHandle(pi.hProcess);return 151;}
        auto size=(static_cast<unsigned long long>(info.nFileSizeHigh)<<32)|info.nFileSizeLow;
        std::fprintf(stdout,"NAD1_INSTALL_ROOT_V1 %ls %ls %s %llu %lu %llu\n",r[1].c_str(),r[2].c_str(),expected.c_str(),size,pi.dwProcessId,created);std::fflush(stdout);
        if(ResumeThread(pi.hThread)==static_cast<DWORD>(-1)){TerminateProcess(pi.hProcess,152);WaitForSingleObject(pi.hProcess,5000);CloseHandle(image);CloseHandle(pi.hThread);CloseHandle(pi.hProcess);return 152;}
        CloseHandle(image);CloseHandle(pi.hThread);DWORD code=144;
        if(WaitForSingleObject(pi.hProcess,180000)==WAIT_OBJECT_0)GetExitCodeProcess(pi.hProcess,&code);
        CloseHandle(pi.hProcess);
        std::fprintf(stdout,"NAD1_INSTALL_V1 %ls %ls %lu\n",r[1].c_str(),r[2].c_str(),code);std::fflush(stdout);
        return static_cast<int>(code);
    }
    SC_HANDLE scm=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CONNECT);if(!scm)return 145;
    DWORD access=SERVICE_QUERY_CONFIG|SERVICE_QUERY_STATUS;
    if(r[3]==L"start")access|=SERVICE_START;
    if(r[3]==L"stop")access|=SERVICE_STOP;
    SC_HANDLE service=OpenServiceW(scm,nad1_service,access);
    if(!service){DWORD error=GetLastError();CloseServiceHandle(scm);
        std::fprintf(stdout,"NAD1_SCM_V1 %ls %ls absent %lu 0 0 0 none 0 0\n",r[1].c_str(),r[2].c_str(),error);std::fflush(stdout);return error==ERROR_SERVICE_DOES_NOT_EXIST?0:146;}
    alignas(QUERY_SERVICE_CONFIGW) std::array<unsigned char,8192> buffer{};DWORD needed=0;
    bool ok=QueryServiceConfigW(service,reinterpret_cast<QUERY_SERVICE_CONFIGW*>(buffer.data()),static_cast<DWORD>(buffer.size()),&needed)!=FALSE;
    auto config=reinterpret_cast<QUERY_SERVICE_CONFIGW*>(buffer.data());
    std::wstring expected=nad1_image,quoted=L"\""+expected+L"\"";
    ok=ok&&config->dwServiceType==SERVICE_WIN32_OWN_PROCESS&&config->lpBinaryPathName
       && (expected==config->lpBinaryPathName || quoted==config->lpBinaryPathName);
    if(!ok){CloseServiceHandle(service);CloseServiceHandle(scm);return 147;}
    DWORD request_error=0;
    if(r[3]==L"start"&&!StartServiceW(service,0,nullptr)){request_error=GetLastError();if(request_error!=ERROR_SERVICE_ALREADY_RUNNING){CloseServiceHandle(service);CloseServiceHandle(scm);return 148;}}
    if(r[3]==L"stop"){SERVICE_STATUS status{};if(!ControlService(service,SERVICE_CONTROL_STOP,&status)){request_error=GetLastError();if(request_error!=ERROR_SERVICE_NOT_ACTIVE){CloseServiceHandle(service);CloseServiceHandle(scm);return 149;}}}
    SERVICE_STATUS_PROCESS status{};
    ok=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&status),sizeof(status),&needed)!=FALSE;
    std::string image_hash="none";unsigned long long created=0;
    if(ok&&status.dwCurrentState==SERVICE_RUNNING){
        HANDLE process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,status.dwProcessId);
        std::array<wchar_t,32768> path{};DWORD length=static_cast<DWORD>(path.size());
        ok=process&&QueryFullProcessImageNameW(process,0,path.data(),&length)&&_wcsicmp(path.data(),nad1_image)==0;
        if(ok){created=time_of(process);HANDLE file=CreateFileW(path.data(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);
            if(file!=INVALID_HANDLE_VALUE){image_hash=hash(file);CloseHandle(file);}else ok=false;}
        if(process)CloseHandle(process);
        if(!created||image_hash.size()!=64)ok=false;
    }
    if(ok)std::fprintf(stdout,"NAD1_SCM_V1 %ls %ls exact %lu %lu %lu %llu %s %lu %lu\n",r[1].c_str(),r[2].c_str(),request_error,status.dwCurrentState,status.dwProcessId,created,image_hash.c_str(),status.dwWin32ExitCode,status.dwServiceSpecificExitCode);
    std::fflush(stdout);
    // Hold the target runner while the exact SCM service is active. Container
    // stdin is not service-lifetime authority. Exact cgroup cleanup can retire
    // this bounded anchor; it never adopts or signals another service.
    if(ok && r[3]==L"start") {
        auto deadline=GetTickCount64()+600000;
        while(GetTickCount64()<deadline){SERVICE_STATUS_PROCESS current{};DWORD used=0;
            if(!QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&current),sizeof(current),&used)||current.dwCurrentState==SERVICE_STOPPED)break;
            Sleep(50);
        }
    }
    CloseServiceHandle(service);CloseServiceHandle(scm);return ok?0:150;
}
