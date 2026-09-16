// Source-owned capability consumer; no vendor scripts or product policy.
#include <windows.h>
#include <cstdio>
#include <string>
#include <vector>

struct Result { bool launched; bool exited; DWORD status; };
static std::wstring image() {
    wchar_t value[32768]{};
    DWORD n = GetModuleFileNameW(nullptr, value, 32768);
    return {value, n};
}
static std::vector<wchar_t> unavailable_environment() {
    // Preserve the parent environment. This block is passed only to one child;
    // no prefix registry, global environment, interpreter or runner is modified.
    wchar_t* env = GetEnvironmentStringsW();
    if (!env) return {};
    std::vector<wchar_t> out;
    std::wstring overrides;
    for (const wchar_t* p = env; *p; p += wcslen(p) + 1) {
        if (!_wcsnicmp(p, L"WINEDLLOVERRIDES=", 17)) overrides = p + 17;
        else out.insert(out.end(), p, p + wcslen(p) + 1);
    }
    FreeEnvironmentStringsW(env);
    if (!overrides.empty()) overrides += L";";
    auto setting = L"WINEDLLOVERRIDES=" + overrides + L"powershell.exe=";
    out.insert(out.end(), setting.begin(), setting.end());
    out.push_back(0); out.push_back(0);
    return out;
}
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
    if (argc == 2 && std::wstring(argv[1]) == L"--self-test") {
        auto env = unavailable_environment();
        return env.size() > 2 && env.back() == 0 && env[env.size()-2] == 0 ? 0 : 1;
    }
    wchar_t sys[32768]{};
    if (!GetSystemDirectoryW(sys, 32768)) return 240;
    std::wstring ps = std::wstring(sys) + L"\\WindowsPowerShell\\v1.0\\powershell.exe";
    const wchar_t* sentinel = L"C:\\IS3-source-owned-sentinel.txt";
    if (GetFileAttributesW(sentinel) != INVALID_FILE_ATTRIBUTES) return 241;
    for (const char* mode : {"baseline", "unavailable", "restored"}) {
        bool unavailable = std::string(mode) == "unavailable";
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
