/* Stand-in for an Electron-style single-instance desktop app, used by the
 * Native Access browser-return test (bridge-manager/tests/native_access_signin).
 * First-party fixture code: it contains and ships no vendor material.
 *
 * The bridge's bundled Native Access profile names an application, a bundled
 * service installer and the service image. The test installs this one program
 * under those names, and its role follows its own file name:
 *
 *   Native Access.exe               the app. The first copy takes the
 *                                   single-instance lock and listens. A later
 *                                   copy hands its command line to the first
 *                                   and exits, the way Electron's
 *                                   requestSingleInstanceLock does on Windows
 *                                   (Chromium's process singleton): a named
 *                                   startup mutex, a message-only window owned
 *                                   by the first copy, and one WM_COPYDATA
 *                                   "START\0<directory>\0<command line>\0" sent
 *                                   with SendMessageTimeout.
 *   NTKDaemon <v> Setup PC.exe /s   copies itself to the service image and
 *                                   registers the service.
 *   NTKDaemon.exe                   the service; listens on 127.0.0.1:5146/5563.
 *
 * What a later copy does is a fixture setting, the first line of
 * C:\LVBStandIn\mode: deliver | hang | refuse | crash | slow <seconds>.
 * The first copy records each return in C:\LVBStandIn\received.log as a
 * SHA-256 and a length, never the URL, so the test can search every file the
 * run wrote for the code without exception. */
#include <winsock2.h>
#include <windows.h>
#include <bcrypt.h>
#include <shellapi.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

#define FIXTURE L"C:\\LVBStandIn\\"
static const wchar_t SERVICE_NAME[] = L"NTKDaemonService";
static const wchar_t SERVICE_IMAGE[] =
    L"C:\\Program Files\\Common Files\\Native Instruments\\NTK\\NTKDaemon.exe";
static const wchar_t STARTUP_MUTEX[] = L"Local\\LVBStandInProcessSingletonStartup";
static const wchar_t WINDOW_CLASS[] = L"LVBStandIn_MessageWindow";
/* Chromium titles its message window with the profile directory. */
static const wchar_t WINDOW_TITLE[] = L"C:\\users\\steamuser\\AppData\\Roaming\\LVB Stand-in";
/* Chromium's process finder waits this long for the first copy to answer. */
static const UINT HANDOFF_TIMEOUT_MS = 20000;

static unsigned long long unix_ms(void) {
    FILETIME now;
    GetSystemTimePreciseAsFileTime(&now);
    ULARGE_INTEGER t;
    t.LowPart = now.dwLowDateTime;
    t.HighPart = now.dwHighDateTime;
    return (t.QuadPart - 116444736000000000ULL) / 10000ULL;
}

/* Append one complete line; the test reads whole lines only. */
static void record(const wchar_t* name, const char* line) {
    wchar_t path[MAX_PATH];
    swprintf(path, MAX_PATH, L"%ls%ls", FIXTURE, name);
    HANDLE file = CreateFileW(path, FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE,
                              NULL, OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) return;
    DWORD written = 0;
    WriteFile(file, line, (DWORD)strlen(line), &written, NULL);
    FlushFileBuffers(file);
    CloseHandle(file);
}

static BOOL sha256_hex(const BYTE* data, DWORD size, char out[65]) {
    BCRYPT_ALG_HANDLE algorithm = NULL;
    BCRYPT_HASH_HANDLE hash = NULL;
    UCHAR digest[32];
    BOOL ok = BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, NULL, 0) >= 0
        && BCryptCreateHash(algorithm, &hash, NULL, 0, NULL, 0, 0) >= 0
        && BCryptHashData(hash, (PUCHAR)data, size, 0) >= 0
        && BCryptFinishHash(hash, digest, sizeof(digest), 0) >= 0;
    if (hash) BCryptDestroyHash(hash);
    if (algorithm) BCryptCloseAlgorithmProvider(algorithm, 0);
    for (int i = 0; ok && i < 32; i++) sprintf(out + 2 * i, "%02x", digest[i]);
    return ok;
}

