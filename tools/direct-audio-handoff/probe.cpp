#include "linux_wait.h"
#include <cstdio>
#include <cstdlib>
#include <cwchar>

static LONG load(volatile LONG* p) noexcept { return InterlockedCompareExchange(p, 0, 0); }
int wmain(int argc, wchar_t** argv) {
    lvb::linux_wait::Runtime runtime;
    if (!runtime.bind()) {
        std::printf("unsupported: %s nt_status=%ld win32=%lu; no Linux syscall executed\n",
                    runtime.refusal(),runtime.status(),runtime.error());
        return argc == 2 && !std::wcscmp(argv[1], L"--native-refusal") ? 0 : 77;
    }
    if (argc != 2 || !std::wcscmp(argv[1], L"--native-refusal")) return 78;
    const auto file = CreateFileW(argv[1], GENERIC_READ | GENERIC_WRITE,
                                 FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_EXISTING, 0, nullptr);
    LARGE_INTEGER size{};
    if (file == INVALID_HANDLE_VALUE || !GetFileSizeEx(file, &size) || size.QuadPart != 4096) return 2;
    const auto mapping = CreateFileMappingW(file, nullptr, PAGE_READWRITE, 0, 4096, nullptr);
    auto* words = static_cast<volatile LONG*>(MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, 4096));
    if (!words) return 3;
    lvb::linux_wait::Deadline now{};
    if (!runtime.monotonic(now) || runtime.wait(words, 99, lvb::linux_wait::after(now, 30000000)) != -11) return 4;
    if (runtime.wait(words, 0, lvb::linux_wait::after(now, 30000000)) != -110) return 5;
    InterlockedExchange(words + 2, 1); // inactive behavior checks passed; native peer may begin
    std::uint32_t woke_native{};
    for (LONG ticket = 1; ticket <= 1000; ++ticket) {
        if (!runtime.monotonic(now)) return 6;
        const auto deadline = lvb::linux_wait::after(now, 3000000000);
        for (;;) {
            const auto observed = load(words);
            if (observed == ticket) break;
            if (observed != ticket - 1) return 11;
            if (!runtime.monotonic(now) || now.sec > deadline.sec ||
                (now.sec == deadline.sec && now.nsec >= deadline.nsec)) return 12;
            if (load(words + 3)) return 7;
            const auto result = runtime.wait(words, ticket - 1, deadline);
            if (result != 0 && result != -11 && result != -4) return 8;
        }
        if (ticket == 1) Sleep(10); // only probe: ensure native peer is actually asleep
        InterlockedExchange(words + 1, ticket);
        const auto result = runtime.wake(words + 1);
        if (result < 0) return 9;
        woke_native += static_cast<std::uint32_t>(result);
    }
    InterlockedExchange(words + 2, 2);
    std::printf("shared_futex_ok exchanges=1000 wine_woke_native=%u timeout_and_eagain=ok\n", woke_native);
    UnmapViewOfFile(const_cast<LONG*>(words)); CloseHandle(mapping); CloseHandle(file);
    return woke_native ? 0 : 10;
}
