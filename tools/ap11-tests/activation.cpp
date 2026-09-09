// Production activation helper against an isolated X server and explicit WM
// double. This checks protocol/lifetime; actual KWin focus is separate
// evidence.
#include "desktop_activation.h"
#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <chrono>
#include <iostream>
#include <thread>
namespace {
void check(bool value, const char *why) {
  if (!value) {
    std::cerr << "FAIL: " << why << '\n';
    std::exit(1);
  }
}
} // namespace
int main() {
  auto *d = XOpenDisplay(nullptr);
  check(d, "isolated display");
  auto root = DefaultRootWindow(d);
  auto atom = XInternAtom(d, "_NET_ACTIVE_WINDOW", False);
  auto create = [&](Window parent) {
    auto w = XCreateSimpleWindow(d, parent, 0, 0, 64, 64, 0, 0, 0);
    XMapWindow(d, w);
    return w;
  };
  auto source = create(root), target = create(root), other = create(root);
  auto child = create(target);
  XSelectInput(d, root, SubstructureNotifyMask);
  auto activate = [&](Window w, Window keyboard) {
    unsigned long value = w;
    XChangeProperty(d, root, atom, XA_WINDOW, 32, PropModeReplace,
                    reinterpret_cast<unsigned char *>(&value), 1);
    XSetInputFocus(d, keyboard, RevertToParent, CurrentTime);
    XSync(d, False);
  };
  AP11::DesktopActivation activation;
  ap11_gui_message_t request{};
  request.native_view = 3;
  request.activation = 12;
  request.user_time = 123456;
  request.requestor_x11 = uint32_t(source);
  auto reply = request;
  reply.target_x11 = uint32_t(target);
  reply.view_epoch = 7;
  unsigned messages = 0;
  // action: 0 deny; 1 accept; 2 property only, no keyboard focus.
  auto run = [&](unsigned action) {
    auto until = std::chrono::steady_clock::now() + std::chrono::seconds(3);
    ap11_gui_message_t result{};
    while (std::chrono::steady_clock::now() < until) {
      while (XPending(d)) {
        XEvent e{};
        XNextEvent(d, &e);
        if (e.type != ClientMessage || e.xclient.message_type != atom)
          continue;
        ++messages;
        check(e.xclient.window == target && e.xclient.format == 32 &&
                  e.xclient.data.l[0] == 1 &&
                  e.xclient.data.l[1] == request.user_time &&
                  e.xclient.data.l[2] == long(source),
              "application source, real click context and owned target");
        if (action) {
          XMapWindow(d, target);
          activate(target, action == 1 ? child : source);
        }
      }
      if (activation.poll(result))
        return result;
      std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    check(false, "bounded activation completion");
    return result;
  };
  auto begin = [&] {
    activate(source, source);
    activation.begin(request);
    activation.target(reply);
  };
  begin();
  auto result = run(1);
  check(result.focus_result == AP11::FocusConfirmed && messages == 1 &&
            result.activation == 12 && result.view_epoch == 7,
        "confirmed active target and child keyboard focus");
  check(!activation.poll(result), "completion delivered once");
  begin();
  result = run(0);
  check(result.focus_result == AP11::FocusDenied && messages == 2,
        "WM refusal explicit and bounded");
  begin();
  result = run(2);
  check(result.focus_result == AP11::FocusDenied && messages == 3,
        "active property alone cannot claim keyboard focus");
  XUnmapWindow(d, target);
  begin();
  result = run(1);
  check(result.focus_result == AP11::FocusConfirmed && messages == 4,
        "WM can restore existing unmapped target without recreation");
  activate(other, other);
  activation.begin(request);
  activation.target(reply);
  result = run(1);
  check(result.focus_result == AP11::FocusDenied && messages == 4,
        "changed foreground cancels handoff without stealing focus back");
  activate(source, source);
  activation.begin(request);
  auto stale = reply;
  ++stale.native_view;
  activation.target(stale);
  check(!activation.poll(result), "old native view cannot select a replacement target");
  stale = reply;
  stale.abi_version = 2;
  activation.target(stale);
  check(!activation.poll(result), "old UI ABI cannot select a target");
  stale = reply;
  ++stale.activation;
  activation.target(stale);
  check(!activation.poll(result), "stale activation does not select a target");
  activation.target(reply);
  result = run(1);
  check(result.focus_result == AP11::FocusConfirmed && messages == 5,
        "current reply remains usable after stale reply");
  begin();
  activation.cancel();
  check(!activation.poll(result), "cancel discards pending replies");
  auto absent = request;
  absent.user_time = 0;
  activation.begin(absent);
  activation.target(reply);
  result = run(1);
  check(result.focus_result == AP11::FocusUnsupported && messages == 5,
        "missing user context does not forge activation");
  activation.begin(request);
  result = run(0); // no target reply: the production two-second deadline
  check(result.focus_result == AP11::FocusDenied && !result.target_x11,
        "missing target reply times out explicitly");
  activation.target(reply);
  result = run(0);
  check(result.focus_result == AP11::FocusDenied &&
            result.target_x11 == reply.target_x11 && result.view_epoch == 7 &&
            messages == 5,
        "late attachment receives bound denial without stale activation");
  activation.target(reply);
  check(!activation.poll(result), "late target denial delivered once");
  activate(source, source);
  activation.begin(request);
  XDestroyWindow(d, target);
  XSync(d, False);
  activation.target(reply);
  result = run(0);
  check(result.focus_result != AP11::FocusConfirmed,
        "destroyed target cannot crash host or report success");
  XDestroyWindow(d, source);
  XDestroyWindow(d, other);
  XCloseDisplay(d);
  std::cout << "AP11 production desktop activation protocol/lifetime PASS\n";
}
