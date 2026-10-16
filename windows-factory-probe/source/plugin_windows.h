#pragma once
// Windows a plug-in opens on its own while it is checked or set up (#414), and
// the installer-registered services Windows would have started at boot.
//
// A protected plug-in can stop inside LoadLibrary, factory creation or
// activation and wait for the user to answer its own sign-in, activation or
// update window. A check cannot answer it; the musician can, in the visible
// setup session. This watcher only observes this process's own top-level
// windows (no titles, no input, no other process). In the visible setup it
// also restores and raises those windows once each, so a prompt is not left
// minimised, off-screen or behind other windows. It never answers a prompt.
#include "linux_vst_bridge/wf0_probe/events.h"
#include <windows.h>
#include <winsvc.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <cstdint>
#include <cwctype>
#include <string>
#include <thread>
#include <vector>

namespace linux_vst_bridge::wf0 {

struct PluginWindowFacts {
    unsigned windows = 0, visible = 0, dialogs = 0, off_screen = 0, minimized = 0;
};

// Classes this host creates itself, and Wine/Windows helper windows that every
// GUI thread may own. Anything else that is top-level in this process belongs
// to the plug-in (or a component it loaded).
inline bool host_window_class(const wchar_t* name) {
    static constexpr const wchar_t* own[] = {
        L"LVBVendorEditorAP11", L"LVBEnvironmentDesktopOwner", L"LVBGraphicsProbe",
        L"IME", L"MSCTFIME UI", L"OleMainThreadWndClass", L"tooltips_class32"};
    for (const auto* item : own)
        if (lstrcmpiW(name, item) == 0) return true;
    return false;
}

inline bool dialog_window_class(const wchar_t* name) { return lstrcmpW(name, L"#32770") == 0; }

class PluginWindowWatch final {
    std::wstring record_;
    bool raise_;
    std::atomic<bool> stop_{false};
    std::atomic<bool> primed_{false};
    std::thread thread_;
    bool written_ = false;
    std::array<HWND, 64> raised_{};
    std::size_t raised_count_ = 0;
    std::vector<std::pair<HWND, ULONGLONG>> first_seen_;

