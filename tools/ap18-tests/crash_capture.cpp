// Private diagnostic validation only. Never loaded into a DAW or vendor process.
// Exercise the installed runner's loader/SEH/unwind traces without account UI.
#include <windows.h>

extern "C" __declspec(dllexport) __declspec(noinline) void ap18_fault_site() {
    *static_cast<volatile LONG*>(nullptr) = 1;
}
extern "C" __declspec(dllexport) __declspec(noinline) void ap18_fault_caller() {
    ap18_fault_site();
}
int main() {
    __try {
        ap18_fault_caller();
    } __except (GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION
                    ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        return 0;
    }
    return 3; // The deliberate fault must reach the handler.
}
