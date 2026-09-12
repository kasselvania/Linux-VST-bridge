// Development-only generated HWND workload. Calls the actual production pump;
// never loads a vendor module, opens an audio endpoint, or changes a product.
#include "vendor_view.h"
#include <objbase.h>
#include <array>
#include <cassert>
#include <cstdio>
#include <cstring>
#include <string>

using linux_vst_bridge::wf0::VendorView;
namespace {
constexpr UINT posted_message = WM_APP + 0x620, sent_message = WM_APP + 0x621;
constexpr unsigned posts = 4096, seeds = 4, sends = 64;
struct alignas(8) Status {
  char magic[8];
  uint64_t version, pid, start, root, child, frequency;
  volatile LONG64 ready, command, phase, phase_qpc, submitted, posted, sent,
      send_failures, down, up, down_qpc, up_qpc, turns, max_turn_qpc,
      finished, error, active_chains,
      loaded_down_posts, loaded_up_posts, loaded_down_chains, loaded_up_chains,
      loaded_down_qpc, loaded_up_qpc, paints, timers, loaded_paints, loaded_timers,
      moves, quit_preserved, bound_preserved;
};
static_assert(sizeof(Status) <= 4096);
Status *s;
HWND child;
uint64_t cutoff;
bool self_test = false;
uint64_t qpc() { LARGE_INTEGER n{}; QueryPerformanceCounter(&n); return n.QuadPart; }
uint64_t load(volatile LONG64 &v) { return uint64_t(InterlockedCompareExchange64(&v, 0, 0)); }
void put(volatile LONG64 &v, uint64_t n) { InterlockedExchange64(&v, LONG64(n)); }
bool post_one() {
  if (load(s->submitted) >= posts || qpc() >= cutoff) return false;
  if (!PostMessageW(child, posted_message, 0, 0)) { put(s->error, 1); return false; }
  InterlockedIncrement64(&s->submitted); return true;
}
LRESULT CALLBACK proc(HWND w, UINT msg, WPARAM wp, LPARAM lp) {
  if (msg == posted_message) {
    // Explicit generated load, not a measured vendor cost. Four finite chains
    // keep posted work available; no message is discarded. One bounded Sleep
    // models synchronous handler elapsed work without burning a CPU core.
    Sleep(1); InterlockedIncrement64(&s->posted); InvalidateRect(child, nullptr, FALSE);
    if (!post_one()) InterlockedDecrement64(&s->active_chains);
    return 0;
  }
  if (msg == sent_message) { Sleep(1); InterlockedIncrement64(&s->sent); return 0; }
  if (msg == WM_MOUSEMOVE) { InterlockedIncrement64(&s->moves); return 0; }
  if (msg == WM_TIMER) {
    InterlockedIncrement64(&s->timers);
    if (load(s->active_chains)) InterlockedIncrement64(&s->loaded_timers);
    return 0;
  }
  if (msg == WM_LBUTTONDOWN && load(s->down) == 1) {
    put(s->loaded_down_posts, load(s->posted)); put(s->loaded_down_chains, load(s->active_chains));
    put(s->loaded_down_qpc, qpc());
  }
  if (msg == WM_LBUTTONUP && load(s->up) == 1) {
    put(s->loaded_up_posts, load(s->posted)); put(s->loaded_up_chains, load(s->active_chains));
    put(s->loaded_up_qpc, qpc());
  }
  if (msg == WM_LBUTTONDOWN) { InterlockedIncrement64(&s->down); put(s->down_qpc, qpc()); SetFocus(w); return 0; }
  if (msg == WM_LBUTTONUP) { InterlockedIncrement64(&s->up); put(s->up_qpc, qpc()); return 0; }
  if (msg == WM_PAINT) {
    InterlockedIncrement64(&s->paints);
    if (load(s->active_chains)) InterlockedIncrement64(&s->loaded_paints);
    PAINTSTRUCT p{}; BeginPaint(w, &p); EndPaint(w, &p); return 0; }
  if (msg == WM_CLOSE) { put(s->command, 3); return 0; }
  return DefWindowProcW(w, msg, wp, lp);
}
}
int wmain(int argc, wchar_t **argv) {
  self_test = argc == 2 && std::wstring(argv[1]) == L"--self-test";
  if (!self_test && argc != 2) return 2;
  HRESULT apartment = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  if (FAILED(apartment)) return 3;
  HANDLE file = INVALID_HANDLE_VALUE;
  if (!self_test) file = CreateFileW(argv[1], GENERIC_READ | GENERIC_WRITE,
      FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
  if (!self_test && file == INVALID_HANDLE_VALUE) return 4;
  HANDLE mapping = CreateFileMappingW(file, nullptr, PAGE_READWRITE, 0, 4096, nullptr);
  if (!mapping) return 5;
  s = static_cast<Status *>(MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, 4096));
  if (!s) return 6;
  std::memset(s, 0, sizeof(*s)); std::memcpy(s->magic, "UIR1", 4); s->version = 2;
  s->pid = GetCurrentProcessId(); FILETIME created{}, exit{}, kernel{}, user{};
  if (!GetProcessTimes(GetCurrentProcess(), &created, &exit, &kernel, &user)) return 7;
  s->start = (uint64_t(created.dwHighDateTime) << 32) | created.dwLowDateTime;
  LARGE_INTEGER frequency{}; QueryPerformanceFrequency(&frequency); s->frequency = frequency.QuadPart;
  WNDCLASSW wc{}; wc.lpfnWndProc = proc; wc.hInstance = GetModuleHandleW(nullptr);
  wc.lpszClassName = L"LVB-UIR1-generated"; wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
  wc.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
  if (!RegisterClassW(&wc)) return 8;
  HWND root = CreateWindowW(wc.lpszClassName, L"UIR1 generated fixture", WS_OVERLAPPEDWINDOW,
      80, 60, 480, 360, nullptr, nullptr, wc.hInstance, nullptr);
  child = CreateWindowW(wc.lpszClassName, L"", WS_CHILD | WS_VISIBLE,
      0, 0, 450, 310, root, nullptr, wc.hInstance, nullptr);
  if (!root || !child) return 9;
  s->root = reinterpret_cast<uintptr_t>(root); s->child = reinterpret_cast<uintptr_t>(child);
  ShowWindow(root, SW_SHOW); UpdateWindow(root); SetForegroundWindow(root); SetFocus(child);
  if (!SetTimer(child, 1, 16, nullptr)) return 11;
  // Production pump preserves WM_QUIT and its code even through filtered paths.
  PostQuitMessage(73);
  if (VendorView::pump()) return 12;
  MSG quit{};
  if (!PeekMessageW(&quit, nullptr, WM_QUIT, WM_QUIT, PM_REMOVE) ||
      quit.message != WM_QUIT || quit.wParam != 73) return 13;
  put(s->quit_preserved, 1);
  // Actual queued sentinels prove the 128-dispatch ceiling without a mock pump.
  for (unsigned i = 0; i < 256; ++i)
    if (!PostThreadMessageW(GetCurrentThreadId(), WM_APP + 0x622, 0, 0)) return 14;
  if (!VendorView::pump()) return 15;
  unsigned remaining = 0;
  while (PeekMessageW(&quit, nullptr, WM_APP + 0x622, WM_APP + 0x622, PM_REMOVE)) ++remaining;
  if (remaining < 128 || remaining >= 256) return 16;
  put(s->bound_preserved, 1);
  put(s->phase, 1); put(s->phase_qpc, qpc()); put(s->ready, 1);
  std::thread traffic;
  const auto deadline = GetTickCount64() + 60000;
  if (self_test) put(s->command, 2);
  bool running = true, stop_requested = false;
  while (running && GetTickCount64() < deadline) {
    auto command = InterlockedExchange64(&s->command, 0);
    if (command == 2 && load(s->phase) == 1) {
      cutoff = qpc() + 6 * s->frequency;
      put(s->phase_qpc, qpc()); put(s->phase, 2);
      for (unsigned i = 0; i < seeds; ++i) if (post_one()) InterlockedIncrement64(&s->active_chains);
      traffic = std::thread([] {
        for (unsigned i = 0; i < sends; ++i) {
          DWORD_PTR result{};
          if (!SendMessageTimeoutW(child, sent_message, 0, 0, SMTO_ABORTIFHUNG | SMTO_BLOCK, 500, &result))
            InterlockedIncrement64(&s->send_failures);
          Sleep(8);
        }
      });
    } else if (command == 3) stop_requested = true;
    else if (command != 0) { put(s->error, 2); running = false; }
    auto before = qpc();
    if (!VendorView::pump()) { put(s->error, 3); break; }
    auto elapsed = qpc() - before;
    InterlockedIncrement64(&s->turns);
    if (elapsed > load(s->max_turn_qpc)) put(s->max_turn_qpc, elapsed);
    if (load(s->phase) == 2 && load(s->active_chains) == 0 && load(s->sent) + load(s->send_failures) == sends) {
      if (traffic.joinable()) traffic.join();
      put(s->phase_qpc, qpc()); put(s->phase, 3);
      if (self_test) running = false;
    }
    if (stop_requested && load(s->phase) != 2) running = false;
    Sleep(4); // The installed owner's maximum socket-wait service cadence.
  }
  if (GetTickCount64() >= deadline) put(s->error, 4);
  // No worker may outlive its UI queue. At the 60s bound all generated requests
  // have exhausted the 6s/4096-post and 64x500ms send limits.
  if (traffic.joinable()) traffic.join();
  const bool exact = load(s->submitted) == load(s->posted) && load(s->active_chains) == 0;
  if (!exact) put(s->error, 5);
  KillTimer(child, 1);
  bool destroyed = DestroyWindow(root) != FALSE && !IsWindow(root) && !IsWindow(child);
  if (!destroyed) put(s->error, 6);
  const auto error = load(s->error);
  if (self_test) {
    assert(error == 0 && load(s->posted) > 0 && load(s->posted) <= posts);
    assert(load(s->sent) == sends && load(s->send_failures) == 0);
    assert(load(s->loaded_paints) > 0 && load(s->loaded_timers) > 0);
    assert(load(s->quit_preserved) == 1 && load(s->bound_preserved) == 1);
    std::printf("UIR1 production pump: submitted=%llu handled=%llu sent=%llu exact cleanup=%d\n",
        static_cast<unsigned long long>(load(s->submitted)), static_cast<unsigned long long>(load(s->posted)),
        static_cast<unsigned long long>(load(s->sent)), destroyed);
  }
  put(s->finished, 1);
  UnmapViewOfFile(s); CloseHandle(mapping); if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
  CoUninitialize(); return error ? 10 : 0;
}