static const wchar_t* file_name(const wchar_t* path) {
    const wchar_t* slash = wcsrchr(path, L'\\');
    return slash ? slash + 1 : path;
}

/* ---- The app's first copy ---------------------------------------------- */

static unsigned returns_received = 0;

/* Electron's 'second-instance' event receives the later copy's arguments. The
 * stand-in checks them and keeps only a digest of the URL. */
static void receive(const COPYDATASTRUCT* data) {
    const wchar_t* text = (const wchar_t*)data->lpData;
    size_t length = data->cbData / sizeof(wchar_t);
    if (length < 7 || wmemcmp(text, L"START\0", 6) != 0) {
        record(L"received.log", "received malformed=1\n");
        return;
    }
    /* START \0 directory \0 command line \0 */
    const wchar_t* directory = text + 6;
    size_t used = 6 + wcsnlen(directory, length - 6) + 1;
    if (used >= length) {
        record(L"received.log", "received malformed=1\n");
        return;
    }
    const wchar_t* command = text + used;
    if (wcsnlen(command, length - used) == length - used) {
        record(L"received.log", "received malformed=1\n");
        return;
    }
    int argc = 0;
    wchar_t** argv = CommandLineToArgvW(command, &argc);
    int gpu = 0, urls = 0, other = 0;
    char digest[65] = "none";
    int url_bytes = 0;
    for (int i = 1; argv && i < argc; i++) {
        if (wcscmp(argv[i], L"--disable-gpu") == 0) {
            gpu++;
        } else if (wcsncmp(argv[i], L"native-access:", 14) == 0) {
            char url[4096];
            int n = WideCharToMultiByte(CP_UTF8, 0, argv[i], -1, url, sizeof(url), NULL, NULL);
            if (n > 1 && sha256_hex((const BYTE*)url, (DWORD)(n - 1), digest)) url_bytes = n - 1;
            SecureZeroMemory(url, sizeof(url));
            urls++;
        } else {
            other++;
        }
    }
    if (argv) {
        for (int i = 0; i < argc; i++) SecureZeroMemory(argv[i], wcslen(argv[i]) * sizeof(wchar_t));
        LocalFree(argv);
    }
    char line[256];
    snprintf(line, sizeof(line),
             "received n=%u args=%d gpu_flag=%d urls=%d other=%d url_sha256=%s url_bytes=%d at_ms=%llu\n",
             ++returns_received, argc, gpu, urls, other, digest, url_bytes, unix_ms());
    record(L"received.log", line);
}

static LRESULT CALLBACK message_window(HWND window, UINT message, WPARAM w, LPARAM l) {
    if (message == WM_COPYDATA) {
        receive((const COPYDATASTRUCT*)l);
        return TRUE;
    }
    return DefWindowProcW(window, message, w, l);
}

static int primary(HWND window) {
    char line[128];
    snprintf(line, sizeof(line), "primary ready pid=%lu at_ms=%llu\n", GetCurrentProcessId(), unix_ms());
    record(L"primary.log", line);
    MSG message;
    while (GetMessageW(&message, NULL, 0, 0) > 0) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    (void)window;
    return 0;
}

/* ---- A later copy ------------------------------------------------------ */

static LONG WINAPI end_without_debugger(EXCEPTION_POINTERS* exception) {
    (void)exception;
    return EXCEPTION_EXECUTE_HANDLER;
}

static int hand_over(HWND remote) {
    wchar_t directory[MAX_PATH] = L"";
    GetCurrentDirectoryW(MAX_PATH, directory);
    const wchar_t* command = GetCommandLineW();
    size_t a = wcslen(directory), b = wcslen(command);
    size_t length = 6 + a + 1 + b + 1;
    wchar_t* payload = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, length * sizeof(wchar_t));
    if (!payload) return 5;
    wmemcpy(payload, L"START\0", 6);
    wmemcpy(payload + 6, directory, a);
    wmemcpy(payload + 6 + a + 1, command, b);
    COPYDATASTRUCT data = {0, (DWORD)(length * sizeof(wchar_t)), payload};
    DWORD_PTR answer = 0;
    LRESULT sent = SendMessageTimeoutW(remote, WM_COPYDATA, 0, (LPARAM)&data, SMTO_ABORTIFHUNG,
                                       HANDOFF_TIMEOUT_MS, &answer);
    SecureZeroMemory(payload, length * sizeof(wchar_t));
    HeapFree(GetProcessHeap(), 0, payload);
    return sent && answer == TRUE ? 0 : 4;
}

