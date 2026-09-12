#pragma once
// A small native bridge control, not the Windows plug-in editor. The host's
// Linux run loop owns all X11 events and recovery actions; no callback work.
#include "public.sdk/source/common/pluginview.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include <X11/Xlib.h>
#include <cstring>
#include <cstdio>
#include <thread>
namespace AP2 {
class RecoveryView final : public Steinberg::CPluginView,
                           public Steinberg::Linux::ITimerHandler {
public:
  explicit RecoveryView(Steinberg::Vst::IEditController *controller)
      : controller_(controller), owner_(std::this_thread::get_id()) {
    setRect({0, 0, 640, 210});
  }
  RecoveryView(Steinberg::Vst::IEditController *controller,const char* failure,uint64_t revision)
      : RecoveryView(controller) {
    failure_=failure;
    std::snprintf(failure_snapshot_,sizeof(failure_snapshot_),revision ?
        "Confirmed snapshot #%llu retained; later edits may be unavailable." :
        "No confirmed complete snapshot is available.",static_cast<unsigned long long>(revision));
  }
  ~RecoveryView() override { removed(); }
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID iid, void **out) override {
    if (!out) return Steinberg::kInvalidArgument;
    if (Steinberg::FUnknownPrivate::iidEqual(iid, Steinberg::Linux::ITimerHandler::iid)) {
      *out = static_cast<Steinberg::Linux::ITimerHandler *>(this); addRef(); return Steinberg::kResultOk;
    }
    return CPluginView::queryInterface(iid, out);
  }
  Steinberg::uint32 PLUGIN_API addRef() override { return CPluginView::addRef(); }
  Steinberg::uint32 PLUGIN_API release() override { return CPluginView::release(); }
  Steinberg::tresult PLUGIN_API isPlatformTypeSupported(Steinberg::FIDString type) override {
    return type && !std::strcmp(type, Steinberg::kPlatformTypeX11EmbedWindowID)
               ? Steinberg::kResultOk : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API attached(void *parent, Steinberg::FIDString type) override {
    using namespace Steinberg;
    if (owner_ != std::this_thread::get_id() || display_ || !parent ||
        isPlatformTypeSupported(type) != kResultOk) return kResultFalse;
    loop_ = FUnknownPtr<Linux::IRunLoop>(plugFrame);
    if (!loop_ || !(display_ = XOpenDisplay(nullptr))) return kResultFalse;
    window_ = XCreateSimpleWindow(display_, reinterpret_cast<Window>(parent), 0, 0, 640, 210, 0, 0, 0x20252b);
    gc_ = XCreateGC(display_, window_, 0, nullptr);
    XSelectInput(display_, window_, ExposureMask | ButtonReleaseMask);
    XMapWindow(display_, window_);
    XFlush(display_);
    if (loop_->registerTimer(this, 50) != kResultOk) { removed(); return kResultFalse; }
    registered_ = true;
    systemWindow = parent;
    return kResultOk;
  }
  Steinberg::tresult PLUGIN_API removed() override {
    if (registered_) { registered_ = false; loop_->unregisterTimer(this); }
    if (display_) {
      XFreeGC(display_, gc_); XDestroyWindow(display_, window_); XCloseDisplay(display_);
      display_ = nullptr; window_ = 0; gc_ = nullptr;
    }
    loop_ = nullptr; systemWindow = nullptr;
    return Steinberg::kResultOk;
  }
  void PLUGIN_API onTimer() override {
    if (!display_ || owner_ != std::this_thread::get_id() || action_) return;
    // Drain only a bounded number per tick. The DAW owns the event thread.
    for (unsigned n = 0; n < 32 && XPending(display_); ++n) {
      XEvent event{}; XNextEvent(display_, &event);
      if (!failure_ && event.type == ButtonRelease && event.xbutton.button == Button1 &&
          event.xbutton.x >= 16 && event.xbutton.x <= 282 &&
          event.xbutton.y >= 154 && event.xbutton.y <= 190) {
        action_ = true;
        draw("Recovering selected complete snapshot...");
        controller_->setParamNormalized(recoveryID, 1.);
        action_ = false;
      }
    }
    draw(nullptr);
  }
private:
  void text(int x, int y, const char *value) {
    XDrawString(display_, window_, gc_, x, y, value, static_cast<int>(std::strlen(value)));
  }
  void parameter(Steinberg::Vst::ParamID id, char (&out)[128]) {
    Steinberg::Vst::String128 value{};
    controller_->getParamStringByValue(id, 0., value);
    for (unsigned i = 0; i < 127 && value[i]; ++i) out[i] = value[i] < 128 ? char(value[i]) : '?';
  }
  void draw(const char *pending) {
    if (failure_) {
      XSetForeground(display_,gc_,0x20252b);XFillRectangle(display_,window_,gc_,0,0,640,210);
      XSetForeground(display_,gc_,0xeeeeee);
      text(16,28,"Bridge instance failed");text(16,60,failure_);
      text(16,92,failure_snapshot_);
      text(16,124,"Remove/reload the device or reopen your saved project.");
      text(16,156,"No stale plug-in state has been substituted.");
      XFlush(display_);return;
    }
    char snapshot[128]{}, status[128]{}, gain[128]{};
    parameter(snapshotID, snapshot); parameter(recoveryID, status);
    controller_->getParamStringByValue(0, controller_->getParamNormalized(0), gain_value_);
    for (unsigned i = 0; i < 127 && gain_value_[i]; ++i) gain[i] = char(gain_value_[i]);
    XSetForeground(display_, gc_, 0x20252b); XFillRectangle(display_, window_, gc_, 0, 0, 640, 210);
    XSetForeground(display_, gc_, 0xeeeeee);
    text(16, 24, "Bridge instance recovery");
    text(16, 47, "Last confirmed complete state:"); text(16, 66, snapshot);
    text(16, 90, "Edits after this snapshot will be lost. Audio tails are not recovered.");
    text(16, 112, "Current controller gain:"); text(174, 112, gain);
    // Keep the complete bounded status visible even on the narrow fixed view.
    const char *s = pending ? pending : status;
    char first[101]{}; std::strncpy(first, s, 100); text(16, 135, first);
    XSetForeground(display_, gc_, 0x45647c); XFillRectangle(display_, window_, gc_, 16, 154, 266, 36);
    XSetForeground(display_, gc_, 0xffffff); text(32, 176, "Recover this failed instance");
    if (std::strlen(s) > 100) text(16, 205, s + 100);
    XFlush(display_);
  }
  Steinberg::IPtr<Steinberg::Vst::IEditController> controller_;
  Steinberg::IPtr<Steinberg::Linux::IRunLoop> loop_;
  Steinberg::Vst::String128 gain_value_{};
  std::thread::id owner_;
  Display *display_ = nullptr;
  Window window_ = 0;
  GC gc_ = nullptr;
  bool registered_ = false, action_ = false;
  const char* failure_=nullptr;
  char failure_snapshot_[128]{};
};
} // namespace AP2
