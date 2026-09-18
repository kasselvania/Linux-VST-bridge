// Closed NAD1 SCM boundary; included after the existing exact-file hash owner.
// No caller-selected service, binary location, endpoint or installer arguments.
#include <winsvc.h>
#include <iphlpapi.h>
#include <cstddef>
#include "nad1_stop.h"
#ifdef NAD1_GENERATED_SERVICE
static constexpr auto nad1_service=L"NAD1FixtureService";
static constexpr auto nad1_image=L"C:\\NAD1Fixture\\NTKDaemon.exe";
static constexpr auto nad1_installer=L"C:\\NAD1Fixture\\Setup.exe";
#else
static constexpr auto nad1_service=L"NTKDaemonService";
static constexpr auto nad1_image=L"C:\\Program Files\\Common Files\\Native Instruments\\NTK\\NTKDaemon.exe";
static constexpr auto nad1_installer=L"C:\\Program Files\\Native Instruments\\Native Access\\resources\\daemon\\win\\NTKDaemon 1.32.0 Setup PC.exe";
#endif
// IP Helper assigns endpoint ownership in the Windows PID domain. Never join
// those PIDs to Linux IDs; the Linux census independently supplies cgroup custody.
static DWORD nad1_endpoints(DWORD pid) {
    HMODULE dll=LoadLibraryExW(L"iphlpapi.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);if(!dll)return 4;
    using Query=DWORD (WINAPI*)(PVOID,PDWORD,BOOL,ULONG,TCP_TABLE_CLASS,ULONG);
    auto query=reinterpret_cast<Query>(GetProcAddress(dll,"GetExtendedTcpTable"));
    std::vector<unsigned char> bytes(2*1024*1024);DWORD length=static_cast<DWORD>(bytes.size());DWORD mask=0;
    if(!query||query(bytes.data(),&length,FALSE,AF_INET,TCP_TABLE_OWNER_PID_LISTENER,0)!=NO_ERROR||length>bytes.size()||length<offsetof(MIB_TCPTABLE_OWNER_PID,table)){FreeLibrary(dll);return 4;}
    auto table=reinterpret_cast<MIB_TCPTABLE_OWNER_PID*>(bytes.data());
    if(table->dwNumEntries>32768||offsetof(MIB_TCPTABLE_OWNER_PID,table)+table->dwNumEntries*sizeof(MIB_TCPROW_OWNER_PID)>length){FreeLibrary(dll);return 4;}
    for(DWORD i=0;i<table->dwNumEntries;i++){const auto& row=table->table[i];
        DWORD port=((row.dwLocalPort&255)<<8)|((row.dwLocalPort>>8)&255);
        if(row.dwOwningPid==pid&&row.dwState==MIB_TCP_STATE_LISTEN&&row.dwLocalAddr==0x0100007f){if(port==5146)mask|=1;if(port==5563)mask|=2;}}
    FreeLibrary(dll);return mask;
}
// Private, non-replacing handoff from the original retained process handle.
// This receipt is created only after that handle signals and its listeners retire.
// The operation nonce and exact Windows generation prevent cross-operation reuse.
static std::wstring nad1_exit_path(const std::wstring& request,const std::vector<std::wstring>& r,DWORD pid,unsigned long long created){
    std::array<wchar_t,32768> full{};DWORD n=GetFullPathNameW(request.c_str(),static_cast<DWORD>(full.size()),full.data(),nullptr);
    if(!n||n>=full.size())return {};
    std::wstring path(full.data(),n);auto slash=path.find_last_of(L"\\/");if(slash==std::wstring::npos)return {};
    return path.substr(0,slash+1)+r[1]+L"-"+r[2]+L"-"+std::to_wstring(pid)+L"-"+std::to_wstring(created)+L"-anchor-exit.private";
}
static std::string nad1_exit_bytes(const std::vector<std::wstring>& r,DWORD pid,unsigned long long created){
    return "NAD1_ANCHOR_EXIT_V1 "+std::string(r[1].begin(),r[1].end())+" "+std::string(r[2].begin(),r[2].end())+" "+std::to_string(pid)+" "+std::to_string(created)+" 0 0\n";
}
static bool nad1_exit_publish(const std::wstring& request,const std::vector<std::wstring>& r,DWORD pid,unsigned long long created){
    auto path=nad1_exit_path(request,r,pid,created);if(path.empty()||!pid||!created)return false;
    auto temporary=path+L".writing";
    HANDLE file=CreateFileW(temporary.c_str(),GENERIC_WRITE,0,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);
    if(file==INVALID_HANDLE_VALUE)return false;
    auto bytes=nad1_exit_bytes(r,pid,created);DWORD written=0;
    bool ok=WriteFile(file,bytes.data(),static_cast<DWORD>(bytes.size()),&written,nullptr)&&written==bytes.size()&&FlushFileBuffers(file);
    CloseHandle(file);return ok&&MoveFileExW(temporary.c_str(),path.c_str(),MOVEFILE_WRITE_THROUGH);
}
// Missing is distinct from malformed/inaccessible. Neither proves retirement.
static int nad1_exit_read(const std::wstring& request,const std::vector<std::wstring>& r,DWORD pid,unsigned long long created){
    auto path=nad1_exit_path(request,r,pid,created);if(path.empty())return -1;
    HANDLE file=CreateFileW(path.c_str(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);
    if(file==INVALID_HANDLE_VALUE)return GetLastError()==ERROR_FILE_NOT_FOUND?0:-1;
    BY_HANDLE_FILE_INFORMATION info{};std::array<char,512> bytes{};DWORD read=0;auto expected=nad1_exit_bytes(r,pid,created);
    bool ok=GetFileInformationByHandle(file,&info)&&!(info.dwFileAttributes&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT))&&info.nNumberOfLinks==1&&info.nFileSizeHigh==0&&info.nFileSizeLow==expected.size()
        &&ReadFile(file,bytes.data(),static_cast<DWORD>(bytes.size()),&read,nullptr)&&read==expected.size()&&std::string(bytes.data(),read)==expected;
    CloseHandle(file);return ok?1:-1;
}
static int nad1_service_request(const std::vector<std::wstring>& r,const std::wstring& request_path) {
    const bool stop_v2=r.size()==7&&r[0]==L"NAD1_STOP_REQUEST_V2"&&r[3]==L"stop";
    if((!stop_v2&&(r.size()!=5 || r[0]!=L"NAD1_SERVICE_V1" || r[3]==L"stop")) || !hex(r[1],32) || !hex(r[2],64)
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
        std::fprintf(stdout,"NAD1_SCM_V1 %ls %ls absent %lu 0 0 0 none 0 0 0\n",r[1].c_str(),r[2].c_str(),error);std::fflush(stdout);return error==ERROR_SERVICE_DOES_NOT_EXIST?0:146;}
    alignas(QUERY_SERVICE_CONFIGW) std::array<unsigned char,8192> buffer{};DWORD needed=0;
    bool ok=QueryServiceConfigW(service,reinterpret_cast<QUERY_SERVICE_CONFIGW*>(buffer.data()),static_cast<DWORD>(buffer.size()),&needed)!=FALSE;
    auto config=reinterpret_cast<QUERY_SERVICE_CONFIGW*>(buffer.data());
    std::wstring expected=nad1_image,quoted=L"\""+expected+L"\"";
    ok=ok&&config->dwServiceType==SERVICE_WIN32_OWN_PROCESS&&config->lpBinaryPathName
       && (expected==config->lpBinaryPathName || quoted==config->lpBinaryPathName);
    if(!ok){CloseServiceHandle(service);CloseServiceHandle(scm);return 147;}
    // Only a prior RUNNING observation may supply a pending service generation.
    // Request material remains private and cannot be selected by an operator.
    if(r[3]==L"stop") {
        auto number=[](const std::wstring& text,unsigned long long& value){
            if(text.empty()||text.size()>20)return false;value=0;
            for(auto c:text){if(c<L'0'||c>L'9')return false;auto d=static_cast<unsigned>(c-L'0');
                if(value>(0xffffffffffffffffULL-d)/10)return false;value=value*10+d;}return true;};
        unsigned long long prior_pid=0,prior_created=0;
        if(!number(r[5],prior_pid)||!number(r[6],prior_created)||prior_pid>0xffffffffULL||(prior_pid==0)!=(prior_created==0)){CloseServiceHandle(service);CloseServiceHandle(scm);return 140;}
        SERVICE_STATUS_PROCESS initial{};DWORD count=0;
        bool queried=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&initial),sizeof(initial),&count)!=FALSE;
        DWORD first_error=queried?0:GetLastError();HANDLE process=nullptr;DWORD pid=0;unsigned long long created=0;
        bool valid=queried;DWORD identity_error=0;bool exit_witness=false;
        if(queried&&initial.dwCurrentState==SERVICE_RUNNING){
            pid=initial.dwProcessId;
            if(prior_pid&&pid!=prior_pid){valid=false;identity_error=ERROR_INVALID_DATA;}
        }else if(queried&&prior_pid)pid=static_cast<DWORD>(prior_pid);
        else if(queried&&initial.dwCurrentState!=SERVICE_STOPPED){valid=false;identity_error=ERROR_INVALID_DATA;}
        if(valid&&pid&&initial.dwCurrentState==SERVICE_STOPPED&&prior_created){
            const int witness=nad1_exit_read(request_path,r,pid,prior_created);
            if(witness<0){valid=false;identity_error=ERROR_INVALID_DATA;}
            else if(witness==1){exit_witness=true;created=prior_created;}
        }
        if(valid&&pid&&!exit_witness){
            process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,pid);
            if(!process)identity_error=GetLastError();
            std::array<wchar_t,32768> path{};DWORD length=static_cast<DWORD>(path.size());
            valid=process&&QueryFullProcessImageNameW(process,0,path.data(),&length)&&_wcsicmp(path.data(),nad1_image)==0;
            if(valid){created=time_of(process);valid=created!=0&&(!prior_created||created==prior_created);}
            if(!valid&&!identity_error)identity_error=ERROR_INVALID_DATA;
        }
        struct IO {
            SC_HANDLE service;HANDLE process;DWORD pid;SERVICE_STATUS_PROCESS first;DWORD first_error;bool initial=true;
            const std::vector<std::wstring>& request;
            uint64_t now(){return GetTickCount64();}
            Nad1StopStatus query(){
                SERVICE_STATUS_PROCESS st{};DWORD count=0,error=0;
                if(initial){st=first;error=first_error;initial=false;}
                else if(!QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&st),sizeof(st),&count))error=GetLastError();
                return {st.dwCurrentState,st.dwCheckPoint,st.dwWaitHint,st.dwWin32ExitCode,st.dwServiceSpecificExitCode,error};
            }
            void begin_control(uint32_t state){std::fprintf(stdout,"NAD1_STOP_BEGIN_V1 %ls %ls %u\n",request[1].c_str(),request[2].c_str(),state);std::fflush(stdout);}
            uint32_t stop(){SERVICE_STATUS st{};return ControlService(service,SERVICE_CONTROL_STOP,&st)?0:GetLastError();}
            uint32_t wait(uint32_t& error){DWORD result=process?WaitForSingleObject(process,0):WAIT_OBJECT_0;error=result==WAIT_FAILED?GetLastError():0;return result;}
            uint32_t endpoints(){return pid?nad1_endpoints(pid):0;}
            void sleep(){Sleep(50);}
        } io{service,process,pid,initial,first_error,true,r};
        auto result=nad1_observe_stop(io,valid);
        std::fprintf(stdout,"NAD1_STOP_OBSERVATION_V2 %ls %ls %u %u %u %u %u %u %u %u %u %u %u %u %llu %llu %llu %u %u %u %u %u\n",
            r[1].c_str(),r[2].c_str(),result.confirmed?1u:0u,result.before.state,result.before.error,result.last.state,result.last.error,
            result.last.checkpoint,result.last.hint,result.process_wait,result.wait_error,result.endpoints,result.control_sent?1u:0u,result.control_error,
            static_cast<unsigned long long>(result.control_started),static_cast<unsigned long long>(result.control_elapsed),static_cast<unsigned long long>(result.elapsed),
            result.queries,result.progress,static_cast<unsigned>(identity_error),result.last.win32,result.last.specific);
        if(exit_witness)std::fprintf(stdout,"NAD1_EXIT_WITNESS_V1 %ls %ls %lu %llu\n",r[1].c_str(),r[2].c_str(),pid,created);
        std::fprintf(stdout,"NAD1_RETIRE_V1 %ls %ls %u %lu %llu %u\n",r[1].c_str(),r[2].c_str(),result.confirmed?1u:0u,pid,created,result.endpoints);
        if(result.confirmed)std::fprintf(stdout,"NAD1_SCM_V1 %ls %ls exact %u 1 0 0 none %u %u 0\n",r[1].c_str(),r[2].c_str(),result.control_error,result.last.win32,result.last.specific);
        std::fflush(stdout);if(process)CloseHandle(process);CloseServiceHandle(service);CloseServiceHandle(scm);return result.confirmed?0:149;
    }
    DWORD request_error=0;
    if(r[3]==L"start"&&!StartServiceW(service,0,nullptr)){request_error=GetLastError();if(request_error!=ERROR_SERVICE_ALREADY_RUNNING){CloseServiceHandle(service);CloseServiceHandle(scm);return 148;}}
    SERVICE_STATUS_PROCESS status{};
    ok=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&status),sizeof(status),&needed)!=FALSE;
    // Startup acknowledgment must not outrun ownership of the running handle.
    // This bounds startup only; the healthy operation anchor has no deadline.
    if(ok&&r[3]==L"start"){
        const auto startup_deadline=GetTickCount64()+25000;
        while(ok&&status.dwCurrentState==SERVICE_START_PENDING&&GetTickCount64()<startup_deadline){
            Sleep(50);ok=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&status),sizeof(status),&needed)!=FALSE;
        }
        ok=ok&&status.dwCurrentState==SERVICE_RUNNING;
    }
    std::string image_hash="none";unsigned long long created=0;DWORD endpoints=0;HANDLE start_anchor=nullptr;
    if(ok&&status.dwCurrentState==SERVICE_RUNNING){
        HANDLE process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|(r[3]==L"start"?SYNCHRONIZE:0),FALSE,status.dwProcessId);
        std::array<wchar_t,32768> path{};DWORD length=static_cast<DWORD>(path.size());
        ok=process&&QueryFullProcessImageNameW(process,0,path.data(),&length)&&_wcsicmp(path.data(),nad1_image)==0;
        if(ok){created=time_of(process);HANDLE file=CreateFileW(path.data(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);
            if(file!=INVALID_HANDLE_VALUE){image_hash=hash(file);CloseHandle(file);}else ok=false;}
        if(ok){endpoints=nad1_endpoints(status.dwProcessId);SERVICE_STATUS_PROCESS after{};DWORD count=0,exit=0;
            ok=QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&after),sizeof(after),&count)&&after.dwProcessId==status.dwProcessId&&after.dwCurrentState==SERVICE_RUNNING&&GetExitCodeProcess(process,&exit)&&exit==STILL_ACTIVE&&time_of(process)==created;}
        if(!created||image_hash.size()!=64)ok=false;
        if(ok&&r[3]==L"start")start_anchor=process;
        else if(process)CloseHandle(process);
    }
    if(ok)std::fprintf(stdout,"NAD1_SCM_V1 %ls %ls exact %lu %lu %lu %llu %s %lu %lu %lu\n",r[1].c_str(),r[2].c_str(),request_error,status.dwCurrentState,status.dwCurrentState==SERVICE_RUNNING?status.dwProcessId:0,created,image_hash.c_str(),status.dwWin32ExitCode,status.dwServiceSpecificExitCode,endpoints);
    std::fflush(stdout);
    // Normal use has no wall-clock deadline. The owner bounds cancellation and
    // cleanup separately. Retain one service generation; never adopt a restart.
    if(ok && r[3]==L"start") {
        HANDLE anchor=start_anchor;DWORD anchor_pid=anchor?status.dwProcessId:0;unsigned long long anchor_created=anchor?created:0;
        while(ok){SERVICE_STATUS_PROCESS current{};DWORD used=0;
            if(!QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<LPBYTE>(&current),sizeof(current),&used)){ok=false;break;}
            if(current.dwCurrentState==SERVICE_STOPPED){
                // Publish before releasing the original handle. A vanished PID
                // must never be mistaken for this generation's exit observation.
                ok=anchor&&WaitForSingleObject(anchor,12000)==WAIT_OBJECT_0&&time_of(anchor)==anchor_created
                    &&nad1_endpoints(anchor_pid)==0&&nad1_exit_publish(request_path,r,anchor_pid,anchor_created);
                break;
            }
            if(current.dwCurrentState==SERVICE_RUNNING){
                ok=anchor&&current.dwProcessId==anchor_pid&&time_of(anchor)==anchor_created&&WaitForSingleObject(anchor,0)==WAIT_TIMEOUT;
            }else if(current.dwCurrentState!=SERVICE_START_PENDING&&current.dwCurrentState!=SERVICE_STOP_PENDING)ok=false;
            if(ok)Sleep(50);
        }
        if(anchor)CloseHandle(anchor);
    }
    CloseServiceHandle(service);CloseServiceHandle(scm);return ok?0:150;
}
