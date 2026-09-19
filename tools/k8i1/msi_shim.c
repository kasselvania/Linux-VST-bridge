/*
 * K8I1 exact Kontakt 8 MSI diversion shim.
 *
 * This 32-bit DLL is selected only by Wine's per-application override for
 * "Kontakt 8 Setup PC.exe".  The generated .def file forwards every pinned
 * Wine MSI export except MsiInstallProductA/W to msi_lvb_real.dll.  These two
 * entry points publish one operation-bound request and wait for one exact
 * result.  There is deliberately no command launcher, shell, network client,
 * registry mutation, or deployment policy in this DLL.
 */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#define K8I1_INSTALL_FAILURE 1603u
#define K8I1_MAX_CONFIG 32768u
#define K8I1_MAX_REQUEST 196608u
#define K8I1_MAX_RESULT 1024u

static const WCHAR k_config_name[] = L"k8i1.private";
static const WCHAR k_setup_name[] = L"Kontakt 8 Setup PC.exe";
static const WCHAR k_package_name[] = L"Kontakt 8 Setup PC.msi";
static const char k_identity[] = "linux-vst-bridge-k8i1-msi-shim/1";

struct Config {
    WCHAR operation[33];
    WCHAR nonce[65];
    WCHAR setup_sha256[65];
    ULONGLONG setup_size;
    WCHAR package_sha256[65];
    WCHAR plan_sha256[65];
    ULONGLONG package_size;
    WCHAR request[MAX_PATH * 4];
    WCHAR result[MAX_PATH * 4];
    DWORD timeout_ms;
};

static int is_hex(const WCHAR *value, DWORD length) {
    DWORD i;
    if (!value || lstrlenW(value) != (int)length) return 0;
    for (i = 0; i < length; ++i) {
        WCHAR c = value[i];
        if (!((c >= L'0' && c <= L'9') || (c >= L'a' && c <= L'f'))) return 0;
    }
    return 1;
}

static int parse_u64(const WCHAR *text, ULONGLONG *out) {
    ULONGLONG value = 0;
    if (!text || !*text) return 0;
    while (*text) {
        ULONGLONG prior = value;
        if (*text < L'0' || *text > L'9') return 0;
        value = value * 10u + (ULONGLONG)(*text++ - L'0');
        if (value < prior) return 0;
    }
    *out = value;
    return 1;
}

static WCHAR *next_line(WCHAR **cursor) {
    WCHAR *line = *cursor;
    WCHAR *end;
    if (!line || !*line) return NULL;
    end = line;
    while (*end && *end != L'\n') ++end;
    if (!*end) return NULL;
    if (end > line && end[-1] == L'\r') end[-1] = 0;
    *end = 0;
    *cursor = end + 1;
    return line;
}

static int same_basename(const WCHAR *path, const WCHAR *expected) {
    const WCHAR *name = path;
    const WCHAR *p;
    if (!path || !*path) return 0;
    for (p = path; *p; ++p) if (*p == L'\\' || *p == L'/') name = p + 1;
    return lstrcmpiW(name, expected) == 0;
}

static int exact_plain_file(const WCHAR *path, ULONGLONG size) {
    HANDLE file;
    BY_HANDLE_FILE_INFORMATION info;
    ULONGLONG actual;
    file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                       FILE_FLAG_OPEN_REPARSE_POINT, NULL);
    if (file == INVALID_HANDLE_VALUE) return 0;
    if (!GetFileInformationByHandle(file, &info) ||
        (info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) ||
        info.nNumberOfLinks != 1) {
        CloseHandle(file);
        return 0;
    }
    actual = ((ULONGLONG)info.nFileSizeHigh << 32) | info.nFileSizeLow;
    CloseHandle(file);
    return actual == size;
}

static int config_path(WCHAR *out, DWORD capacity) {
    HMODULE self;
    DWORD length;
    WCHAR *p;
    self = NULL;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                            GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                            (LPCWSTR)&config_path, &self)) return 0;
    length = GetModuleFileNameW(self, out, capacity);
    if (!length || length >= capacity) return 0;
    p = out + length;
    while (p > out && p[-1] != L'\\' && p[-1] != L'/') --p;
    if ((DWORD)(p - out) + (DWORD)(sizeof(k_config_name) / sizeof(WCHAR)) > capacity) return 0;
    lstrcpyW(p, k_config_name);
    return 1;
}

