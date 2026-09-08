// Real X11 event routing to the production panel. The parent selects mouse
// presses just as a host may; XTest lets the server assign the implicit grab.
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "vendor_panel.h"
#include <X11/extensions/XTest.h>
#include <algorithm>
#include <chrono>
#include <iostream>
#include <thread>
#include <vector>
using namespace Steinberg;
namespace {
void check(bool value, const char *why) {
  if (!value) {
    std::cerr << "FAIL: " << why << '\n';
    std::exit(1);
  }
}
struct Frame final : IPlugFrame, Linux::IRunLoop, AP11::PanelOwner {
  std::vector<Linux::ITimerHandler *> timers;
  unsigned opens = 0, closes = 0;
  tresult PLUGIN_API queryInterface(const TUID id, void **out) override {
    if (!out)
      return kInvalidArgument;
    *out = nullptr;
    if (FUnknownPrivate::iidEqual(id, Linux::IRunLoop::iid))
      *out = static_cast<Linux::IRunLoop *>(this);
    else if (FUnknownPrivate::iidEqual(id, IPlugFrame::iid) ||
             FUnknownPrivate::iidEqual(id, FUnknown::iid))
      *out = static_cast<IPlugFrame *>(this);
    else
      return kNoInterface;
    addRef();
    return kResultOk;
  }
  uint32 PLUGIN_API addRef() override { return 1; }
  uint32 PLUGIN_API release() override { return 1; }
  tresult PLUGIN_API resizeView(IPlugView *, ViewRect *) override {
    return kResultOk;
  }
  tresult PLUGIN_API registerEventHandler(Linux::IEventHandler *,
                                          int) override {
    return kNotImplemented;
  }
  tresult PLUGIN_API unregisterEventHandler(Linux::IEventHandler *) override {
    return kNotImplemented;
  }
  tresult PLUGIN_API registerTimer(Linux::ITimerHandler *t,
                                   Linux::TimerInterval) override {
    t->addRef();
    timers.push_back(t);
    return kResultOk;
  }
  tresult PLUGIN_API unregisterTimer(Linux::ITimerHandler *t) override {
    auto i = std::find(timers.begin(), timers.end(), t);
    check(i != timers.end(), "registered panel timer");
    timers.erase(i);
    t->release();
    return kResultOk;
  }
  void panelLoop(Linux::IRunLoop *) override {}
  void panelOpen() override { ++opens; }
  void panelClose() override { ++closes; }
  const char *panelStatus() const override { return "Fixture"; }
  void drain() {
    for (unsigned i = 0; i < 20; ++i) {
      for (auto *t : timers)
        t->onTimer();
      std::this_thread::sleep_for(std::chrono::milliseconds(2));
    }
  }
};
} // namespace
int main() {
  auto *display = XOpenDisplay(nullptr);
  check(display != nullptr, "isolated X server");
  auto parent = XCreateSimpleWindow(display, DefaultRootWindow(display), 40, 40,
                                    560, 150, 0, 0, 0);
  XSelectInput(display, parent, ButtonPressMask | ButtonReleaseMask);
  XMapWindow(display, parent);
  XSync(display, False);
  Frame frame;
  auto *controller = new Vst::EditController;
  auto *panel = new AP11::VendorPanel(controller, frame, "SDK fixture");
  controller->release();
  check(panel->setFrame(&frame) == kResultOk &&
            panel->attached(reinterpret_cast<void *>(parent),
                            kPlatformTypeX11EmbedWindowID) == kResultOk,
        "production panel attached");
  frame.drain();
  check(frame.opens == 1, "one initial open");
  auto click = [&](int x) {
    Window child;
    int rootX = 0, rootY = 0;
    XTranslateCoordinates(display, parent, DefaultRootWindow(display), x, 100,
                          &rootX, &rootY, &child);
    check(XTestFakeMotionEvent(display, DefaultScreen(display), rootX, rootY,
                               CurrentTime),
          "server pointer motion");
    check(XTestFakeButtonEvent(display, 1, True, CurrentTime), "server press");
    check(XTestFakeButtonEvent(display, 1, False, CurrentTime),
          "server release");
    XSync(display, False);
    frame.drain();
  };
  click(400);
  check(frame.closes == 1,
        "close click reaches child despite host press selection");
  click(100);
  check(frame.opens == 2, "open click reaches child after close");
  check(panel->removed() == kResultOk && frame.timers.empty() &&
            frame.closes == 2,
        "panel removal closes once and cancels timer");
  panel->setFrame(nullptr);
  panel->release();
  XDestroyWindow(display, parent);
  XCloseDisplay(display);
  std::cout
      << "AP11 production X11 panel implicit-grab routing and lifetime PASS\n";
}
