#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <cstdio>
#include <string>
#include <vector>

static bool hex(const std::wstring &value, size_t count) {
    if (value.size() != count) return false;
    for (wchar_t c : value)
        if (!((c >= L'0' && c <= L'9') || (c >= L'a' && c <= L'f'))) return false;
    return true;
}

static bool decimal(const std::wstring &value, ULONGLONG &number) {
    if (value.empty()) return false;
    number = 0;
    for (wchar_t c : value) {
        if (c < L'0' || c > L'9') return false;
        ULONGLONG prior = number;
        number = number * 10 + static_cast<unsigned>(c - L'0');
        if (number < prior) return false;
    }
    return true;
}

static bool read_lines(const wchar_t *path, std::vector<std::wstring> &lines) {
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
                              FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    BY_HANDLE_FILE_INFORMATION info{};
    if (!GetFileInformationByHandle(file, &info) || info.nFileSizeHigh || !info.nFileSizeLow ||
        info.nFileSizeLow > 131072 || (info.nFileSizeLow & 1) ||
        (info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) ||
        info.nNumberOfLinks != 1) {
        CloseHandle(file); return false;
    }
    std::vector<wchar_t> data(info.nFileSizeLow / sizeof(wchar_t) + 1);
    DWORD read = 0;
    if (!ReadFile(file, data.data(), info.nFileSizeLow, &read, nullptr) || read != info.nFileSizeLow) {
        CloseHandle(file); return false;
    }
    CloseHandle(file); data[read / sizeof(wchar_t)] = 0;
    std::wstring content(data.data(), read / sizeof(wchar_t));
    size_t start = 0;
    while (start < content.size()) {
        size_t end = content.find(L'\n', start);
        if (end == std::wstring::npos) return false;
        std::wstring line = content.substr(start, end - start);
        if (!line.empty() && line.back() == L'\r') line.pop_back();
        lines.push_back(line); start = end + 1;
    }
    return true;
}

static DWORD watch_process(DWORD pid, ULONGLONG created_expected, const std::wstring &path_expected,
                           ULONGLONG size_expected, DWORD timeout_ms, DWORD &exit_code) {
    HANDLE process = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (!process) return GetLastError();
    FILETIME created_time{}, exited{}, kernel{}, user{};
    ULONGLONG created_value = 0;
    WCHAR path[32768]; DWORD length = 32768;
    if (!GetProcessTimes(process, &created_time, &exited, &kernel, &user) ||
        !QueryFullProcessImageNameW(process, 0, path, &length)) {
        DWORD error = GetLastError(); CloseHandle(process); return error ? error : ERROR_INVALID_DATA;
    }
    created_value = (static_cast<ULONGLONG>(created_time.dwHighDateTime) << 32) | created_time.dwLowDateTime;
    if (created_value != created_expected || _wcsicmp(path, path_expected.c_str()) != 0) {
        CloseHandle(process); return ERROR_INVALID_DATA;
    }
    HANDLE image = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
                               FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
    if (image == INVALID_HANDLE_VALUE) { DWORD error = GetLastError(); CloseHandle(process); return error; }
    BY_HANDLE_FILE_INFORMATION info{};
    ULONGLONG size = 0;
    bool identity = GetFileInformationByHandle(image, &info) &&
        !(info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) &&
        info.nNumberOfLinks == 1;
    size = (static_cast<ULONGLONG>(info.nFileSizeHigh) << 32) | info.nFileSizeLow;
    CloseHandle(image);
    if (!identity || size != size_expected) { CloseHandle(process); return ERROR_INVALID_DATA; }
    DWORD waited = WaitForSingleObject(process, timeout_ms);
    if (waited != WAIT_OBJECT_0) {
        DWORD error = waited == WAIT_TIMEOUT ? WAIT_TIMEOUT : GetLastError();
        CloseHandle(process); return error ? error : ERROR_INVALID_DATA;
    }
    if (!GetExitCodeProcess(process, &exit_code) || exit_code == STILL_ACTIVE) {
        DWORD error = GetLastError(); CloseHandle(process); return error ? error : ERROR_INVALID_DATA;
    }
    CloseHandle(process); return ERROR_SUCCESS;
}

static int self_test() {
    WCHAR system[MAX_PATH], command[MAX_PATH * 2];
    if (!GetSystemDirectoryW(system, MAX_PATH)) return 91;
    swprintf_s(command, L"\"%ls\\cmd.exe\" /c exit 23", system);
    STARTUPINFOW startup{}; startup.cb = sizeof(startup); PROCESS_INFORMATION child{};
    if (!CreateProcessW(nullptr, command, nullptr, nullptr, FALSE, CREATE_SUSPENDED, nullptr, nullptr,
                        &startup, &child)) return 92;
    FILETIME created{}, exited{}, kernel{}, user{};
    WCHAR path[32768]; DWORD length = 32768; BY_HANDLE_FILE_INFORMATION info{};
    if (!GetProcessTimes(child.hProcess, &created, &exited, &kernel, &user) ||
        !QueryFullProcessImageNameW(child.hProcess, 0, path, &length)) return 93;
    HANDLE image = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, 0, nullptr);
    if (image == INVALID_HANDLE_VALUE || !GetFileInformationByHandle(image, &info)) return 94;
    CloseHandle(image);
    ULONGLONG created_value = (static_cast<ULONGLONG>(created.dwHighDateTime) << 32) | created.dwLowDateTime;
    ULONGLONG size = (static_cast<ULONGLONG>(info.nFileSizeHigh) << 32) | info.nFileSizeLow;
    ResumeThread(child.hThread); CloseHandle(child.hThread);
    DWORD result = 0;
    DWORD error = watch_process(child.dwProcessId, created_value, path, size, 10000, result);
    CloseHandle(child.hProcess);
    return error == ERROR_SUCCESS && result == 23 ? 0 : 95;
}

int wmain(int argc, wchar_t **argv) {
    if (argc == 2 && !_wcsicmp(argv[1], L"--self-test")) return self_test();
    if (argc != 2) return 64;
    std::vector<std::wstring> lines;
    if (!read_lines(argv[1], lines) || lines.size() != 9 || lines[0] != L"K8I1_SETUP_WATCH_V1" ||
        !hex(lines[1], 32) || !hex(lines[2], 64)) return 65;
    ULONGLONG pid64, created, size, timeout;
    if (!decimal(lines[3], pid64) || !pid64 || pid64 > 0xffffffffULL ||
        !decimal(lines[4], created) || !created || !decimal(lines[6], size) || !size ||
        !decimal(lines[7], timeout) || timeout < 1000 || timeout > 7200000 || lines[5].empty() ||
        lines[8] != L"") return 66;
    DWORD exit_code = 0;
    DWORD error = watch_process(static_cast<DWORD>(pid64), created, lines[5], size,
                                static_cast<DWORD>(timeout), exit_code);
    std::printf("K8I1_SETUP_WATCH_RESULT_V1 %ls %ls %lu %I64u %lu %lu\n",
                lines[1].c_str(), lines[2].c_str(), static_cast<unsigned long>(pid64),
                created, static_cast<unsigned long>(error), static_cast<unsigned long>(exit_code));
    return error == ERROR_SUCCESS ? 0 : 67;
}
