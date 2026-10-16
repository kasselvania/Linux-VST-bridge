#pragma once
// Windows a plug-in opens on its own while it is checked or set up (#414), and
// manager-declared service dependencies.
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
        if (record_.empty()) return;
        const std::string text = std::string("{\"schema\":1,\"windows\":") + std::to_string(facts.windows) +
            ",\"visible\":" + std::to_string(facts.visible) + ",\"dialogs\":" + std::to_string(facts.dialogs) +
            ",\"off_screen\":" + std::to_string(facts.off_screen) + ",\"minimized\":" +
            std::to_string(facts.minimized) + ",\"observed_ms\":" + std::to_string(elapsed) + "}\n";
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
            publish(settled, scan.now - started);
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

// Only manager-declared dependencies may be started. The declaration contains
// service key names, never image paths or commands. Missing services are facts,
// not permission to install or recreate vendor state. Total wait is 20 seconds.
inline void ensure_required_services(EventWriter& events) {
    wchar_t names[1024]{};
    const DWORD length=GetEnvironmentVariableW(L"LVB_REQUIRED_SERVICES",names,1024);
    if(!length || length>=1024)return;
    const SC_HANDLE manager=OpenSCManagerW(nullptr,nullptr,SC_MANAGER_CONNECT);
    if(!manager){events.lifecycle("required_services",",\"available\":false");return;}
    const ULONGLONG deadline=GetTickCount64()+20000;
    std::wstring list(names,length);std::size_t offset=0;unsigned count=0;
    while(offset<list.size() && count++<16){
        const auto end=list.find(L';',offset);
        const auto name=list.substr(offset,end==std::wstring::npos?list.size()-offset:end-offset);
        offset=end==std::wstring::npos?list.size():end+1;
        if(name.empty() || name.size()>64 || !std::all_of(name.begin(),name.end(),[](wchar_t c){
            return (c>=L'A'&&c<=L'Z')||(c>=L'a'&&c<=L'z')||(c>=L'0'&&c<=L'9')||c==L'_'||c==L'-';}))continue;
        const std::string plain(name.begin(),name.end());
        const SC_HANDLE service=OpenServiceW(manager,name.c_str(),SERVICE_QUERY_STATUS|SERVICE_START);
        DWORD before=0,after=0,start_result=ERROR_SUCCESS,error=0;
        if(service){
            SERVICE_STATUS_PROCESS status{};DWORD size=0;
            if(QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<BYTE*>(&status),sizeof(status),&size)){
                before=after=status.dwCurrentState;
                if(before==SERVICE_STOPPED)start_result=StartServiceW(service,0,nullptr)?ERROR_SUCCESS:GetLastError();
                do{
                    if(!QueryServiceStatusEx(service,SC_STATUS_PROCESS_INFO,reinterpret_cast<BYTE*>(&status),sizeof(status),&size)){error=GetLastError();break;}
                    after=status.dwCurrentState;
                    if(after!=SERVICE_START_PENDING || GetTickCount64()>=deadline)break;
                    Sleep(100);
                }while(true);
            }else error=GetLastError();
            CloseServiceHandle(service);
        }else error=GetLastError();
        events.lifecycle("required_service",",\"name\":\""+plain+"\",\"before\":"+std::to_string(before)+
            ",\"start_result\":"+std::to_string(start_result)+",\"after\":"+std::to_string(after)+",\"error\":"+std::to_string(error));
    }
    CloseServiceHandle(manager);
}

} // namespace linux_vst_bridge::wf0