static int read_config(struct Config *cfg) {
    WCHAR path[MAX_PATH * 4];
    WCHAR content[K8I1_MAX_CONFIG / sizeof(WCHAR)];
    WCHAR *cursor;
    WCHAR *line[11];
    HANDLE file;
    BY_HANDLE_FILE_INFORMATION info;
    DWORD bytes = 0;
    DWORD i;
    ULONGLONG timeout;
    if (!config_path(path, (DWORD)(sizeof(path) / sizeof(path[0])))) return 0;
    file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                       FILE_FLAG_OPEN_REPARSE_POINT, NULL);
    if (file == INVALID_HANDLE_VALUE) return 0;
    if (!GetFileInformationByHandle(file, &info) ||
        (info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) ||
        info.nNumberOfLinks != 1 || info.nFileSizeHigh || !info.nFileSizeLow ||
        info.nFileSizeLow >= K8I1_MAX_CONFIG || (info.nFileSizeLow & 1u) ||
        !ReadFile(file, content, info.nFileSizeLow, &bytes, NULL) || bytes != info.nFileSizeLow) {
        CloseHandle(file);
        return 0;
    }
    CloseHandle(file);
    content[bytes / sizeof(WCHAR)] = 0;
    cursor = content;
    for (i = 0; i < 11; ++i) if (!(line[i] = next_line(&cursor))) return 0;
    if (*cursor || lstrcmpW(line[0], L"K8I1_CONFIG_V1") ||
        !is_hex(line[1], 32) || !is_hex(line[2], 64) || !is_hex(line[3], 64) ||
        !parse_u64(line[4], &cfg->setup_size) || !is_hex(line[5], 64) ||
        !parse_u64(line[6], &cfg->package_size) || !*line[7] || !*line[8] ||
        !is_hex(line[9], 64) || !parse_u64(line[10], &timeout) || timeout < 1000 || timeout > 7200000u) return 0;
    if (lstrlenW(line[7]) >= (int)(sizeof(cfg->request) / sizeof(WCHAR)) ||
        lstrlenW(line[8]) >= (int)(sizeof(cfg->result) / sizeof(WCHAR))) return 0;
    lstrcpyW(cfg->operation, line[1]); lstrcpyW(cfg->nonce, line[2]);
    lstrcpyW(cfg->setup_sha256, line[3]); lstrcpyW(cfg->package_sha256, line[5]);
    lstrcpyW(cfg->plan_sha256, line[9]);
    lstrcpyW(cfg->request, line[7]); lstrcpyW(cfg->result, line[8]);
    cfg->timeout_ms = (DWORD)timeout;
    return 1;
}

static int current_setup_matches(const struct Config *cfg) {
    WCHAR path[MAX_PATH * 4];
    DWORD length = GetModuleFileNameW(NULL, path, (DWORD)(sizeof(path) / sizeof(path[0])));
    return length && length < (DWORD)(sizeof(path) / sizeof(path[0])) &&
           same_basename(path, k_setup_name) && exact_plain_file(path, cfg->setup_size);
}

static int append_utf16(WCHAR *out, DWORD capacity, DWORD *used, const WCHAR *value) {
    DWORD length = (DWORD)lstrlenW(value);
    if (*used + length + 1 >= capacity) return 0;
    CopyMemory(out + *used, value, length * sizeof(WCHAR));
    *used += length; out[(*used)++] = L'\n'; out[*used] = 0;
    return 1;
}

static int append_u64(WCHAR *out, DWORD capacity, DWORD *used, ULONGLONG value) {
    WCHAR digits[32];
    DWORD count = 0, i;
    if (!value) digits[count++] = L'0';
    while (value && count < 31) { digits[count++] = (WCHAR)(L'0' + value % 10); value /= 10; }
    for (i = 0; i < count / 2; ++i) { WCHAR c = digits[i]; digits[i] = digits[count-i-1]; digits[count-i-1] = c; }
    digits[count] = 0;
    return append_utf16(out, capacity, used, digits);
}

static int append_hex_utf16(WCHAR *out, DWORD capacity, DWORD *used, const WCHAR *value) {
    static const WCHAR digits[] = L"0123456789abcdef";
    DWORD i, length = value ? (DWORD)lstrlenW(value) : 0;
    if (*used + length * 4 + 1 >= capacity) return 0;
    for (i = 0; i < length; ++i) {
        unsigned v = (unsigned)value[i];
        out[(*used)++] = digits[(v >> 12) & 15]; out[(*used)++] = digits[(v >> 8) & 15];
        out[(*used)++] = digits[(v >> 4) & 15]; out[(*used)++] = digits[v & 15];
    }
    out[(*used)++] = L'\n'; out[*used] = 0;
    return 1;
}