static int secondary(HWND remote) {
    char mode[64] = "deliver";
    HANDLE setting = CreateFileW(FIXTURE L"mode", GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE,
                                 NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (setting != INVALID_HANDLE_VALUE) {
        DWORD n = 0;
        if (ReadFile(setting, mode, sizeof(mode) - 1, &n, NULL)) mode[n] = 0;
        CloseHandle(setting);
        mode[strcspn(mode, "\r\n")] = 0;
    }
    char line[160];
    snprintf(line, sizeof(line), "secondary start pid=%lu mode=%s at_ms=%llu\n",
             GetCurrentProcessId(), mode, unix_ms());
    record(L"secondary.log", line);
    int code;
    unsigned seconds = 0;
    if (strcmp(mode, "hang") == 0) {
        /* Never hands over and never exits; only the adapter's limit ends it. */
        Sleep(INFINITE);
        code = 9;
    } else if (strcmp(mode, "refuse") == 0) {
        code = 3;
    } else if (strcmp(mode, "crash") == 0) {
        /* An unhandled fault, ended without a debugger as Windows would. */
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
        SetUnhandledExceptionFilter(end_without_debugger);
        RaiseException(EXCEPTION_ACCESS_VIOLATION, EXCEPTION_NONCONTINUABLE, 0, NULL);
        code = 8;
    } else if (sscanf(mode, "slow %u", &seconds) == 1) {
        Sleep(seconds * 1000);
        code = hand_over(remote);
    } else {
        code = hand_over(remote);
    }
    snprintf(line, sizeof(line), "secondary end pid=%lu code=%d at_ms=%llu\n",
             GetCurrentProcessId(), code, unix_ms());
    record(L"secondary.log", line);
    return code;
}

static int application(HINSTANCE instance) {
    HWND remote = FindWindowExW(HWND_MESSAGE, NULL, WINDOW_CLASS, WINDOW_TITLE);
    if (!remote) {
        /* Serialize startup as Chromium does: ask for the mutex without
         * initial ownership, wait for it, then look again before claiming. */
        HANDLE startup = CreateMutexW(NULL, FALSE, STARTUP_MUTEX);
        if (!startup) return 6;
        WaitForSingleObject(startup, INFINITE);
        remote = FindWindowExW(HWND_MESSAGE, NULL, WINDOW_CLASS, WINDOW_TITLE);
        HWND own = NULL;
        if (!remote) {
            WNDCLASSW type = {0};
            type.lpfnWndProc = message_window;
            type.hInstance = instance;
            type.lpszClassName = WINDOW_CLASS;
            if (RegisterClassW(&type))
                own = CreateWindowExW(0, WINDOW_CLASS, WINDOW_TITLE, 0, 0, 0, 0, 0, HWND_MESSAGE,
                                      NULL, instance, NULL);
        }
        ReleaseMutex(startup);
        if (own) return primary(own);
        if (!remote) return 7;
    }
    return secondary(remote);
}

/* ---- The bundled service ---------------------------------------------- */

static SERVICE_STATUS_HANDLE service_handle;
static SERVICE_STATUS service_status;
static HANDLE stop_requested;

static void publish(DWORD state) {
    service_status.dwServiceType = SERVICE_WIN32_OWN_PROCESS;
    service_status.dwCurrentState = state;
    service_status.dwControlsAccepted = state == SERVICE_RUNNING ? SERVICE_ACCEPT_STOP : 0;
    SetServiceStatus(service_handle, &service_status);
}

static DWORD WINAPI service_control(DWORD control, DWORD type, LPVOID data, LPVOID context) {
    (void)type; (void)data; (void)context;
    if (control == SERVICE_CONTROL_STOP) {
        publish(SERVICE_STOP_PENDING);
        SetEvent(stop_requested);
    }
    return NO_ERROR;
}

static void WINAPI service_main(DWORD argc, LPWSTR* argv) {
    (void)argc; (void)argv;
    service_handle = RegisterServiceCtrlHandlerExW(SERVICE_NAME, service_control, NULL);
    if (!service_handle) return;
    stop_requested = CreateEventW(NULL, TRUE, FALSE, NULL);
    publish(SERVICE_START_PENDING);
    WSADATA wsa;
    SOCKET listeners[2] = {INVALID_SOCKET, INVALID_SOCKET};
    const unsigned short ports[2] = {5146, 5563};
    BOOL ready = WSAStartup(MAKEWORD(2, 2), &wsa) == 0;
    for (int i = 0; ready && i < 2; i++) {
        SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
        struct sockaddr_in address = {0};
        address.sin_family = AF_INET;
        address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
        address.sin_port = htons(ports[i]);
        ready = s != INVALID_SOCKET && bind(s, (struct sockaddr*)&address, sizeof(address)) == 0
            && listen(s, 4) == 0;
        if (s != INVALID_SOCKET) listeners[i] = s;
    }
    if (ready) {
        publish(SERVICE_RUNNING);
        WaitForSingleObject(stop_requested, INFINITE);
    } else {
        service_status.dwWin32ExitCode = ERROR_SERVICE_SPECIFIC_ERROR;
        service_status.dwServiceSpecificExitCode = 2;
    }
    for (int i = 0; i < 2; i++)
        if (listeners[i] != INVALID_SOCKET) closesocket(listeners[i]);
    WSACleanup();
    publish(SERVICE_STOPPED);
}

static int service(void) {
    SERVICE_TABLE_ENTRYW table[] = {{(LPWSTR)SERVICE_NAME, service_main}, {NULL, NULL}};
    return StartServiceCtrlDispatcherW(table) ? 0 : (int)GetLastError();
}

static int install_service(void) {
    wchar_t own[MAX_PATH];
    if (!GetModuleFileNameW(NULL, own, MAX_PATH)) return 10;
    CreateDirectoryW(L"C:\\Program Files\\Common Files\\Native Instruments", NULL);
    CreateDirectoryW(L"C:\\Program Files\\Common Files\\Native Instruments\\NTK", NULL);
    if (!CopyFileW(own, SERVICE_IMAGE, FALSE)) return 11;
    SC_HANDLE manager = OpenSCManagerW(NULL, NULL, SC_MANAGER_CREATE_SERVICE);
    if (!manager) return 12;
    wchar_t command[MAX_PATH + 2];
    swprintf(command, MAX_PATH + 2, L"\"%ls\"", SERVICE_IMAGE);
    SC_HANDLE created = CreateServiceW(manager, SERVICE_NAME, L"LVB stand-in service", SERVICE_ALL_ACCESS,
                                       SERVICE_WIN32_OWN_PROCESS, SERVICE_DEMAND_START,
                                       SERVICE_ERROR_NORMAL, command, NULL, NULL, NULL, NULL, NULL);
    DWORD error = created ? 0 : GetLastError();
    if (created) CloseServiceHandle(created);
    CloseServiceHandle(manager);
    return error == 0 || error == ERROR_SERVICE_EXISTS ? 0 : 13;
}

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, PWSTR arguments, int show) {
    (void)previous; (void)arguments; (void)show;
    wchar_t own[MAX_PATH];
    if (!GetModuleFileNameW(NULL, own, MAX_PATH)) return 60;
    const wchar_t* name = file_name(own);
    int argc = 0;
    wchar_t** argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    BOOL silent = argv && argc == 2 && wcscmp(argv[1], L"/s") == 0;
    if (argv) LocalFree(argv);
    if (_wcsicmp(name, L"Native Access.exe") == 0) return application(instance);
    if (_wcsicmp(name, L"NTKDaemon.exe") == 0) return service();
    size_t n = wcslen(name);
    if (silent && _wcsnicmp(name, L"NTKDaemon ", 10) == 0 && n > 13
        && _wcsicmp(name + n - 13, L" Setup PC.exe") == 0)
        return install_service();
    return 64;
}
