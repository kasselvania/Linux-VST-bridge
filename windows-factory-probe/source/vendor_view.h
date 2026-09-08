#pragma once
#include "gui_channel.h"
#include "pluginterfaces/base/keycodes.h"
#include "pluginterfaces/gui/iplugview.h"
#include "pluginterfaces/gui/iplugviewcontentscalesupport.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include <algorithm>
#include <cmath>
#include <thread>
#include <windows.h>

namespace linux_vst_bridge::wf0 {
// Owns one real IPlugView and its local Win32 parent, on the controller's UI
// thread. It has no processor, socket, audio queue, or session-creation API.
class VendorView final : public Steinberg::IPlugFrame {
  using View = Steinberg::IPlugView;
  std::atomic<Steinberg::uint32> references_{1};
  std::thread::id owner_ = std::this_thread::get_id();
  View *view_ = nullptr;
  HWND window_ = nullptr;
  bool attached_ = false, closing_ = false;
  void (*trace_)(void *, uint32_t) = nullptr;
  void *trace_context_ = nullptr;
  void (*fault_trace_)(void *, EXCEPTION_POINTERS *) = nullptr;
  LONG attach_fault(EXCEPTION_POINTERS *e) {
    if (fault_trace_)
      fault_trace_(trace_context_, e);
    // Preserve normal exception handling and outer process containment. This
    // records only bounded exception metadata, never a vendor memory dump.
    return EXCEPTION_CONTINUE_SEARCH;
  }
  static Steinberg::tresult attach_view(VendorView *self) {
    __try {
      return self->view_->attached(self->window_, Steinberg::kPlatformTypeHWND);
    } __except (self->attach_fault(GetExceptionInformation())) {
      return Steinberg::kInternalError;
    }
  }
  void stage(uint32_t code) {
    if (trace_)
      trace_(trace_context_, code);
  }
  static bool same(const Steinberg::ViewRect &a,
                   const Steinberg::ViewRect &b) {
    return a.left == b.left && a.top == b.top && a.right == b.right &&
           a.bottom == b.bottom;
  }
  unsigned resizing_ = 0;
  uint32_t error_ = 0;
  static Steinberg::int16 modifiers() {
    using namespace Steinberg;
    return (GetKeyState(VK_SHIFT) < 0 ? kShiftKey : 0) |
           (GetKeyState(VK_MENU) < 0 ? kAlternateKey : 0) |
           (GetKeyState(VK_CONTROL) < 0 ? kCommandKey : 0) |
           ((GetKeyState(VK_LWIN) < 0 || GetKeyState(VK_RWIN) < 0) ? kControlKey
                                                                   : 0);
  }
  static Steinberg::int16 key(WPARAM code) {
    using namespace Steinberg;
    if (code >= VK_F1 && code <= VK_F12)
      return int16(KEY_F1 + code - VK_F1);
    if (code >= VK_NUMPAD0 && code <= VK_NUMPAD9)
      return int16(KEY_NUMPAD0 + code - VK_NUMPAD0);
    switch (code) {
    case VK_BACK:
      return KEY_BACK;
    case VK_TAB:
      return KEY_TAB;
    case VK_RETURN:
      return KEY_RETURN;
    case VK_ESCAPE:
      return KEY_ESCAPE;
    case VK_SPACE:
      return KEY_SPACE;
    case VK_HOME:
      return KEY_HOME;
    case VK_END:
      return KEY_END;
    case VK_LEFT:
      return KEY_LEFT;
    case VK_UP:
      return KEY_UP;
    case VK_RIGHT:
      return KEY_RIGHT;
    case VK_DOWN:
      return KEY_DOWN;
    case VK_PRIOR:
      return KEY_PAGEUP;
    case VK_NEXT:
      return KEY_PAGEDOWN;
    case VK_INSERT:
      return KEY_INSERT;
    case VK_DELETE:
      return KEY_DELETE;
    case VK_SHIFT:
      return KEY_SHIFT;
    case VK_CONTROL:
      return KEY_CONTROL;
    case VK_MENU:
      return KEY_ALT;
    default:
      return 0;
    }
  }
  static LRESULT CALLBACK procedure(HWND window, UINT msg, WPARAM wp,
                                    LPARAM lp) {
    auto *self = reinterpret_cast<VendorView *>(
        GetWindowLongPtrW(window, GWLP_USERDATA));
    if (msg == WM_NCCREATE) {
      auto *c = reinterpret_cast<CREATESTRUCTW *>(lp);
      self = static_cast<VendorView *>(c->lpCreateParams);
      SetWindowLongPtrW(window, GWLP_USERDATA,
                        reinterpret_cast<LONG_PTR>(self));
      self->window_ = window;
    }
    if (!self)
      return DefWindowProcW(window, msg, wp, lp);
    try {
      if (msg == WM_CLOSE) {
        self->close();
        return 0;
      }
      if (msg == WM_SETFOCUS && self->view_ && self->attached_) {
        self->view_->onFocus(true);
        if (auto child = GetWindow(window, GW_CHILD))
          SetFocus(child);
        return 0;
      }
      // Child windows receive their normal Win32 input. Only input addressed to
      // this host parent is forwarded through IPlugView, avoiding duplicate
      // keys.
      if (self->view_ && self->attached_) {
        if (msg == WM_MOUSEWHEEL &&
            self->view_->onWheel(float(GET_WHEEL_DELTA_WPARAM(wp)) /
                                 WHEEL_DELTA) == Steinberg::kResultOk)
          return 0;
        if ((msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && key(wp) &&
            self->view_->onKeyDown(0, key(wp), modifiers()) ==
                Steinberg::kResultOk)
          return 0;
        if ((msg == WM_KEYUP || msg == WM_SYSKEYUP) &&
            self->view_->onKeyUp(
                Steinberg::char16(MapVirtualKeyW(UINT(wp), MAPVK_VK_TO_CHAR) &
                                  0xffff),
                key(wp), modifiers()) == Steinberg::kResultOk)
          return 0;
        if (msg == WM_CHAR && wp >= 0x21 && wp != 0x7f &&
            self->view_->onKeyDown(Steinberg::char16(wp), 0, modifiers()) ==
                Steinberg::kResultOk)
          return 0;
      }
      if (msg == WM_KILLFOCUS && self->view_ && self->attached_)
        self->view_->onFocus(false);
      if (msg == WM_SIZE && self->attached_ && !self->closing_ &&
          !self->resizing_ && wp != SIZE_MINIMIZED) {
        RECT r{};
        GetClientRect(window, &r);
        Steinberg::ViewRect size{0, 0, r.right, r.bottom};
        if (self->resizing_ < 8) {
          ++self->resizing_;
          Steinberg::ViewRect current{};
          auto result = self->view_->getSize(&current);
          if (result == Steinberg::kResultOk && !same(current, size))
            result = self->view_->onSize(&size);
          --self->resizing_;
          if (result != Steinberg::kResultOk)
            self->error_ = AP11::Size;
        } else
          self->error_ = AP11::Size;
        return 0;
      }
      if (msg == WM_SIZING && self->view_) {
        auto *r = reinterpret_cast<RECT *>(lp);
        RECT client{};
        GetClientRect(window, &client);
        RECT old{};
        GetWindowRect(window, &old);
        int frame_x = old.right - old.left - client.right,
            frame_y = old.bottom - old.top - client.bottom;
        Steinberg::ViewRect size{0, 0, r->right - r->left - frame_x,
                                 r->bottom - r->top - frame_y};
        if (self->view_->checkSizeConstraint(&size) == Steinberg::kResultOk) {
          r->right = r->left + size.getWidth() + frame_x;
          r->bottom = r->top + size.getHeight() + frame_y;
          return TRUE;
        }
      }
    } catch (...) {
      self->error_ = AP11::Controller;
    }
    return DefWindowProcW(window, msg, wp, lp);
  }
  bool resize(Steinberg::ViewRect &size) {
    if (size.getWidth() < 1 || size.getHeight() < 1 || size.getWidth() > 8192 ||
        size.getHeight() > 8192)
      return false;
    RECT current{};
    if (GetClientRect(window_, &current) &&
        current.right == size.getWidth() && current.bottom == size.getHeight())
      return true;
    RECT r{0, 0, size.getWidth(), size.getHeight()};
    auto style = DWORD(GetWindowLongPtrW(window_, GWL_STYLE));
    if (!AdjustWindowRectEx(&r, style, FALSE, 0))
      return false;
    return SetWindowPos(window_, nullptr, 0, 0, r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE) != FALSE;
  }

public:
  void diagnostic(void (*trace)(void *, uint32_t), void *context) {
    trace_ = trace;
    trace_context_ = context;
  }
  void fault_diagnostic(void (*trace)(void *, EXCEPTION_POINTERS *)) {
    fault_trace_ = trace;
  }
  uint64_t opens = 0, closes = 0, focuses = 0;
  bool scale_supported = false;
  float scale = 1.f;
  ~VendorView() {
    if (!close())
      std::terminate();
  }
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id,
                                               void **out) override {
    if (!out)
      return Steinberg::kInvalidArgument;
    *out = nullptr;
    if (Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::IPlugFrame::iid) ||
        Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::FUnknown::iid)) {
      *out = static_cast<Steinberg::IPlugFrame *>(this);
      addRef();
      return Steinberg::kResultOk;
    }
    return Steinberg::kNoInterface;
  }
  Steinberg::uint32 PLUGIN_API addRef() override { return ++references_; }
  Steinberg::uint32 PLUGIN_API release() override { return --references_; }
  Steinberg::tresult PLUGIN_API resizeView(View *view,
                                           Steinberg::ViewRect *size) override {
    if (std::this_thread::get_id() != owner_ || view != view_ || !size ||
        !window_ || closing_ || resizing_ >= 8)
      return Steinberg::kResultFalse;
    ++resizing_;
    stage(12);
    Steinberg::ViewRect current{};
    auto result = view_->getSize(&current);
    if (result == Steinberg::kResultOk) {
      stage(13);
      result = resize(*size) ? Steinberg::kResultOk : Steinberg::kResultFalse;
      stage(15);
      if (result == Steinberg::kResultOk &&
          !same(current, *size)) {
        stage(14);
        result = view_->onSize(size);
      }
    }
    stage(16);
    --resizing_;
    return result;
  }
  bool open(Steinberg::Vst::IEditController &controller) {
    using namespace Steinberg;
    if (owner_ != std::this_thread::get_id()) {
      error_ = AP11::WrongThread;
      return false;
    }
    if (window_ && view_) {
      ShowWindow(window_, SW_RESTORE);
      SetForegroundWindow(window_);
      SetFocus(window_);
      ++focuses;
      return true;
    }
    error_ = 0;
    try {
      stage(1);
      view_ = controller.createView(Vst::ViewType::kEditor);
      if (!view_) {
        error_ = AP11::NoView;
        return false;
      }
      stage(2);
      if (view_->isPlatformTypeSupported(kPlatformTypeHWND) != kResultOk) {
        error_ = AP11::Platform;
        close();
        return false;
      }
      WNDCLASSW wc{};
      wc.lpfnWndProc = procedure;
      wc.hInstance = GetModuleHandleW(nullptr);
      wc.lpszClassName = L"LVBVendorEditorAP11";
      wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
      if (!RegisterClassW(&wc) &&
          GetLastError() != ERROR_CLASS_ALREADY_EXISTS) {
        error_ = AP11::Attach;
        close();
        return false;
      }
      stage(3);
      bool resizable = view_->canResize() == kResultTrue;
      DWORD style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX |
                    (resizable ? (WS_THICKFRAME | WS_MAXIMIZEBOX) : 0);
      stage(4);
      window_ = CreateWindowExW(0, wc.lpszClassName, L"Vendor editor", style,
                                CW_USEDEFAULT, CW_USEDEFAULT, 640, 480, nullptr,
                                nullptr, wc.hInstance, this);
      if (!window_) {
        error_ = AP11::Attach;
        close();
        return false;
      }
      stage(6);
      ViewRect rect{};
      if (view_->getSize(&rect) != kResultOk) {
        error_ = AP11::Size;
        close();
        return false;
      }
      RECT work{};
      SystemParametersInfoW(SPI_GETWORKAREA, 0, &work, 0);
      FUnknownPtr<IPlugViewContentScaleSupport> content(view_);
      scale_supported = bool(content);
      scale = 1.f;
      if (content && rect.getWidth() > 0 && rect.getHeight() > 0) {
        scale =
            std::min({1.f, float(work.right - work.left - 24) / rect.getWidth(),
                      float(work.bottom - work.top - 64) / rect.getHeight()});
        scale = std::max(.5f, scale);
        stage(7);
        if (content->setContentScaleFactor(scale) != kResultOk) {
          error_ = AP11::Size;
          close();
          return false;
        }
        if (view_->getSize(&rect) != kResultOk) {
          error_ = AP11::Size;
          close();
          return false;
        }
      }
      // Initial scale precedes frame installation. The SDK explicitly permits
      // this order: getSize then returns the scaled size without a resizeView
      // callback into a not-yet-attached platform view (observed with Serum).
      stage(5);
      if (view_->setFrame(this) != kResultOk) {
        error_ = AP11::Attach;
        close();
        return false;
      }
      stage(8);
      if (!resize(rect)) {
        error_ = AP11::Size;
        close();
        return false;
      }
      stage(9);
      if (attach_view(this) != kResultOk) {
        error_ = AP11::Attach;
        close();
        return false;
      }
      attached_ = true;
      stage(10);
      ViewRect attached_size{};
      if (view_->getSize(&attached_size) != kResultOk ||
          !resize(attached_size)) {
        error_ = AP11::Size;
        close();
        return false;
      }
      stage(11);
      ShowWindow(window_, SW_SHOW);
      SetForegroundWindow(window_);
      SetFocus(window_);
      stage(100);
      ++opens;
      return true;
    } catch (...) {
      error_ = AP11::Controller;
      close();
      return false;
    }
  }
  bool close() {
    if (owner_ != std::this_thread::get_id()) {
      error_ = AP11::WrongThread;
      return false;
    }
    if (closing_)
      return true;
    closing_ = true;
    try {
      if (view_ && attached_) {
        if (view_->removed() != Steinberg::kResultOk) {
          error_ = AP11::Removal;
          closing_ = false;
          return false;
        }
        attached_ = false;
        ++closes;
      }
      if (view_) {
        if (view_->setFrame(nullptr) != Steinberg::kResultOk) {
          error_ = AP11::Removal;
          closing_ = false;
          return false;
        }
        view_->release();
        view_ = nullptr;
      }
      if (window_) {
        auto w = window_;
        window_ = nullptr;
        DestroyWindow(w);
      }
      closing_ = false;
      return true;
    } catch (...) {
      error_ = AP11::Removal;
      closing_ = false;
      return false;
    }
  }
  bool is_open() const { return attached_; }
  uint32_t error() const { return error_; }
  HWND window() const { return window_; }
  static bool pump() {
    MSG msg{};
    for (unsigned i = 0;
         i < 128 && PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE); ++i) {
      if (msg.message == WM_QUIT) {
        PostQuitMessage(int(msg.wParam));
        return false;
      }
      TranslateMessage(&msg);
      DispatchMessageW(&msg);
    }
    return true;
  }
};
} // namespace linux_vst_bridge::wf0
