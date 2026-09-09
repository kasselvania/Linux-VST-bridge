#include "environment_desktop.h"
#include <cassert>
#include <iostream>
#include <thread>

int main() {
  using linux_vst_bridge::wf0::EnvironmentDesktop;
  HWND retained = nullptr;
  {
    EnvironmentDesktop desktop;
    bool wrong_open = true, wrong_pump = true;
    std::thread other([&] { wrong_open = desktop.open(); wrong_pump = desktop.pump(); });
    other.join();
    assert(!wrong_open && !wrong_pump);
    assert(desktop.open());
    retained = desktop.window();
    assert(IsWindow(retained) && !IsWindowVisible(retained));
    assert(GetWindowThreadProcessId(retained, nullptr) == GetCurrentThreadId());
    assert(!desktop.open());
    assert(desktop.pump());
    // A short-lived independent GUI window must not retire the owner window.
    auto child = CreateWindowExW(0, L"STATIC", L"", WS_POPUP, 0, 0, 1, 1,
                                  nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
    assert(child && DestroyWindow(child));
    assert(desktop.pump() && IsWindow(retained) && !IsWindowVisible(retained));
  }
  assert(!IsWindow(retained));
  {
    EnvironmentDesktop desktop;
    assert(desktop.open());
    PostMessageW(desktop.window(), WM_CLOSE, 0, 0);
    assert(!desktop.pump()); // loss cannot retain a healthy readiness claim
  }
  std::cout << "AP14 environment desktop ownership/lifetime PASS\n";
}
