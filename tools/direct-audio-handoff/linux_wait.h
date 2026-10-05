#pragma once
#include <windows.h>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <type_traits>

// Original adapter for the x86-64 Wine/Linux runner boundary. This is a private
// runner capability, not an assumed ABI on arbitrary Wine or native Windows.
namespace lvb::linux_wait {
struct Deadline { std::int64_t sec{}, nsec{}; };
struct Arguments {
    std::uint64_t address{};
    Deadline deadline{};
    std::uint32_t expected{}, operation{};
    std::int64_t result{};
};
static_assert(sizeof(Arguments) == 40 && offsetof(Arguments, deadline) == 8 &&
              offsetof(Arguments, expected) == 24 && offsetof(Arguments, result) == 32);
static_assert(std::is_trivially_copyable_v<Arguments>);

class Runtime {
    using Dispatcher = std::int32_t(WINAPI*)(std::uint64_t, unsigned, void*);
    Dispatcher dispatcher_{};
    std::uint64_t handle_{};
    HMODULE module_{};
    const char* refusal_{"Wine/Linux dispatcher unavailable"};
    LONG status_{};
    DWORD error_{};
    std::int64_t invoke(Arguments& args) const noexcept {
        if (!dispatcher_ || !handle_) return -38;
        const auto status = dispatcher_(handle_, 0, &args);
        return status == 0 ? args.result : -38;
    }
public:
    Runtime() = default;
    Runtime(const Runtime&) = delete;
    Runtime& operator=(const Runtime&) = delete;
    ~Runtime() { if (module_) FreeLibrary(module_); } // only after all waiters retire
    // Called inactive. Missing/non-Linux exports refuse before helper loading.
    bool bind() noexcept {
#if defined(_M_X64)
        const auto module = GetModuleHandleW(L"ntdll.dll");
        if (!module || !GetProcAddress(module, "wine_get_version")) return false;
        using HostVersion = void(__cdecl*)(const char**, const char**);
        const auto host = reinterpret_cast<HostVersion>(GetProcAddress(module, "wine_get_host_version"));
        if (!host) return false;
        const char *system{}, *release{};
        host(&system, &release);
        if (!system || std::strcmp(system, "Linux")) return false;
        // The export is a POINTER VARIABLE, not the dispatcher function itself.
        const auto exported = GetProcAddress(module, "__wine_unix_call_dispatcher");
        if (!exported) return false;
        dispatcher_ = *reinterpret_cast<Dispatcher const*>(exported);
        if (!dispatcher_) return false;
        // The SDK module loader restricts the process default search to
        // SYSTEM32. Select the product-owned companion beside this executable
        // explicitly; never relax that policy or search the vendor directory.
        wchar_t path[32768]{};
        const auto length=GetModuleFileNameW(nullptr,path,32768);
        if (!length || length>=32768) {
            refusal_="host executable path unavailable"; error_=GetLastError(); return false;
        }
        DWORD base=length;
        while (base && path[base-1]!=L'\\' && path[base-1]!=L'/') --base;
        constexpr wchar_t name[]=L"lvb-direct-wait.dll";
        if (!base || base+sizeof(name)/sizeof(name[0])>32768) {
            refusal_="host companion path unavailable"; return false;
        }
        std::memcpy(path+base,name,sizeof(name));
        module_=LoadLibraryExW(path,nullptr,
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);
        if (!module_) { refusal_="builtin companion could not load"; error_=GetLastError(); return false; }
        using Abi=std::uint32_t(__cdecl*)();
        const auto abi=reinterpret_cast<Abi>(GetProcAddress(module_,"lvb_wait_abi"));
        using Query=LONG(NTAPI*)(HANDLE,void*,ULONG,void*,SIZE_T,SIZE_T*);
        const auto query=reinterpret_cast<Query>(GetProcAddress(module,"NtQueryVirtualMemory"));
        // Private class 1000 from the selected Wine runner. The returned value
        // owns no storage; module_ pins the builtin ELF companion/table.
        if (!abi || abi()!=1 || !query) { refusal_="builtin ABI/query unavailable"; return false; }
        status_=query(GetCurrentProcess(),module_,1000,&handle_,sizeof handle_,nullptr);
        if (status_!=0 || !handle_) { refusal_="builtin Unix table unavailable"; return false; }
        return true;
#else
        return false;
#endif
    }
    const char* refusal() const noexcept { return refusal_; }
    LONG status() const noexcept { return status_; }
    DWORD error() const noexcept { return error_; }
    bool monotonic(Deadline& value) const noexcept {
        Arguments args{};
        if (invoke(args) != 0) return false;
        value = args.deadline;
        return value.sec >= 0 && value.nsec >= 0 && value.nsec < 1000000000;
    }
    std::int64_t wait(volatile LONG* word, LONG expected, Deadline deadline) const noexcept {
        Arguments args{reinterpret_cast<std::uintptr_t>(word), deadline,
                       static_cast<std::uint32_t>(expected), 1, 0};
        return invoke(args); // 0, -EAGAIN/-EINTR retry same deadline, -ETIMEDOUT, error
    }
    std::int64_t wake(volatile LONG* word) const noexcept {
        Arguments args{reinterpret_cast<std::uintptr_t>(word), {}, 0, 2, 0};
        return invoke(args);
    }
};
inline Deadline after(Deadline now, std::int64_t nanoseconds) noexcept {
    now.sec += nanoseconds / 1000000000;
    now.nsec += nanoseconds % 1000000000;
    if (now.nsec >= 1000000000) { ++now.sec; now.nsec -= 1000000000; }
    return now;
}
} // namespace lvb::linux_wait
