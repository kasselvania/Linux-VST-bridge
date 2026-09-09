#pragma once
#include <windows.h>

namespace linux_vst_bridge::wf0 {
// Initialize the shared Windows desktop before admitting a short-lived scanner
// or DSP process. Under the pinned Wine X11 driver, the first GUI process owns
// the desktop's cursor-clipping X window. A sleeping, non-GUI keeper alone does
// not establish that lifetime; scanner retirement can otherwise leave a stale
// desktop property which later editor message processing dereferences.
// This invisible owner window never loads a plug-in or accepts editor commands.
class EnvironmentDesktop final {
  DWORD owner_ = GetCurrentThreadId();
  HWND window_ = nullptr;
  static constexpr const wchar_t *name_ = L"LVBEnvironmentDesktopOwner";

public:
  EnvironmentDesktop() = default;
  EnvironmentDesktop(const EnvironmentDesktop &) = delete;
  EnvironmentDesktop &operator=(const EnvironmentDesktop &) = delete;
  ~EnvironmentDesktop() {
    if (window_)
      DestroyWindow(window_);
  }
  bool open() {
    if (GetCurrentThreadId() != owner_ || window_)
      return false;
    WNDCLASSW wc{};
    wc.lpfnWndProc = DefWindowProcW;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.lpszClassName = name_;
    if (!RegisterClassW(&wc) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS)
      return false;
    // A real hidden top-level window initializes the GUI driver and desktop.
    // HWND_MESSAGE alone does not establish the X11 desktop resource lifetime.
    window_ = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, name_, L"",
                              WS_POPUP, 0, 0, 1, 1, nullptr, nullptr,
                              wc.hInstance, nullptr);
    return window_ && GetDesktopWindow();
  }
  bool pump() {
    if (GetCurrentThreadId() != owner_ || !window_ || !IsWindow(window_))
      return false;
    MSG msg{};
    for (unsigned i = 0; i < 128 && PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE); ++i) {
      if (msg.message == WM_QUIT)
        return false;
      TranslateMessage(&msg);
      DispatchMessageW(&msg);
    }
    return IsWindow(window_) != FALSE;
  }
  HWND window() const { return window_; }
};
} // namespace linux_vst_bridge::wf0
