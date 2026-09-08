#pragma once
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "public.sdk/source/common/pluginview.h"
#include <X11/Xlib.h>
#include <cstring>
#include <thread>
namespace AP11 {
struct PanelOwner {
  virtual void panelLoop(Steinberg::Linux::IRunLoop *) = 0;
  virtual void panelOpen() = 0;
  virtual void panelClose() = 0;
  virtual const char *panelStatus() const = 0;
  virtual ~PanelOwner() = default;
};
// The retained controller reference owns the delegate for the entire view
// lifetime. No window pointer, timer, or task crosses the process boundary.
class VendorPanel final : public Steinberg::CPluginView,
                          public Steinberg::Linux::ITimerHandler {
  Steinberg::IPtr<Steinberg::Vst::IEditController> controller_;
  PanelOwner &owner_;
  const char *name_;
  Steinberg::IPtr<Steinberg::Linux::IRunLoop> loop_;
  std::thread::id thread_ = std::this_thread::get_id();
  Display *display_ = nullptr;
  Window window_ = 0;
  GC gc_ = nullptr;
  bool registered_ = false;
  void text(int x, int y, const char *value) {
    XDrawString(display_, window_, gc_, x, y, value, int(std::strlen(value)));
  }
  void draw() {
    XSetForeground(display_, gc_, 0x20252b);
    XFillRectangle(display_, window_, gc_, 0, 0, 560, 150);
    XSetForeground(display_, gc_, 0xeeeeee);
    text(18, 26, name_);
    text(18, 53, owner_.panelStatus());
    XSetForeground(display_, gc_, 0x45647c);
    XFillRectangle(display_, window_, gc_, 18, 77, 244, 40);
    XFillRectangle(display_, window_, gc_, 282, 77, 258, 40);
    XSetForeground(display_, gc_, 0xffffff);
    text(34, 102, "Open / focus vendor editor");
    text(300, 102, "Close vendor editor");
    text(18, 139, "The editor opens in its own window.");
    XFlush(display_);
  }

public:
  VendorPanel(Steinberg::Vst::IEditController *c, PanelOwner &owner,
              const char *name)
      : controller_(c), owner_(owner), name_(name) {
    setRect({0, 0, 560, 150});
  }
  ~VendorPanel() override { removed(); }
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id,
                                               void **out) override {
    if (!out)
      return Steinberg::kInvalidArgument;
    if (Steinberg::FUnknownPrivate::iidEqual(
            id, Steinberg::Linux::ITimerHandler::iid)) {
      *out = static_cast<Steinberg::Linux::ITimerHandler *>(this);
      addRef();
      return Steinberg::kResultOk;
    }
    return CPluginView::queryInterface(id, out);
  }
  Steinberg::uint32 PLUGIN_API addRef() override {
    return CPluginView::addRef();
  }
  Steinberg::uint32 PLUGIN_API release() override {
    return CPluginView::release();
  }
  Steinberg::tresult PLUGIN_API
  isPlatformTypeSupported(Steinberg::FIDString type) override {
    return type && !std::strcmp(type, Steinberg::kPlatformTypeX11EmbedWindowID)
               ? Steinberg::kResultOk
               : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API attached(void *parent,
                                         Steinberg::FIDString type) override {
    using namespace Steinberg;
    if (thread_ != std::this_thread::get_id() || display_ || !parent ||
        isPlatformTypeSupported(type) != kResultOk)
      return kResultFalse;
    loop_ = FUnknownPtr<Linux::IRunLoop>(plugFrame);
    if (!loop_ || !(display_ = XOpenDisplay(nullptr)))
      return kResultFalse;
    window_ = XCreateSimpleWindow(display_, reinterpret_cast<Window>(parent), 0,
                                  0, 560, 150, 0, 0, 0x20252b);
    gc_ = XCreateGC(display_, window_, 0, nullptr);
    XSelectInput(display_, window_, ExposureMask | ButtonPressMask | ButtonReleaseMask);
    XMapWindow(display_, window_);
    XFlush(display_);
    if (loop_->registerTimer(this, 50) != kResultOk) {
      removed();
      return kResultFalse;
    }
    registered_ = true;
    systemWindow = parent;
    owner_.panelLoop(loop_);
    owner_.panelOpen();
    draw();
    return kResultOk;
  }
  Steinberg::tresult PLUGIN_API removed() override {
    if (thread_ != std::this_thread::get_id())
      return Steinberg::kResultFalse;
    if (registered_) {
      registered_ = false;
      loop_->unregisterTimer(this);
    }
    if (display_) {
      owner_.panelClose();
      XFreeGC(display_, gc_);
      XDestroyWindow(display_, window_);
      XCloseDisplay(display_);
      display_ = nullptr;
      window_ = 0;
      gc_ = nullptr;
    }
    loop_ = nullptr;
    systemWindow = nullptr;
    return Steinberg::kResultOk;
  }
  void PLUGIN_API onTimer() override {
    if (!display_ || thread_ != std::this_thread::get_id())
      return;
    for (unsigned i = 0; i < 32 && XPending(display_); ++i) {
      XEvent e{};
      XNextEvent(display_, &e);
      if (e.type == ButtonRelease && e.xbutton.button == Button1 &&
          e.xbutton.y >= 77 && e.xbutton.y <= 117) {
        if (e.xbutton.x >= 18 && e.xbutton.x <= 262)
          owner_.panelOpen();
        else if (e.xbutton.x >= 282 && e.xbutton.x <= 540)
          owner_.panelClose();
      }
    }
    draw();
  }
};
} // namespace AP11
