// Production IPlugView ownership against an isolated X11 host shell.
// This tests protocol/lifetime mechanics; actual Bitwig acceptance is separate.
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "vendor_panel.h"
#include <X11/Xlib.h>
#include <X11/Xatom.h>
#include "editor_lifecycle.h"
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
  AP11::ActivationContext lastActivation{};
  uint32_t lifecycle = AP11::Opening;
  uint64_t lastToken = 0;
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
  void panelOpen(uint64_t token, AP11::ActivationContext context = {}) override {
    lastToken = token;
    ++opens;
    lastActivation = context;
  }
  void panelClose(uint64_t token) override { check(token == lastToken, "exact native view close"); ++closes; }
  uint32_t panelState(uint64_t token) const override { return token == lastToken ? lifecycle : AP11::Absent; }
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
  const auto protocols = XInternAtom(display, "WM_PROTOCOLS", False);
  const auto close = XInternAtom(display, "WM_DELETE_WINDOW", False);
  XSetWMProtocols(display, parent, const_cast<Atom *>(&close), 1);
  unsigned long requestor = parent, timestamp = 12345;
  XChangeProperty(display, DefaultRootWindow(display), XInternAtom(display, "_NET_ACTIVE_WINDOW", False),
      XA_WINDOW, 32, PropModeReplace, reinterpret_cast<unsigned char *>(&requestor), 1);
  XChangeProperty(display, parent, XInternAtom(display, "_NET_WM_USER_TIME", False),
      XA_CARDINAL, 32, PropModeReplace, reinterpret_cast<unsigned char *>(&timestamp), 1);
  XSync(display, False);
  Frame frame;
  auto *controller = new Vst::EditController;
  auto *panel = new AP11::VendorPanel(controller, frame, 7);
  controller->release();
  check(panel->setFrame(&frame) == kResultOk &&
            panel->attached(reinterpret_cast<void *>(parent),
                            kPlatformTypeX11EmbedWindowID) == kResultOk,
        "production panel attached");
  check(frame.opens == 1 && frame.lastToken == 7 && frame.lastActivation.user_time == timestamp &&
        frame.lastActivation.requestor_x11 == parent, "one automatic open with host context");
  check(panel->attached(reinterpret_cast<void *>(parent), kPlatformTypeX11EmbedWindowID) != kResultOk &&
        frame.opens == 1, "duplicate attach creates no second editor request");
  ViewRect size{};
  check(panel->getSize(&size) == kResultOk && size.getWidth() > 0 && size.getHeight() > 0 &&
        panel->onSize(&size) == kResultOk, "nonzero SDK geometry lifecycle");
  Window root, actualParent, *children = nullptr;
  unsigned count = 0;
  check(XQueryTree(display, parent, &root, &actualParent, &children, &count) && count == 0,
        "no bridge control windows or button hit regions");
  if (children) XFree(children);
  XMapWindow(display, parent); XSync(display, False); frame.drain();
  XWindowAttributes attributes{};
  check(XGetWindowAttributes(display, parent, &attributes) && attributes.map_state == IsUnmapped,
        "host shell hidden without destroying host ownership");
  frame.lifecycle = AP11::ClosedByVendor;
  frame.drain();
  unsigned closeMessages = 0;
  while (XPending(display)) {
    XEvent event{}; XNextEvent(display, &event);
    if (event.type == ClientMessage && event.xclient.window == parent &&
        event.xclient.message_type == protocols && Atom(event.xclient.data.l[0]) == close)
      ++closeMessages;
  }
  check(closeMessages == 1, "vendor close requests one normal host-owned shell retirement");
  check(panel->removed() == kResultOk && panel->removed() == kResultOk && frame.timers.empty() &&
        frame.closes == 1, "native removal closes exact view once and cancels timer");
  check(XGetWindowAttributes(display, parent, &attributes), "host parent survives delegate retirement");
  panel->setFrame(nullptr);
  panel->release();
  XDestroyWindow(display, parent);
  XCloseDisplay(display);
  std::cout
      << "AP15 production X11 delegate ownership and retirement PASS\n";
}
