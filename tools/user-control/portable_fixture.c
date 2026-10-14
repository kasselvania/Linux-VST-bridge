/* Source-owned portable Win32 program for installed user-control acceptance. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <wchar.h>

#ifndef FIXTURE_BUILD
#define FIXTURE_BUILD 1
#endif
#if FIXTURE_BUILD == 1
#define FIXTURE_VERSION L"A"
#elif FIXTURE_BUILD == 2
#define FIXTURE_VERSION L"B"
#else
#error FIXTURE_BUILD must be 1 (A) or 2 (B)
#endif

static wchar_t body[4096];
static HWND text_window;

static int read_resource(wchar_t *text, int capacity)
{
    wchar_t path[4096], *name;
    char bytes[2048];
    LARGE_INTEGER size;
    DWORD length = GetModuleFileNameW(NULL, path, 4096), received;
    HANDLE file;
    int result;
    if (!length || length >= 4096) return 0;
    name = wcsrchr(path, L'\\');
    if (!name || (size_t)(name - path) + 14 >= 4096) return 0;
    wcscpy(name + 1, L"resource.txt");
    file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                       FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) return 0;
    result = GetFileSizeEx(file, &size) && size.QuadPart > 0 &&
             size.QuadPart <= (LONGLONG)sizeof(bytes);
    if (result)
        result = ReadFile(file, bytes, (DWORD)size.QuadPart, &received, NULL) &&
                 received == (DWORD)size.QuadPart;
    CloseHandle(file);
    if (!result) return 0;
    result = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes,
                                 (int)received, text, capacity - 1);
    if (!result) return 0;
    text[result] = L'\0';
    return 1;
}

static LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wp, LPARAM lp)
{
    switch (message)
    {
    case WM_CREATE:
        text_window = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", body,
                         WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE |
                         ES_READONLY | ES_AUTOVSCROLL,
                         16, 16, 600, 300, hwnd, NULL, GetModuleHandleW(NULL), NULL);
        return text_window ? 0 : -1;
    case WM_SIZE:
        MoveWindow(text_window, 16, 16,
                   LOWORD(lp) > 32 ? LOWORD(lp) - 32 : 0,
                   HIWORD(lp) > 32 ? HIWORD(lp) - 32 : 0, TRUE);
        return 0;
    case WM_CLOSE:
        DestroyWindow(hwnd);
        return 0;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, message, wp, lp);
}

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, PWSTR args, int show)
{
    wchar_t resource[2049];
    WNDCLASSW window_class = {0};
    HWND window;
    MSG message;
    int result;
    (void)previous;
    if (!read_resource(resource, 2049))
    {
        MessageBoxW(NULL, L"Could not read UTF-8 resource.txt beside the executable.",
                    L"Portable program build " FIXTURE_VERSION, MB_OK | MB_ICONERROR);
        return 2;
    }
    if (swprintf(body, 4096, L"Portable program build %ls\r\n\r\nSibling resource:\r\n%ls"
                 L"\r\n\r\nArguments:\r\n%ls", FIXTURE_VERSION, resource, args) < 0)
        return 3;
    window_class.lpfnWndProc = window_proc;
    window_class.hInstance = instance;
    window_class.lpszClassName = L"LVBPortableProgramFixture";
    window_class.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
    window_class.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    if (!RegisterClassW(&window_class)) return 4;
    window = CreateWindowExW(0, window_class.lpszClassName,
                 L"Portable program build " FIXTURE_VERSION, WS_OVERLAPPEDWINDOW,
                 CW_USEDEFAULT, CW_USEDEFAULT, 680, 380, NULL, NULL, instance, NULL);
    if (!window) return 5;
    ShowWindow(window, show);
    while ((result = GetMessageW(&message, NULL, 0, 0)) > 0)
    {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    return result < 0 ? 6 : (int)message.wParam;
}
