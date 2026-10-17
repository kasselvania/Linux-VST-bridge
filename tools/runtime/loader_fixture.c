/* Original account-free native-loader and cross-architecture child smoke test.
 * Copyright 2026 LVB contributors. SPDX-License-Identifier: MIT
 */
#ifndef UNICODE
#define UNICODE
#endif
#include <windows.h>
#include <stdio.h>
#include <wchar.h>

static int child(const WCHAR *path, unsigned int bits)
{
    STARTUPINFOW startup = {0};
    PROCESS_INFORMATION process = {0};
    WCHAR command[2 * MAX_PATH];
    DWORD status, code;
    BOOL wow64 = FALSE;
    int result = 1;
    if (wcslen(path) >= MAX_PATH || wcschr(path, L'"')) return 1;
    swprintf(command, 2 * MAX_PATH, L"\"%ls\" --child", path);
    startup.cb = sizeof(startup);
    if (!CreateProcessW(path, command, NULL, NULL, FALSE, CREATE_NO_WINDOW,
                        NULL, NULL, &startup, &process)) {
        fprintf(stderr, "CreateProcess(%u-bit) failed: %lu\n", bits, (unsigned long)GetLastError());
        return 1;
    }
    status = WaitForSingleObject(process.hProcess, 10000);
    if (status != WAIT_OBJECT_0) {
        fprintf(stderr, "%u-bit child did not exit within 10 seconds: %#lx\n", bits, (unsigned long)status);
        TerminateProcess(process.hProcess, 1);
        WaitForSingleObject(process.hProcess, 5000);
    } else if (GetExitCodeProcess(process.hProcess, &code) && code == bits
               && IsWow64Process(process.hProcess, &wow64)) {
        printf("%u-bit child executed and returned its actual pointer width; WoW64: %u\n", bits, (unsigned int)wow64);
        result = 0;
    } else {
        fprintf(stderr, "%u-bit child returned an unexpected result\n", bits);
    }
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    return result;
}

int wmain(int argc, WCHAR **argv)
{
    WCHAR own[MAX_PATH];
    DWORD length, type;
    unsigned int bits = sizeof(void *) * 8;
    int result;
    if (argc == 2 && !wcscmp(argv[1], L"--child")) return bits;
    if ((argc != 2 && argc != 3) ||
        (argc == 3 && wcscmp(argv[2], L"--wait-for-readback"))) return 2;
    length = GetModuleFileNameW(NULL, own, MAX_PATH);
    if (!length || length >= MAX_PATH || !GetBinaryTypeW(own, &type) ||
        type != (bits == 64 ? SCS_64BIT_BINARY : SCS_32BIT_BINARY)) return 2;
    if (!GetBinaryTypeW(argv[1], &type) ||
        type != (bits == 64 ? SCS_32BIT_BINARY : SCS_64BIT_BINARY)) return 2;
    printf("Native parent pointer width: %u; opposite PE header verified\n", bits);
    fflush(stdout);
    if (argc == 3) Sleep(5000);
    result = child(own, bits);
    result |= child(argv[1], 96 - bits);
    return result;
}
