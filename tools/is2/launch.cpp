// Source-owned installer launch observer. No SDK/audio/vendor-name behavior.
// CreateProcessW retains its normal manifest refusal; no runas/elevation override.
#include <windows.h>
#include <bcrypt.h>
#include <cstdio>
#include <string>
#include <vector>
#include <array>
#include <cstdint>

static std::string hash(HANDLE f) {
    BCRYPT_ALG_HANDLE alg{}; BCRYPT_HASH_HANDLE state{}; DWORD size=0,n=0;
    if(BCryptOpenAlgorithmProvider(&alg,BCRYPT_SHA256_ALGORITHM,nullptr,0)<0)return {};
    if(BCryptGetProperty(alg,BCRYPT_OBJECT_LENGTH,reinterpret_cast<PUCHAR>(&size),sizeof(size),&n,0)<0){BCryptCloseAlgorithmProvider(alg,0);return {};}
    std::vector<unsigned char> object(size);std::array<unsigned char,32> result{};
    bool ok=BCryptCreateHash(alg,&state,object.data(),size,nullptr,0,0)>=0;
    std::array<unsigned char,65536> bytes{};DWORD read=0;LARGE_INTEGER zero{};
    if(!SetFilePointerEx(f,zero,nullptr,FILE_BEGIN))ok=false;
    while(ok){if(!ReadFile(f,bytes.data(),static_cast<DWORD>(bytes.size()),&read,nullptr)){ok=false;break;}if(!read)break;ok=BCryptHashData(state,bytes.data(),read,0)>=0;}
    if(ok)ok=BCryptFinishHash(state,result.data(),static_cast<ULONG>(result.size()),0)>=0;
    if(state)BCryptDestroyHash(state);BCryptCloseAlgorithmProvider(alg,0);
    if(!ok)return {};std::string text;const char hex[]="0123456789abcdef";
    for(auto b:result){text+=hex[b>>4];text+=hex[b&15];}return text;
}
static bool hex(const std::wstring& s,size_t length){if(s.size()!=length)return false;for(auto c:s)if(!((c>=L'0'&&c<=L'9')||(c>=L'a'&&c<=L'f')))return false;return true;}
static unsigned long long time_of(HANDLE p){FILETIME c{},e{},k{},u{};if(!GetProcessTimes(p,&c,&e,&k,&u))return 0;return (static_cast<unsigned long long>(c.dwHighDateTime)<<32)|c.dwLowDateTime;}
static bool same_file(HANDLE a,HANDLE b){BY_HANDLE_FILE_INFORMATION x{},y{};return GetFileInformationByHandle(a,&x)&&GetFileInformationByHandle(b,&y)&&x.dwVolumeSerialNumber==y.dwVolumeSerialNumber&&x.nFileIndexHigh==y.nFileIndexHigh&&x.nFileIndexLow==y.nFileIndexLow;}
int wmain(int argc,wchar_t** argv){
    if(argc==2 && std::wstring(argv[1])==L"--self-test")return hex(L"0123456789abcdef",16)&&!hex(L"g",1)?0:1;
    if(argc!=2)return 120;
    HANDLE request=CreateFileW(argv[1],GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);
    if(request==INVALID_HANDLE_VALUE)return 121;
    BY_HANDLE_FILE_INFORMATION metadata{};std::array<wchar_t,8192> buffer{};DWORD bytes=0;
    bool ok=GetFileInformationByHandle(request,&metadata)&&!(metadata.dwFileAttributes&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT))&&metadata.nFileSizeHigh==0&&metadata.nFileSizeLow<sizeof(buffer)&&metadata.nFileSizeLow%2==0;
    if(ok)ok=ReadFile(request,buffer.data(),metadata.nFileSizeLow,&bytes,nullptr)&&bytes==metadata.nFileSizeLow;
    CloseHandle(request);if(!ok)return 122;
    std::wstring all(buffer.data(),bytes/2);std::vector<std::wstring> lines;size_t pos=0;
    for(;;){auto end=all.find(L'\n',pos);if(end==std::wstring::npos)break;lines.push_back(all.substr(pos,end-pos));pos=end+1;}
    if(pos!=all.size()||lines.size()!=7||lines[0]!=L"IS2_LAUNCH_V1"||!hex(lines[1],32)||!hex(lines[2],64)||lines[3]!=L"2"||!hex(lines[4],64))return 123;
    wchar_t* end=nullptr;auto expected_size=wcstoull(lines[5].c_str(),&end,10);
    if(!end||*end||!expected_size||expected_size>2147483648ULL||lines[6].empty()||lines[6].find(L'"')!=std::wstring::npos)return 124;
    // Hold the exact image read-only without write/delete sharing through creation.
    HANDLE image=CreateFileW(lines[6].c_str(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);
    if(image==INVALID_HANDLE_VALUE)return 125;
    BY_HANDLE_FILE_INFORMATION info{};ok=GetFileInformationByHandle(image,&info)&&!(info.dwFileAttributes&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT))&&((static_cast<unsigned long long>(info.nFileSizeHigh)<<32)|info.nFileSizeLow)==expected_size;
    std::string expected(lines[4].begin(),lines[4].end());if(ok)ok=hash(image)==expected;
    if(!ok){CloseHandle(image);return 126;}
    STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};
    std::wstring command=L"\""+lines[6]+L"\"";
    // Environment and current directory are inherited unchanged. No shell, extra
    // installer arguments, token in child environment, or manifest bypass.
    if(!CreateProcessW(lines[6].c_str(),command.data(),nullptr,nullptr,FALSE,CREATE_SUSPENDED,nullptr,nullptr,&si,&pi)){
        DWORD error=GetLastError();std::fprintf(stderr,"IS2_REFUSED_V1 %ls %ls 2 %lu\n",lines[1].c_str(),lines[2].c_str(),error);std::fflush(stderr);CloseHandle(image);return 127;
    }
    std::array<wchar_t,32768> actual{};DWORD length=static_cast<DWORD>(actual.size());
    ok=QueryFullProcessImageNameW(pi.hProcess,0,actual.data(),&length)!=FALSE;
    HANDLE opened=ok?CreateFileW(actual.data(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr):INVALID_HANDLE_VALUE;
    ok=opened!=INVALID_HANDLE_VALUE&&same_file(image,opened);if(opened!=INVALID_HANDLE_VALUE)CloseHandle(opened);
    auto created=time_of(pi.hProcess);auto own_created=time_of(GetCurrentProcess());
    if(!ok||!created||!own_created){TerminateProcess(pi.hProcess,128);WaitForSingleObject(pi.hProcess,5000);CloseHandle(pi.hThread);CloseHandle(pi.hProcess);CloseHandle(image);return 128;}
    // Same inherited stderr pipe as Wine's process trace. This complete bounded
    // frame precedes ResumeThread and therefore all target-driven child activity.
    char frame[512]{};int n=std::snprintf(frame,sizeof(frame),"IS2_ROOT_V1 %ls %ls 2 %s %llu %lu %llu %lu %llu\n",lines[1].c_str(),lines[2].c_str(),expected.c_str(),expected_size,pi.dwProcessId,created,GetCurrentProcessId(),own_created);
    DWORD written=0;ok=n>0&&n<static_cast<int>(sizeof(frame))&&WriteFile(GetStdHandle(STD_ERROR_HANDLE),frame,static_cast<DWORD>(n),&written,nullptr)&&written==static_cast<DWORD>(n);
    if(!ok||ResumeThread(pi.hThread)==static_cast<DWORD>(-1)){TerminateProcess(pi.hProcess,129);WaitForSingleObject(pi.hProcess,5000);CloseHandle(pi.hThread);CloseHandle(pi.hProcess);CloseHandle(image);return 129;}
    CloseHandle(pi.hThread);CloseHandle(image);
    DWORD result=130;if(WaitForSingleObject(pi.hProcess,3600000)==WAIT_OBJECT_0)GetExitCodeProcess(pi.hProcess,&result);
    CloseHandle(pi.hProcess);return static_cast<int>(result);
}