static UINT publish_and_wait(const struct Config *cfg, const WCHAR *package,
                             const WCHAR *properties, WCHAR call_kind) {
    WCHAR request[K8I1_MAX_REQUEST / sizeof(WCHAR)];
    WCHAR result[K8I1_MAX_RESULT / sizeof(WCHAR)];
    WCHAR call[2] = {call_kind, 0};
    WCHAR setup_path[MAX_PATH * 4];
    FILETIME created, exited, kernel, user;
    ULONGLONG created_value;
    DWORD used = 0, written = 0, bytes = 0, waited = 0;
    HANDLE file;
    if (!GetModuleFileNameW(NULL, setup_path, (DWORD)(sizeof(setup_path)/sizeof(setup_path[0]))) ||
        !GetProcessTimes(GetCurrentProcess(), &created, &exited, &kernel, &user))
        return K8I1_INSTALL_FAILURE;
    created_value = ((ULONGLONG)created.dwHighDateTime << 32) | created.dwLowDateTime;
    request[0] = 0;
    if (!append_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, L"K8I1_REQUEST_V1") ||
        !append_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, cfg->operation) ||
        !append_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, cfg->nonce) ||
        !append_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, L"1") ||
        !append_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, call) ||
        !append_u64(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, GetCurrentProcessId()) ||
        !append_u64(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, created_value) ||
        !append_hex_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, setup_path) ||
        !append_hex_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, package) ||
        !append_hex_utf16(request, (DWORD)(sizeof(request)/sizeof(request[0])), &used, properties ? properties : L""))
        return K8I1_INSTALL_FAILURE;
    file = CreateFileW(cfg->request, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_NEW,
                       FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, NULL);
    if (file == INVALID_HANDLE_VALUE) return K8I1_INSTALL_FAILURE;
    bytes = used * sizeof(WCHAR);
    if (!WriteFile(file, request, bytes, &written, NULL) || written != bytes || !FlushFileBuffers(file)) {
        CloseHandle(file); return K8I1_INSTALL_FAILURE;
    }
    CloseHandle(file);
    for (;;) {
        file = CreateFileW(cfg->result, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                           FILE_FLAG_OPEN_REPARSE_POINT, NULL);
        if (file != INVALID_HANDLE_VALUE) break;
        if (waited >= cfg->timeout_ms) return K8I1_INSTALL_FAILURE;
        Sleep(100); waited += 100;
    }
    {
        BY_HANDLE_FILE_INFORMATION info;
        if (!GetFileInformationByHandle(file, &info) ||
            (info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)) ||
            info.nNumberOfLinks != 1 || info.nFileSizeHigh || info.nFileSizeLow >= sizeof(result) ||
            (info.nFileSizeLow & 1u)) { CloseHandle(file); return K8I1_INSTALL_FAILURE; }
    }
    if (!ReadFile(file, result, sizeof(result)-sizeof(WCHAR), &bytes, NULL) ||
        bytes >= sizeof(result) || (bytes & 1u)) { CloseHandle(file); return K8I1_INSTALL_FAILURE; }
    CloseHandle(file); result[bytes/sizeof(WCHAR)] = 0;
    {
        WCHAR *cursor = result;
        WCHAR *line[7];
        DWORD i;
        for (i=0; i<7; ++i) if (!(line[i]=next_line(&cursor))) return K8I1_INSTALL_FAILURE;
        if (*cursor || lstrcmpW(line[0],L"K8I1_RESULT_V1") || lstrcmpW(line[1],cfg->operation) ||
            lstrcmpW(line[2],cfg->nonce) || lstrcmpW(line[3],L"1") || lstrcmpW(line[4],L"verified") ||
            lstrcmpW(line[5],cfg->plan_sha256) || !is_hex(line[6],64)) return K8I1_INSTALL_FAILURE;
    }
    return ERROR_SUCCESS;
}

static UINT intercept_w(const WCHAR *package, const WCHAR *properties, WCHAR kind) {
    struct Config cfg;
    UINT result;
    if (!read_config(&cfg) || !current_setup_matches(&cfg) || !package ||
        !same_basename(package, k_package_name) || !exact_plain_file(package, cfg.package_size))
        ExitProcess(K8I1_INSTALL_FAILURE);
    result = publish_and_wait(&cfg, package, properties, kind);
    if (result != ERROR_SUCCESS) ExitProcess(K8I1_INSTALL_FAILURE);
    return ERROR_SUCCESS;
}

__declspec(dllexport) UINT WINAPI MsiInstallProductW(LPCWSTR package, LPCWSTR command_line) {
    return intercept_w(package, command_line, L'W');
}

__declspec(dllexport) UINT WINAPI MsiInstallProductA(LPCSTR package, LPCSTR command_line) {
    WCHAR wide_package[MAX_PATH * 4];
    WCHAR wide_command[32768];
    int p, c;
    if (!package) ExitProcess(K8I1_INSTALL_FAILURE);
    p = MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS, package, -1, wide_package,
                            (int)(sizeof(wide_package)/sizeof(wide_package[0])));
    c = command_line ? MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS, command_line, -1,
                                            wide_command, (int)(sizeof(wide_command)/sizeof(wide_command[0]))) : 1;
    if (!p || !c) ExitProcess(K8I1_INSTALL_FAILURE);
    if (!command_line) wide_command[0] = 0;
    return intercept_w(wide_package, wide_command, L'A');
}

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reason; (void)reserved; (void)k_identity;
    return TRUE;
}
