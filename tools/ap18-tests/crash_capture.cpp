// Private diagnostic validation only. Never loaded into a DAW or vendor process.
// Exercise the installed runner's loader/SEH/unwind traces without account UI.
#include <windows.h>
#include <cstdio>
#include <cstring>

extern "C" __declspec(dllexport) __declspec(noinline) void ap18_fault_site() {
    *static_cast<volatile LONG*>(nullptr) = 1;
}
extern "C" __declspec(dllexport) __declspec(noinline) void ap18_fault_caller() {
    ap18_fault_site();
}
int main(int argc, char** argv) {
    // Fixture-only: no crash dialog or debugger. The fatal mode leaves the
    // deliberate access violation unhandled; it is not SIGKILL or a parser mock.
    SetErrorMode(SEM_NOGPFAULTERRORBOX);
    Sleep(300); // Permit the existing supervisor to retain actual image maps.
    if (argc == 2 && std::strcmp(argv[1], "--normal") == 0) {
        std::puts("{\"event\":\"lifecycle\",\"state\":\"scanner_completed\"}");
        return 0;
    }
    if (argc == 2 && std::strcmp(argv[1], "--exit") == 0) return 23;
    if (argc == 2 && std::strcmp(argv[1], "--fatal-late") == 0) {
        // Exceed the old first-64-KiB capture before the actual exception.
        for (unsigned i = 0; i < 16384; ++i) std::fputs("CA1 fixture startup padding\n", stderr);
        std::fflush(stderr);
        ap18_fault_caller();
        return 4;
    }
    if (argc == 2 && std::strcmp(argv[1], "--fatal") == 0) {
        ap18_fault_caller();
        return 4; // Unreachable: a normal return would invalidate the proof.
    }
    __try {
        ap18_fault_caller();
    } __except (GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION
                    ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        std::puts("{\"event\":\"lifecycle\",\"state\":\"scanner_completed\"}");
        return 0;
    }
    return 3; // The deliberate fault must reach the handler.
}
