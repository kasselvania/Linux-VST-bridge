/* Source-owned x86 fixture for the exact K8I1 interception boundary. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

typedef UINT (WINAPI *install_w)(LPCWSTR, LPCWSTR);
typedef UINT (WINAPI *install_a)(LPCSTR, LPCSTR);

static int sibling(WCHAR *path, DWORD capacity, const WCHAR *name)
{
    DWORD length = GetModuleFileNameW(NULL, path, capacity);
    WCHAR *cursor;
    if (!length || length >= capacity) return 0;
    cursor = path + length;
    while (cursor > path && cursor[-1] != L'\\' && cursor[-1] != L'/') --cursor;
    if ((DWORD)(cursor - path) + lstrlenW(name) + 1 > capacity) return 0;
    lstrcpyW(cursor, name);
    return 1;
}

int wmain(int argc, WCHAR **argv)
{
    WCHAR dll_path[32768];
    HMODULE module;
    if (argc != 3 || (lstrcmpW(argv[1], L"A") && lstrcmpW(argv[1], L"W"))) return 110;
    if (!sibling(dll_path, 32768, L"msi.dll")) return 111;
    module = LoadLibraryW(dll_path);
    if (!module) return 112;
    if (!lstrcmpW(argv[1], L"W"))
    {
        install_w call = (install_w)GetProcAddress(module, "MsiInstallProductW");
        if (!call) return 113;
        return (int)call(argv[2], L"K8I1_GENERATED=1");
    }
    else
    {
        char package[32768];
        install_a call = (install_a)GetProcAddress(module, "MsiInstallProductA");
        if (!call || !WideCharToMultiByte(CP_ACP, WC_NO_BEST_FIT_CHARS, argv[2], -1,
                                          package, sizeof(package), NULL, NULL)) return 114;
        return (int)call(package, "K8I1_GENERATED=1");
    }
}
