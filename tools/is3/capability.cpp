// Source-owned capability consumer; no vendor scripts or product policy.
#include <windows.h>
#include <bcrypt.h>
#include <algorithm>
#include <array>
#include <cstdio>
#include <string>
#include <vector>

struct Result { bool launched; bool exited; DWORD status; };
static std::wstring image() {
    wchar_t value[32768]{};
    DWORD n = GetModuleFileNameW(nullptr, value, 32768);
    return {value, n};
}
static std::wstring key(const std::wstring& entry) {
    auto split = entry.find(L'=', (!entry.empty() && entry[0]==L'=') ? 1 : 0);
    return split == std::wstring::npos ? L"" : entry.substr(0, split);
}
static int compare(const std::wstring& a, const std::wstring& b) {
    return CompareStringOrdinal(a.c_str(), -1, b.c_str(), -1, TRUE) - CSTR_EQUAL;
}
static bool valid(const std::vector<wchar_t>& block) {
    if(block.size()<2 || block.back()!=0 || block[block.size()-2]!=0)return false;
    if(block.size()==2)return block[0]==0;
    size_t at=0; std::wstring previous;
    while(at<block.size()-1) {
        size_t end=at;while(end<block.size() && block[end])++end;
        if(end==at || end>=block.size()-1)return false;
        std::wstring entry(block.data()+at,end-at), name=key(entry);
        if(name.empty() || (!previous.empty() && compare(previous,name)>=0))return false;
        if(name[0]==L'=' && !(name.size()==3 && name[2]==L':' &&
            ((name[1]>=L'A' && name[1]<=L'Z') || (name[1]>=L'a' && name[1]<=L'z'))))return false;
        previous=name;at=end+1;
    }
    return at==block.size()-1;
}
static std::vector<std::wstring> inherited() {
    wchar_t* env=GetEnvironmentStringsW();std::vector<std::wstring> result;
    if(!env)return result;
    for(const wchar_t* p=env;*p;p+=wcslen(p)+1)result.emplace_back(p);
    FreeEnvironmentStringsW(env);return result;
}
static std::vector<wchar_t> canonical(std::vector<std::wstring> entries, bool inject) {
    std::sort(entries.begin(),entries.end(),[](const auto& a,const auto& b){return compare(key(a),key(b))<0;});
    std::wstring previous;bool found=false;
    for(auto& entry:entries) {
        auto name=key(entry);
        if(name.empty() || (!previous.empty() && compare(previous,name)==0))return {};
        previous=name;
        if(compare(name,L"WINEDLLOVERRIDES")==0) {
            found=true;
            if(inject){auto value=entry.substr(name.size()+1);entry=L"WINEDLLOVERRIDES="+value+(value.empty()?L"":L";")+L"powershell.exe=";}
        }
    }
    if(inject && !found)entries.emplace_back(L"WINEDLLOVERRIDES=powershell.exe=");
    std::sort(entries.begin(),entries.end(),[](const auto& a,const auto& b){return compare(key(a),key(b))<0;});
    std::vector<wchar_t> out;
    for(const auto& entry:entries){out.insert(out.end(),entry.begin(),entry.end());out.push_back(0);}
    out.push_back(0);if(entries.empty())out.push_back(0);
    return valid(out)?out:std::vector<wchar_t>{};
}
static std::string sha(const std::wstring& value) {
    BCRYPT_ALG_HANDLE alg{};BCRYPT_HASH_HANDLE hash{};DWORD n=0,size=0;
    if(BCryptOpenAlgorithmProvider(&alg,BCRYPT_SHA256_ALGORITHM,nullptr,0)<0)return {};
    bool ok=BCryptGetProperty(alg,BCRYPT_OBJECT_LENGTH,reinterpret_cast<PUCHAR>(&size),sizeof(size),&n,0)>=0;
    std::vector<unsigned char> object(size);std::array<unsigned char,32> bytes{};
    if(ok)ok=BCryptCreateHash(alg,&hash,object.data(),size,nullptr,0,0)>=0;
    if(ok)ok=BCryptHashData(hash,reinterpret_cast<PUCHAR>(const_cast<wchar_t*>(value.data())),static_cast<ULONG>(value.size()*2),0)>=0;
    if(ok)ok=BCryptFinishHash(hash,bytes.data(),32,0)>=0;
    if(hash)BCryptDestroyHash(hash);BCryptCloseAlgorithmProvider(alg,0);
    std::string result;if(!ok)return result;
    for(auto c:bytes){result+="0123456789abcdef"[c>>4];result+="0123456789abcdef"[c&15];}return result;
}
static bool receipt(const char* mode,const char* origin,const std::vector<std::wstring>& entries) {
    unsigned count=0;std::wstring value;
    for(const auto& e:entries)if(compare(key(e),L"WINEDLLOVERRIDES")==0){++count;value=e.substr(key(e).size()+1);}
    auto hash=sha(value);if(hash.size()!=64)return false;
    std::printf("IS3_ENV_V1 mode=%s origin=%s present=%u length=%zu sha256=%s duplicate_count=%u\n",mode,origin,unsigned(count!=0),value.size(),hash.c_str(),count?count-1:0);
    std::fflush(stdout);return count<=1;
}
static std::vector<std::wstring> unpack(const std::vector<wchar_t>& block) {
    std::vector<std::wstring> rows;
    if(!valid(block))return rows;
    for(const wchar_t* p=block.data();*p;p+=wcslen(p)+1)rows.emplace_back(p);
    return rows;
}
static std::vector<wchar_t> unavailable_environment() {return canonical(inherited(),true);}
static Result execute(const std::wstring& exe, const std::wstring& args, bool unavailable) {
    auto command = L"\"" + exe + L"\" " + args;
    auto env = unavailable ? unavailable_environment() : std::vector<wchar_t>{};
    if (unavailable && env.empty()) return {false, false, ERROR_NOT_ENOUGH_MEMORY};
    STARTUPINFOW startup{}; startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(exe.c_str(), command.data(), nullptr, nullptr, FALSE,
                        CREATE_UNICODE_ENVIRONMENT, unavailable ? env.data() : nullptr,
                        nullptr, &startup, &process)) return {false, false, GetLastError()};
    CloseHandle(process.hThread);
    DWORD wait = WaitForSingleObject(process.hProcess, 10000), status = 0;
    bool exited = wait == WAIT_OBJECT_0 && GetExitCodeProcess(process.hProcess, &status);
    if (!exited) {
        // Only the exact handle we created is eligible for timeout cleanup.
        TerminateProcess(process.hProcess, 252);
        WaitForSingleObject(process.hProcess, 5000);
        status = 252;
    }
    CloseHandle(process.hProcess);
    return {true, exited, status};
}
static void emit(const char* mode, const char* stage, Result r, bool side_effect) {
    std::printf("IS3_CAP_V1 mode=%s stage=%s launched=%u exited=%u status=%lu side_effect=%u tick=%llu\n",
                mode, stage, unsigned(r.launched), unsigned(r.exited), r.status,
                unsigned(side_effect), GetTickCount64());
    std::fflush(stdout);
}
int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wstring(argv[1]) == L"--fallback") return 43;
    if(argc==3 && std::wstring(argv[1])==L"--environment-receipt") {
        std::wstring mode=argv[2];if(mode!=L"baseline" && mode!=L"unavailable" && mode!=L"restored")return 244;
        return receipt(std::string(mode.begin(),mode.end()).c_str(),"child",inherited())?0:245;
    }
    if (argc == 2 && std::wstring(argv[1]) == L"--self-test") {
        auto env=canonical({L"z=2",L"=C:=C:\\owned",L"WineDllOverrides=n,b",L"a=1"},true);
        if(!valid(env))return 1;
        auto rows=unpack(env);
        if(rows.size()!=4 || rows[0]!=L"=C:=C:\\owned" || rows[2]!=L"WINEDLLOVERRIDES=n,b;powershell.exe=")return 2;
        if(!canonical({L"WINEDLLOVERRIDES=x",L"winedlloverrides=y"},true).empty())return 3;
        if(!canonical({L"bad"},true).empty() || !canonical({L"=invalid=x"},false).empty())return 4;
        auto broken=env;broken.push_back(0);if(valid(broken))return 5;
        broken.pop_back();broken.pop_back();if(valid(broken))return 6;
        std::vector<wchar_t> unsorted={L'z',L'=',L'1',0,L'a',L'=',L'2',0,0};if(valid(unsorted))return 7;
        if(sha(L"")!="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")return 8;
        return 0;
    }
    auto self=execute(image(),L"--self-test",false);
    if(!self.launched || !self.exited || self.status!=0)return 249;
    wchar_t sys[32768]{};
    if (!GetSystemDirectoryW(sys, 32768)) return 240;
    std::wstring ps = std::wstring(sys) + L"\\WindowsPowerShell\\v1.0\\powershell.exe";
    const wchar_t* sentinel = L"C:\\IS3-source-owned-sentinel.txt";
    if (GetFileAttributesW(sentinel) != INVALID_FILE_ATTRIBUTES) return 241;
    for (const char* mode : {"baseline", "unavailable", "restored"}) {
        bool unavailable = std::string(mode) == "unavailable";
        auto block=canonical(inherited(),unavailable);if(block.empty())return 246;
        if(!receipt(mode,"expected",unpack(block)))return 247;
        std::wstring wide(mode,mode+strlen(mode));
        auto delivered=execute(image(),L"--environment-receipt "+wide,unavailable);
        if(!delivered.launched || !delivered.exited || delivered.status!=0)return 248;
        auto side = execute(ps, L"-C \"[IO.File]::WriteAllText('C:\\IS3-source-owned-sentinel.txt','source_owned'); exit 37\"", unavailable);
        bool wrote = GetFileAttributesW(sentinel) != INVALID_FILE_ATTRIBUTES;
        emit(mode, "side_effect", side, wrote);
        if (wrote && !DeleteFileW(sentinel)) return 242;
        auto capability = execute(ps, L"-C \"if (Get-Command Get-CimInstance -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }\"", unavailable);
        emit(mode, "capability", capability, false);
        auto empty = execute(ps, L"-C \"if ((Get-CimInstance Win32_Process | Where-Object {$_.ExecutablePath -eq 'C:\\IS3-known-absent.exe'}).Count -eq 0) { exit 1 } else { exit 0 }\"", unavailable);
        emit(mode, "empty_query", empty, false);
        // This is the source-owned consumer's declared fallback contract.
        // It is not evidence that a proprietary installer selected its fallback.
        bool use_fallback = !capability.launched || (capability.exited && capability.status != 0);
        if (use_fallback) emit(mode, "fallback", execute(image(), L"--fallback", false), false);
        else emit(mode, "fallback", {false, false, 0}, false);
    }
    return 0;
}