    struct Scan {
        PluginWindowWatch* self;
        PluginWindowFacts facts;
        std::vector<HWND> found;
        ULONGLONG now;
    };
    static BOOL CALLBACK visit(HWND window, LPARAM context) {
        auto& scan = *reinterpret_cast<Scan*>(context);
        DWORD pid = 0;
        GetWindowThreadProcessId(window, &pid);
        if (pid != GetCurrentProcessId()) return TRUE;
        wchar_t name[64]{};
        if (GetClassNameW(window, name, 64) <= 0 || host_window_class(name)) return TRUE;
        const bool visible = IsWindowVisible(window) != FALSE;
        const bool dialog = dialog_window_class(name);
        // Hidden helper windows (message pumps, toolkit owners) are not
        // prompts. A dialog stays a prompt even while it is hidden.
        if (!visible && !dialog) return TRUE;
        if (scan.found.size() < 64) scan.found.push_back(window);
        ++scan.facts.windows;
        if (visible) ++scan.facts.visible;
        if (dialog) ++scan.facts.dialogs;
        if (IsIconic(window)) ++scan.facts.minimized;
        RECT rect{};
        const RECT screen{GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN)};
        RECT overlap{};
        if (GetWindowRect(window, &rect) && !IsIconic(window) && !IntersectRect(&overlap, &rect, &screen))
            ++scan.facts.off_screen;
        return TRUE;
    }
    bool raised(HWND window) const {
        return std::find(raised_.begin(), raised_.begin() + raised_count_, window) !=
               raised_.begin() + raised_count_;
    }
    // Asynchronous Win32 requests only: the owning thread may be blocked in the
    // plug-in's own modal loop, and this thread must never wait on it.
    void raise(HWND window) {
        if (raised(window) || raised_count_ == raised_.size()) return;
        raised_[raised_count_++] = window;
        if (IsIconic(window)) ShowWindowAsync(window, SW_RESTORE);
        else if (!IsWindowVisible(window)) ShowWindowAsync(window, SW_SHOWNORMAL);
        RECT rect{}, overlap{};
        const RECT screen{GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN)};
        UINT flags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS;
        int x = 0, y = 0;
        if (GetWindowRect(window, &rect) && !IntersectRect(&overlap, &rect, &screen)) {
            const int width = rect.right - rect.left, height = rect.bottom - rect.top;
            x = std::max(0, (GetSystemMetrics(SM_CXSCREEN) - width) / 2);
            y = std::max(0, (GetSystemMetrics(SM_CYSCREEN) - height) / 3);
        } else {
            flags |= SWP_NOMOVE;
        }
        SetWindowPos(window, HWND_TOP, x, y, 0, 0, flags);
        SetForegroundWindow(window);
    }
    void publish(const PluginWindowFacts& facts, ULONGLONG elapsed) {
        if (written_ || record_.empty()) return;
        written_ = true;
        const std::string text = std::string("{\"schema\":1,\"windows\":") + std::to_string(facts.windows) +
            ",\"visible\":" + std::to_string(facts.visible) + ",\"dialogs\":" + std::to_string(facts.dialogs) +
            ",\"off_screen\":" + std::to_string(facts.off_screen) + ",\"minimized\":" +
            std::to_string(facts.minimized) + ",\"first_seen_ms\":" + std::to_string(elapsed) + "}\n";
        const std::wstring temporary = record_ + L".tmp";
        const HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS,
            FILE_ATTRIBUTE_NORMAL, nullptr);
        if (file == INVALID_HANDLE_VALUE) return;
        DWORD written = 0;
        const bool ok = WriteFile(file, text.data(), DWORD(text.size()), &written, nullptr) &&
            written == text.size() && FlushFileBuffers(file);
        CloseHandle(file);
        if (!ok || !MoveFileExW(temporary.c_str(), record_.c_str(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH))
            DeleteFileW(temporary.c_str());
    }
    void run() {
        const ULONGLONG started = GetTickCount64();
        // First user32 use happens here, before the owner thread enters the
        // plug-in's loader notification (which holds the loader lock).
        Scan prime{this, {}, {}, started};
        EnumWindows(&PluginWindowWatch::visit, reinterpret_cast<LPARAM>(&prime));
        primed_.store(true);
        while (!stop_.load()) {
            Scan scan{this, {}, {}, GetTickCount64()};
            EnumWindows(&PluginWindowWatch::visit, reinterpret_cast<LPARAM>(&scan));
            // A window counts once it has existed for one second: a splash
            // that closes itself is not a prompt.
            std::vector<std::pair<HWND, ULONGLONG>> seen;
            PluginWindowFacts settled;
            for (HWND window : scan.found) {
                auto prior = std::find_if(first_seen_.begin(), first_seen_.end(),
                    [&](const auto& row) { return row.first == window; });
                const ULONGLONG since = prior == first_seen_.end() ? scan.now : prior->second;
                seen.emplace_back(window, since);
                if (scan.now - since >= 1000) {
                    settled = scan.facts;
                    if (raise_) raise(window);
                }
            }
            first_seen_ = std::move(seen);
            if (settled.windows) publish(scan.facts, scan.now - started);
            for (int slice = 0; slice < 5 && !stop_.load(); ++slice) Sleep(50);
        }
    }

public:
    // `record` is a file in this session's own directory; empty disables the
    // record. `raise` restores/raises plug-in windows (visible setup only).
    PluginWindowWatch(std::wstring record, bool raise) : record_(std::move(record)), raise_(raise) {
        thread_ = std::thread([this] { run(); });
        for (int wait = 0; wait < 200 && !primed_.load(); ++wait) Sleep(10);
    }
    ~PluginWindowWatch() {
        stop_.store(true);
        if (thread_.joinable()) thread_.join();
    }
    PluginWindowWatch(const PluginWindowWatch&) = delete;
    PluginWindowWatch& operator=(const PluginWindowWatch&) = delete;
};

// The product always runs this host under its pinned Wine runner. On Windows
// itself (CI), boot already started automatic services; never start any there.
inline bool running_under_wine() {
    const HMODULE ntdll = GetModuleHandleW(L"ntdll.dll");
    return ntdll && GetProcAddress(ntdll, "wine_get_version") != nullptr;
}

