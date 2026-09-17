// Source-owned renderer consumer. No vendor bytes, credentials, network or audio.
#include <windows.h>
#include <cstdio>
#include <string>
#include <array>

int wmain(int argc,wchar_t** argv) {
    if(argc==2 && std::wstring(argv[1])==L"--self-test")return 0;
    bool software=false;std::wstring role;
    for(int i=1;i<argc;++i){std::wstring a=argv[i];
        if(a==L"--disable-gpu")software=true;
        else if(a==L"--type=gpu-process"||a==L"--type=renderer"||a==L"--type=utility")role=a;
        else return 91;
    }
    wchar_t path[32768]{};if(!GetModuleFileNameW(nullptr,path,32768))return 92;
    FILE* config=_wfopen(L"case.txt",L"rb");char setting[32]{};
    if(!config)return 96;
    std::fread(setting,1,sizeof(setting)-1,config);std::fclose(config);
    if(std::string(setting)!="hold" && std::string(setting)!="differential")return 97;
    const bool hold=std::string(setting)=="hold";
    if(!role.empty()) {
        if(role==L"--type=gpu-process"&&!software){
            std::fprintf(stderr,"[%lu:1:0916/170000.123:ERROR:gpu_process_host.cc(12)] GPU process exited unexpectedly: exit_code=5\n",GetCurrentProcessId());std::fflush(stderr);
            return 5;
        }
        if(hold){Sleep(300000);}
        return 0;
    }
    std::fprintf(stdout,"NAUI2_READY_V1 %u\n",software?1u:0u);std::fflush(stdout);
    std::array<const wchar_t*,3> roles{L"--type=gpu-process",L"--type=renderer",L"--type=utility"};
    for(auto r:roles){
        std::wstring cmd=L"\""+std::wstring(path)+L"\" "+r+(software?L" --disable-gpu":L"");
        STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};
        if(!CreateProcessW(path,cmd.data(),nullptr,nullptr,FALSE,0,nullptr,nullptr,&si,&pi))return 93;
        CloseHandle(pi.hThread);
        if(WaitForSingleObject(pi.hProcess,hold?300000:10000)!=WAIT_OBJECT_0){CloseHandle(pi.hProcess);return 94;}
        DWORD code=0;if(!GetExitCodeProcess(pi.hProcess,&code)){CloseHandle(pi.hProcess);return 95;}
        CloseHandle(pi.hProcess);
    }
    std::fprintf(stdout,"NAUI2_EXIT_V1 0\n");std::fflush(stdout);return 0;
}
