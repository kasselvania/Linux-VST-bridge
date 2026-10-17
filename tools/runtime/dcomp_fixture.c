/* Original account-free DirectComposition creation discriminator.
 * Copyright 2026 LVB contributors. SPDX-License-Identifier: MIT
 * Creation does not prove composition, pixel output or vendor editor support.
 */
#define COBJMACROS
#ifndef UNICODE
#define UNICODE
#endif
#include <windows.h>
#include <objbase.h>
#include <stdio.h>
#include <wchar.h>

typedef HRESULT (WINAPI *create_device)(void *, REFIID, void **);
static const IID unknown_id = {0,0,0,{0xc0,0,0,0,0,0,0,0x46}};

int wmain(int argc, WCHAR **argv)
{
    const char *names[] = {"DCompositionCreateDevice", "DCompositionCreateDevice2", "DCompositionCreateDevice3"};
    WCHAR path[MAX_PATH];
    HMODULE library;
    DWORD length;
    unsigned int n;
    int failed = 0;
    HRESULT hr;
    if (argc > 2 || (argc == 2 && wcscmp(argv[1], L"--wait-for-readback"))) return 2;
    hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    if (FAILED(hr)) return 2;
    library = LoadLibraryW(L"dcomp.dll");
    if (!library) { CoUninitialize(); return 2; }
    length = GetModuleFileNameW(library, path, MAX_PATH);
    if (!length || length >= MAX_PATH) { FreeLibrary(library); CoUninitialize(); return 2; }
    printf("Loaded DComp module: %ls; PE pointer bits: %u\n", path, (unsigned int)(sizeof(void *) * 8));
    fflush(stdout);
    if (argc == 2) Sleep(5000);
    for (n = 0; n < 3; ++n) {
        union { FARPROC address; create_device create; } call;
        IUnknown *device = NULL;
        call.address = GetProcAddress(library, names[n]);
        if (!call.address) { printf("%s: missing export\n", names[n]); failed = 1; continue; }
        hr = call.create(NULL, &unknown_id, (void **)&device);
        printf("%s: %#lx; object: %s\n", names[n], (unsigned long)hr, device ? "present" : "absent");
        if (hr != S_OK || !device) failed = 1;
        if (device) IUnknown_Release(device);
    }
    FreeLibrary(library);
    CoUninitialize();
    return failed;
}