// Installer-registered automatic services (for example a vendor licence
// service) are running before the plug-in loads, as Windows guarantees after
// boot. Wine's own services (images under the Windows directory) are left to
// Wine. Bounded: 20 seconds in total; then the plug-in loads regardless and
// the record says which services were not running.
inline void ensure_auto_start_services(EventWriter& events) {
    const ULONGLONG started = GetTickCount64(), deadline = started + 20000;
    const SC_HANDLE manager = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE);
    if (!manager) {
        events.lifecycle("auto_start_services", ",\"available\":false,\"error\":" + std::to_string(GetLastError()));
        return;
    }
    std::vector<BYTE> buffer(64 * 1024);
    DWORD needed = 0, count = 0, resume = 0;
    BOOL listed = EnumServicesStatusExW(manager, SC_ENUM_PROCESS_INFO, SERVICE_WIN32, SERVICE_STATE_ALL,
        buffer.data(), DWORD(buffer.size()), &needed, &count, &resume, nullptr);
    if (!listed && GetLastError() == ERROR_MORE_DATA && needed <= 1024 * 1024) {
        buffer.assign(std::size_t(needed) + 4096, 0);
        resume = 0;
        listed = EnumServicesStatusExW(manager, SC_ENUM_PROCESS_INFO, SERVICE_WIN32, SERVICE_STATE_ALL,
            buffer.data(), DWORD(buffer.size()), &needed, &count, &resume, nullptr);
    }
    if (!listed) {
        events.lifecycle("auto_start_services", ",\"available\":false,\"error\":" + std::to_string(GetLastError()));
        CloseServiceHandle(manager);
        return;
    }
    wchar_t windows_directory[MAX_PATH]{};
    const UINT windows_length = GetSystemWindowsDirectoryW(windows_directory, MAX_PATH);
    const std::wstring system_root = windows_length && windows_length < MAX_PATH
        ? std::wstring(windows_directory, windows_length) + L"\\" : L"C:\\windows\\";
    auto lower = [](std::wstring text) {
        for (auto& c : text) c = wchar_t(std::towlower(c));
        return text;
    };
    // Plain ASCII service key names only; anything else is recorded as null.
    auto plain = [](const wchar_t* name) {
        std::string out;
        for (std::size_t i = 0; name[i]; ++i) {
            const wchar_t c = name[i];
            if (i >= 64 || c > 127 || !(std::iswalnum(c) || c == L' ' || c == L'.' || c == L'_' || c == L'-'))
                return std::string("null");
            out += char(c);
        }
        return out.empty() ? std::string("null") : "\"" + out + "\"";
    };
    struct Row { SC_HANDLE handle; std::string name; DWORD before, start_result, after; };
    std::vector<Row> rows;
    const auto* entries = reinterpret_cast<const ENUM_SERVICE_STATUS_PROCESSW*>(buffer.data());
    for (DWORD i = 0; i < count && rows.size() < 32; ++i) {
        const SC_HANDLE service = OpenServiceW(manager, entries[i].lpServiceName,
            SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS | SERVICE_START);
        if (!service) continue;
        DWORD size = 0;
        QueryServiceConfigW(service, nullptr, 0, &size);
        std::vector<BYTE> config(std::size_t(std::max<DWORD>(size, DWORD(sizeof(QUERY_SERVICE_CONFIGW)))) + 16);
        auto* value = reinterpret_cast<QUERY_SERVICE_CONFIGW*>(config.data());
        if (size > 64 * 1024 || !QueryServiceConfigW(service, value, DWORD(config.size()), &size) ||
            value->dwStartType != SERVICE_AUTO_START || !value->lpBinaryPathName) {
            CloseServiceHandle(service);
            continue;
        }
        std::wstring image = value->lpBinaryPathName;
        if (!image.empty() && image.front() == L'"') image.erase(0, 1);
        if (lower(image).rfind(lower(system_root), 0) == 0) {
            CloseServiceHandle(service);
            continue;
        }
        rows.push_back({service, plain(entries[i].lpServiceName), entries[i].ServiceStatusProcess.dwCurrentState,
            ERROR_SUCCESS, entries[i].ServiceStatusProcess.dwCurrentState});
    }
    for (auto& row : rows) {
        if (row.before == SERVICE_STOPPED)
            row.start_result = StartServiceW(row.handle, 0, nullptr) ? ERROR_SUCCESS : GetLastError();
    }
    bool pending = true;
    while (pending) {
        pending = false;
        for (auto& row : rows) {
            SERVICE_STATUS_PROCESS status{};
            DWORD size = 0;
            if (QueryServiceStatusEx(row.handle, SC_STATUS_PROCESS_INFO, reinterpret_cast<BYTE*>(&status),
                    sizeof(status), &size))
                row.after = status.dwCurrentState;
            if (row.after == SERVICE_START_PENDING) pending = true;
        }
        if (pending && GetTickCount64() >= deadline) break;
        if (pending) Sleep(100);
    }
    unsigned running = 0;
    for (auto& row : rows) {
        if (row.after == SERVICE_RUNNING) ++running;
        events.lifecycle("auto_start_service", ",\"name\":" + row.name + ",\"before\":" +
            std::to_string(row.before) + ",\"start_result\":" + std::to_string(row.start_result) +
            ",\"after\":" + std::to_string(row.after));
        CloseServiceHandle(row.handle);
    }
    CloseServiceHandle(manager);
    events.lifecycle("auto_start_services", ",\"available\":true,\"count\":" + std::to_string(rows.size()) +
        ",\"running\":" + std::to_string(running) + ",\"elapsed_ms\":" +
        std::to_string(GetTickCount64() - started) + ",\"deadline_expired\":" + (pending ? "true" : "false"));
}

} // namespace linux_vst_bridge::wf0
