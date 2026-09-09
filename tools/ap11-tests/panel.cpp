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
  uint32 references = 1;
  bool acceptOpen = true;
  unsigned refuseUnregister = 0, refuseRegister = 0, failures = 0, unregisterCalls = 0;
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
  uint32 PLUGIN_API addRef() override { return ++references; }
  uint32 PLUGIN_API release() override { check(references > 1,"host reference balance");return --references; }
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
    if (refuseRegister) { --refuseRegister; return kResultFalse; }
    t->addRef();
    timers.push_back(t);
    return kResultOk;
  }
  tresult PLUGIN_API unregisterTimer(Linux::ITimerHandler *t) override {
    ++unregisterCalls;
    if (refuseUnregister) { --refuseUnregister; return kResultFalse; }
    auto i = std::find(timers.begin(), timers.end(), t);
    check(i != timers.end(), "registered panel timer");
    timers.erase(i);
    t->release();
    return kResultOk;
  }
  void panelLoop(Linux::IRunLoop *) override {}
  bool panelOpen(uint64_t token, AP11::ActivationContext context = {}) override {
    lastToken = token;
    ++opens;
    lastActivation = context;
    return acceptOpen;
  }
  bool panelClose(uint64_t token) override { check(token == lastToken, "exact native view close"); ++closes; return true; }
  void panelFailure(uint64_t token, uint32_t) override { check(token==lastToken,"failure owns exact native view"); ++failures; lifecycle=AP11::EditorFailed; }
  uint32_t panelState(uint64_t token) const override { return token == lastToken ? lifecycle : AP11::Absent; }
  void drain() {
    for (unsigned i = 0; i < 20; ++i) {
      const auto copy=timers;
      for (auto *t : copy) { t->addRef(); t->onTimer(); t->release(); }
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
  const auto unregisteredBefore=frame.unregisterCalls;
  frame.refuseUnregister=1;
  check(panel->removed()!=kResultOk && frame.timers.size()==1 && frame.closes==1,
        "failed unregister keeps exact live registration and refuses removal completion");
  check(panel->removed() == kResultOk && panel->removed() == kResultOk && frame.timers.empty() &&
        frame.closes == 1, "native removal closes exact view once and cancels timer");
  check(XGetWindowAttributes(display, parent, &attributes), "host parent survives delegate retirement");
  check(frame.unregisterCalls==unregisteredBefore+2,"one refusal and one accepted exact unregister");
  panel->setFrame(nullptr);
  panel->release();
  check(frame.references==1,"host frame and run loop return to baseline");
  for (uint64_t token=8;token<24;++token) {
    const auto priorOpens=frame.opens,priorCloses=frame.closes;
    controller=new Vst::EditController;
    panel=new AP11::VendorPanel(controller,frame,token);controller->release();
    frame.lifecycle=AP11::Opening;
    check(panel->setFrame(&frame)==kResultOk &&
          panel->attached(reinterpret_cast<void*>(parent),kPlatformTypeX11EmbedWindowID)==kResultOk,
          "repeated SDK attachment");
    XMapWindow(display,parent);XSync(display,False);frame.drain();
    check(frame.opens==priorOpens+1,"each attached lifetime emits one automatic open");
    bool refusedWrongThread=false;
    std::thread wrong([&] {ViewRect r{};refusedWrongThread=panel->getSize(&r)!=kResultOk &&
      panel->onSize(&r)!=kResultOk && panel->setFrame(nullptr)!=kResultOk &&
      panel->removed()!=kResultOk;});wrong.join();
    check(refusedWrongThread,"native UI ownership checked before host or X11 work");
    panel->removed();panel->release();
    check(frame.closes==priorCloses+1 && frame.timers.empty() && frame.references==1,
          "repeated removals balance exact close, timers and host references");
  }
  const auto refusedParent=XCreateSimpleWindow(display,DefaultRootWindow(display),0,0,1,1,0,0,0);
  controller=new Vst::EditController;panel=new AP11::VendorPanel(controller,frame,24);controller->release();
  const auto priorOpens=frame.opens;
  panel->setFrame(&frame);
  check(panel->attached(reinterpret_cast<void*>(refusedParent),kPlatformTypeX11EmbedWindowID)!=kResultOk &&
        frame.opens==priorOpens && frame.timers.empty(),"host without close protocol refused before opening");
  panel->release();XDestroyWindow(display,refusedParent);
  frame.acceptOpen=false;controller=new Vst::EditController;
  panel=new AP11::VendorPanel(controller,frame,25);controller->release();panel->setFrame(&frame);
  check(panel->attached(reinterpret_cast<void*>(parent),kPlatformTypeX11EmbedWindowID)!=kResultOk &&
        frame.timers.empty() && frame.references==1,"unavailable editor control refuses host attachment without blank shell");
  panel->release();
  frame.acceptOpen=true;frame.refuseRegister=1;controller=new Vst::EditController;
  panel=new AP11::VendorPanel(controller,frame,26);controller->release();panel->setFrame(&frame);
  const auto registrationOpens=frame.opens, unregisterBefore=frame.unregisterCalls;
  check(panel->attached(reinterpret_cast<void*>(parent),kPlatformTypeX11EmbedWindowID)!=kResultOk &&
        frame.opens==registrationOpens && frame.unregisterCalls==unregisterBefore && frame.timers.empty(),
        "registration refusal never claims a timer or editor owner");panel->release();
  // Releasing after a refused remove must detach the owner, not leave a
  // dangling callback. The independent registration completes on the loop.
  frame.acceptOpen=true;controller=new Vst::EditController;
  panel=new AP11::VendorPanel(controller,frame,26);controller->release();panel->setFrame(&frame);
  check(panel->attached(reinterpret_cast<void*>(parent),kPlatformTypeX11EmbedWindowID)==kResultOk,"abandon fixture attach");
  frame.refuseUnregister=2;check(panel->removed()!=kResultOk,"abandon unregister refused");
  panel->release();frame.drain();check(frame.timers.empty() && frame.references==1,"destructor leaves no dangling timer; detached receipt retires");
  struct Fault final : AP11::ShellFaults {
    AP11::ShellOperation operation=AP11::ShellOperation::SendClose;
    AP11::ShellResult result=AP11::ShellResult::Transient;
    unsigned remaining=1, sends=0, unmaps=0;
    AP11::ShellResult before(AP11::ShellOperation op) override {
      if(op==AP11::ShellOperation::SendClose)++sends;
      if(op==AP11::ShellOperation::Unmap)++unmaps;
      if(op==operation && remaining) { --remaining; return result; }
      return AP11::ShellResult::Accepted;
    }
  } fault;
  auto attach=[&](uint64_t token) {
    fault.sends=fault.unmaps=0;
    controller=new Vst::EditController;panel=new AP11::VendorPanel(controller,frame,token,&fault);
    controller->release();panel->setFrame(&frame);frame.lifecycle=AP11::Opening;
    check(panel->attached(reinterpret_cast<void*>(parent),kPlatformTypeX11EmbedWindowID)==kResultOk,"fault fixture attach");
  };
  auto closeCount=[&] {unsigned n=0;while(XPending(display)){XEvent e{};XNextEvent(display,&e);
    if(e.type==ClientMessage) {
      check(e.xclient.window==parent && e.xclient.message_type==protocols && e.xclient.format==32 &&
            Atom(e.xclient.data.l[0])==close,"exact WM_DELETE to the host-owned parent");
      ++n;
    }}return n;};
  closeCount();attach(27);frame.lifecycle=AP11::ClosedByVendor;frame.drain();XSync(display,False);
  check(closeCount()==1,"transient close send refusal retries to one accepted WM_DELETE");panel->removed();panel->release();
  fault.operation=AP11::ShellOperation::Unmap;fault.result=AP11::ShellResult::Rejected;fault.remaining=100;
  XSync(display,False);check(XGetWindowAttributes(display,parent,&attributes),"live parent before unmap refusal");
  const auto baselineMask=attributes.all_event_masks;
  const auto closeBefore=frame.closes, unregisterBeforeUnmap=frame.unregisterCalls;
  attach(28);controller->addRef();auto failBefore=frame.failures;
  XMapWindow(display,parent);XSync(display,False);frame.drain();XSync(display,False);
  check(frame.failures==failBefore+1,"failed unmap reports editor failure instead of hidden-shell success");
  check(closeCount()==1,"rejected unmap still delivers one accepted host retirement");
  frame.drain();XSync(display,False);
  check(closeCount()==0 && frame.failures==failBefore+1 && fault.sends==1 && fault.unmaps==1,
        "later turns duplicate neither host retirement nor editor failure and never retry rejected unmap");
  check(panel->removed()==kResultOk && panel->removed()==kResultOk && frame.closes==closeBefore+1 &&
        frame.unregisterCalls==unregisterBeforeUnmap+1 && frame.timers.empty() && frame.references==1,
        "normal removal balances exact close, timer, frame and loop after unmap refusal");
  panel->release();check(controller->release()==0,"unmap failure leaves no controller reference");
  XSync(display,False);check(XGetWindowAttributes(display,parent,&attributes) && attributes.all_event_masks==baselineMask,
        "host parent survives and delegate XCB event selection retires with connection");
  XUnmapWindow(display,parent);XSync(display,False);
  fault.result=AP11::ShellResult::Transient;fault.remaining=100;attach(32);failBefore=frame.failures;
  XMapWindow(display,parent);XSync(display,False);frame.drain();XSync(display,False);
  check(frame.failures==failBefore+1 && fault.unmaps==3 && fault.sends==1 && closeCount()==1,
        "exhausted transient unmap still delivers one checked host retirement");panel->removed();panel->release();
  fault.operation=AP11::ShellOperation::SendClose;fault.result=AP11::ShellResult::Rejected;fault.remaining=100;
  attach(33);failBefore=frame.failures;frame.lifecycle=AP11::ClosedByVendor;frame.drain();XSync(display,False);
  check(frame.failures==failBefore+1 && fault.sends==1 && closeCount()==0,
        "conclusively rejected host-close send is not retried");panel->removed();panel->release();
  fault.operation=AP11::ShellOperation::Flush;fault.result=AP11::ShellResult::ConnectionLost;fault.remaining=1;
  attach(29);failBefore=frame.failures;frame.lifecycle=AP11::ClosedByVendor;frame.drain();XSync(display,False);
  check(frame.failures==failBefore+1 && closeCount()==1 && fault.sends==1,
        "flush connection loss after accepted send retires editor without duplicate sends");panel->removed();panel->release();
  fault.operation=AP11::ShellOperation::SendClose;fault.result=AP11::ShellResult::ConnectionLost;fault.remaining=1;
  attach(31);failBefore=frame.failures;frame.lifecycle=AP11::ClosedByVendor;frame.drain();XSync(display,False);
  check(frame.failures==failBefore+1 && closeCount()==0 && panel->shellResult()==AP11::ShellResult::ConnectionLost,
        "connection failure before host-close send cannot claim accepted retirement");panel->removed();panel->release();
  fault.remaining=0;attach(30);failBefore=frame.failures;XDestroyWindow(display,parent);XSync(display,False);
  frame.lifecycle=AP11::ClosedByVendor;frame.drain();
  check(frame.failures==failBefore+1 && panel->shellResult()==AP11::ShellResult::ParentGone,
        "parent disappearance has exact loss classification");panel->removed();panel->release();
  check(frame.references==1 && frame.timers.empty(),"fault cycles restore all host references");
  XCloseDisplay(display);
  std::cout
      << "AP15 production X11 delegate ownership and retirement PASS\n";
}
