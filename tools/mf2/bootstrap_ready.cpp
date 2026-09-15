// Source-owned bootstrap payload. No SDK, account, input or vendor files.
#include <windows.h>
int main() {
    const char ready[] = "MF2_BOOTSTRAP_READY_V1\n";
    DWORD written=0;
    if (!WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),ready,sizeof(ready)-1,&written,nullptr)
        || written != sizeof(ready)-1) return 21;
#ifdef MF2_BOOTSTRAP_HOLD
    Sleep(180000);
#else
    Sleep(1000);
#endif
    return 0;
}
