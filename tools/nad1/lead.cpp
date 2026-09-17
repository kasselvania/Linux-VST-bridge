// Source-owned process identity instrumentation, never the vendor daemon.
#include <windows.h>
int wmain(int argc, wchar_t**) {
    if (argc != 1) return 64;
    Sleep(12000);
    return 0;
}
