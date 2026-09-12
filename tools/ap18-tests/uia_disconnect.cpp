// Account-free reproduction of the observed pinned-runtime disconnect fault.
// Dynamic loading also verifies the operation-scoped unavailable posture.
#include <windows.h>

int main() {
    HMODULE module = LoadLibraryW(L"uiautomationcore.dll");
    if (!module) return GetLastError() == ERROR_MOD_NOT_FOUND ? 0 : 3;
    using Disconnect = HRESULT (WINAPI*)(void*);
    auto disconnect = reinterpret_cast<Disconnect>(
        GetProcAddress(module, "UiaDisconnectProvider"));
    if (!disconnect) return 4;
    __try {
        disconnect(nullptr);
    } __except (GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION
                    ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        FreeLibrary(module);
        return 42; // Observed failure path caught by this disposable fixture.
    }
    FreeLibrary(module);
    return 5; // Distinct from both unavailable and reproduced access violation.
}
