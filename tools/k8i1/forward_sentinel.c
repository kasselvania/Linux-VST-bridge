#define WIN32_LEAN_AND_MEAN
#include <windows.h>

UINT WINAPI MsiGetFileVersionA(
    LPCSTR path, LPSTR version, LPDWORD version_size, LPSTR language, LPDWORD language_size)
{
    (void)path; (void)version; (void)version_size; (void)language; (void)language_size;
    return 4242u;
}

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved)
{
    (void)instance; (void)reason; (void)reserved;
    return TRUE;
}
