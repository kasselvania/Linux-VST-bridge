// Original first-party Win32 edge. No plug-in, installer or licensed state.
#include "editor_presentation.h"
#include <windows.h>
#include <fcntl.h>
#include <io.h>
#include <cstdio>
#include <cstdlib>
#include <cwchar>
#include <memory>
#include <thread>
using namespace LVBEditorPresentation;
namespace {
Record identity;
HWND window = nullptr;
bool ready = false;
bool menu_active = false, menu_reported = false;
ULONGLONG menu_deadline = 0;
uint32_t clicks = 0;
constexpr UINT control = WM_APP + 1;
constexpr UINT_PTR popup_timer = 1;
void write(uint32_t kind, uint32_t result = 0) {
  auto record = identity;
  record.kind = kind; record.result = result;
  if (window) {
    RECT size{}; GetClientRect(window, &size);
    record.width = uint32_t(size.right); record.height = uint32_t(size.bottom);
  }
  if (std::fwrite(&record, sizeof(record), 1, stdout) != 1 || std::fflush(stdout))
    ExitProcess(2);
}
LRESULT CALLBACK procedure(HWND hwnd, UINT message, WPARAM w, LPARAM l) {
  if (message == WM_LBUTTONDOWN) { write(InputObserved, ++clicks); return 0; }
  if (message == WM_ENTERMENULOOP) {
    menu_active = true; menu_reported = false; menu_deadline = GetTickCount64() + 2000;
    SetTimer(hwnd, popup_timer, 25, nullptr);
  }
  if (message == WM_TIMER && w == popup_timer && menu_active && !menu_reported) {
    bool visible = false;
    EnumThreadWindows(GetCurrentThreadId(), [](HWND candidate, LPARAM context) -> BOOL {
      wchar_t name[32]{}; GetClassNameW(candidate, name, 32);
      if (IsWindowVisible(candidate) && std::wcscmp(name, L"#32768") == 0)
        *reinterpret_cast<bool*>(context) = true;
      return TRUE;
    }, reinterpret_cast<LPARAM>(&visible));
    if (visible || GetTickCount64() >= menu_deadline) {
      menu_reported = true; write(PopupVisible, visible ? 1u : 0u);
      if (!visible) EndMenu();
    }
    return 0;
  }
  if (message == WM_EXITMENULOOP) { menu_active = false; KillTimer(hwnd, popup_timer); }
  if (message == control) {
    std::unique_ptr<Record> request(reinterpret_cast<Record*>(l));
    if (!request || !request->valid() || request->owner != identity.owner ||
        request->epoch != identity.epoch || request->window != identity.window ||
        request->presentation != identity.presentation) { PostQuitMessage(3); return 0; }
    switch (request->kind) {
    case Resize: {
      if (!request->width || !request->height || request->width > 65535 || request->height > 65535) {
        PostQuitMessage(4); return 0;
      }
      RECT rect{0, 0, LONG(request->width), LONG(request->height)};
      AdjustWindowRect(&rect, DWORD(GetWindowLongPtrW(hwnd, GWL_STYLE)), FALSE);
      if (!SetWindowPos(hwnd, nullptr, 0, 0, rect.right - rect.left, rect.bottom - rect.top,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE)) { PostQuitMessage(5); return 0; }
      write(WindowResized); return 0;
    }
    case Focus:
      SetForegroundWindow(hwnd); SetFocus(hwnd);
      write(FocusResult, GetFocus() == hwnd ? 0 : 1); return 0;
    case Popup: {
      HMENU menu = CreatePopupMenu();
      if (!menu || !AppendMenuW(menu, MF_STRING, 1, L"First-party popup")) {
        if (menu) DestroyMenu(menu); PostQuitMessage(6); return 0;
      }
      POINT point{24, 24}; ClientToScreen(hwnd, &point);
      const auto selected = TrackPopupMenuEx(menu, TPM_RETURNCMD,
                                            point.x, point.y, hwnd, nullptr);
      DestroyMenu(menu); write(PopupResult, uint32_t(selected)); return 0;
    }
    case CancelPopup: EndMenu(); return 0;
    case Close: DestroyWindow(hwnd); return 0;
    default: PostQuitMessage(7); return 0;
    }
  }
  if (message == WM_CLOSE) { DestroyWindow(hwnd); return 0; }
  if (message == WM_DESTROY) {
    window = nullptr; if (ready) write(WindowClosed); PostQuitMessage(0); return 0;
  }
  if (message == WM_PAINT) {
    PAINTSTRUCT paint{}; auto dc = BeginPaint(hwnd, &paint);
    RECT area{}; GetClientRect(hwnd, &area);
    auto background = CreateSolidBrush(RGB(28, 76, 102));
    FillRect(dc, &area, background); DeleteObject(background);
    SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(255, 255, 255));
    DrawTextW(dc, L"First-party Wine editor", -1, &area, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    EndPaint(hwnd, &paint); return 0;
  }
  return DefWindowProcW(hwnd, message, w, l);
}
}
bool decimal(const char* text, uint64_t& value) {
  if (!text || !*text) return false;
  value = 0;
  for (const char* c = text; *c; ++c) {
    if (*c < '0' || *c > '9') return false;
    const auto digit = uint64_t(*c - '0');
    if (value > (UINT64_MAX - digit) / 10) return false;
    value = value * 10 + digit;
  }
  return true;
}
int main(int argc, char** argv) {
  if (argc != 4) return 10;
  uint64_t owner = 0, epoch = 0, mode = 0;
  if (!decimal(argv[1], owner) || !decimal(argv[2], epoch) || !decimal(argv[3], mode) ||
      epoch > UINT32_MAX || mode > UINT32_MAX) return 11;
  identity.owner = owner; identity.epoch = uint32_t(epoch); identity.presentation = uint32_t(mode);
  if (!identity.valid()) return 11;
  _setmode(_fileno(stdin), _O_BINARY); _setmode(_fileno(stdout), _O_BINARY);
  WNDCLASSW wc{}; wc.lpfnWndProc = procedure; wc.hInstance = GetModuleHandleW(nullptr);
  wc.lpszClassName = L"LVBFirstPartyEmbeddingV1"; wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
  if (!RegisterClassW(&wc)) return 12;
  const DWORD style = identity.presentation == EmbeddedX11 ? WS_POPUP : WS_OVERLAPPEDWINDOW;
  window = CreateWindowExW(0, wc.lpszClassName, L"First-party Wine editor", style,
                           40, 80, 640, 400, nullptr, nullptr, wc.hInstance, nullptr);
  if (!window) return 13;
  ShowWindow(window, SW_SHOWNOACTIVATE); UpdateWindow(window);
  const auto xid = reinterpret_cast<uintptr_t>(GetPropW(window, L"__wine_x11_whole_window"));
  if (!xid || xid > UINT32_MAX) {
    std::fputs("missing Wine X11 whole-window capability\n", stderr);
    DestroyWindow(window); return 77;
  }
  identity.window = uint32_t(xid); ready = true; write(WindowReady);
  // The window/Win32 operations stay on main. The bounded fixture sends a
  // finite command stream; EOF requests ordinary window destruction.
  std::thread reader([editor = window] {
    for (;;) {
      auto request = std::make_unique<Record>();
      if (std::fread(request.get(), sizeof(Record), 1, stdin) != 1) {
        PostMessageW(editor, WM_CLOSE, 0, 0); return;
      }
      if (!PostMessageW(editor, control, 0, reinterpret_cast<LPARAM>(request.get()))) return;
      request.release();
    }
  });
  reader.detach(); // process ends with this finite first-party window lifetime
  MSG message{};
  while (GetMessageW(&message, nullptr, 0, 0) > 0) { TranslateMessage(&message); DispatchMessageW(&message); }
  return int(message.wParam);
}
