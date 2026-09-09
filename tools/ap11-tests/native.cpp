// Production native controller + processor IMessage seam, with a bounded
// backend double. Real Windows view behavior is checked by windows.cpp.
#include "ap4_backend.h"
#include "commercial_controller.h"
#include "processor.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include <deque>
#include <iostream>
#include <memory>
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
void check(bool v, const char *why) {
  if (!v) {
    std::cerr << "FAIL: " << why << '\n';
    std::exit(1);
  }
}
std::deque<ap11_gui_message_t> events;
std::vector<ap11_gui_message_t> commands;
uint32_t gui_fault = 0, caps = 0, closes = 0;
uint64_t generation = 7, revision = 1;
double dsp = .5;
uint32_t save_code=0, state_calls=0;
std::thread::id owner = std::this_thread::get_id();
struct Host final : HostApplication,
                    Linux::IRunLoop,
                    IComponentHandler,
                    IComponentHandler2 {
  std::vector<Linux::ITimerHandler *> timers;
  std::vector<uint32_t> gestures;
  AP8::Controller *controller = nullptr;
  AP2::Processor *processor = nullptr;
  bool flush = true, reentrant = false;
  uint32 dirty = 0, restarts = 0;
  tresult PLUGIN_API queryInterface(const TUID id, void **out) override {
    if (!out)
      return kInvalidArgument;
    *out = nullptr;
    if (FUnknownPrivate::iidEqual(id, Linux::IRunLoop::iid))
      *out = static_cast<Linux::IRunLoop *>(this);
    else if (FUnknownPrivate::iidEqual(id, IComponentHandler::iid))
      *out = static_cast<IComponentHandler *>(this);
    else if (FUnknownPrivate::iidEqual(id, IComponentHandler2::iid))
      *out = static_cast<IComponentHandler2 *>(this);
    else
      return HostApplication::queryInterface(id, out);
    addRef();
    return kResultOk;
  }
  uint32 PLUGIN_API addRef() override { return HostApplication::addRef(); }
  uint32 PLUGIN_API release() override { return HostApplication::release(); }
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
    check(i != timers.end(), "registered timer ownership");
    timers.erase(i);
    t->release();
    return kResultOk;
  }
  void tick() {
    auto copy = timers;
    for (auto *t : copy) {
      t->addRef();
      t->onTimer();
      t->release();
    }
  }
  void audio(double value) {
    ParameterChanges changes;
    int32 index = 0;
    auto *q = changes.addParameterData(0, index);
    q->addPoint(0, value, index);
    ProcessData d{};
    d.symbolicSampleSize = kSample32;
    d.processMode = kRealtime;
    d.inputParameterChanges = &changes;
    check(processor->process(d) == kResultOk,
          "real native zero-frame host parameter flush");
  }
  tresult PLUGIN_API beginEdit(ParamID id) override {
    check(std::this_thread::get_id() == owner && id == 0,
          "host gesture owner and ID");
    gestures.push_back(AP11::Begin);
    return kResultOk;
  }
  tresult PLUGIN_API performEdit(ParamID id, ParamValue value) override {
    check(std::this_thread::get_id() == owner && id == 0,
          "host value owner and ID");
    gestures.push_back(AP11::Value);
    check(controller->setParamNormalized(id, value) == kResultOk,
          "synchronous host echo");
    if (reentrant)
      tick();
    if (flush)
      audio(value);
    return kResultOk;
  }
  tresult PLUGIN_API endEdit(ParamID id) override {
    check(id == 0, "end ID");
    gestures.push_back(AP11::End);
    return kResultOk;
  }
  tresult PLUGIN_API restartComponent(int32 flags) override {
    restarts |= flags;
    if (flags & kParamValuesChanged)
      audio(controller->getParamNormalized(0));
    return kResultOk;
  }
  tresult PLUGIN_API setDirty(TBool value) override {
    dirty += value ? 1 : 0;
    return kResultOk;
  }
  tresult PLUGIN_API requestOpenEditor(FIDString) override { return kResultOk; }
  tresult PLUGIN_API startGroupEdit() override {
    gestures.push_back(AP11::GroupBegin);
    return kResultOk;
  }
  tresult PLUGIN_API finishGroupEdit() override {
    gestures.push_back(AP11::GroupEnd);
    return kResultOk;
  }
};
void event(uint32_t kind, double value = 0) {
  ap11_gui_message_t e{};
  e.kind = kind;
  e.value = value;
  e.revision = revision++;
  events.push_back(e);
}
} // namespace
extern "C" {
uint32_t __wrap_ap9_open(const uint8_t *, uint64_t *out) {
  *out = 1;
  return 0;
}
uint32_t __wrap_ap5_report_path(uint64_t, uint8_t *p, uint32_t n) {
  if (n)
    *p = 0;
  return 0;
}
uint32_t __wrap_ap10_setup(uint64_t, uint32_t, uint32_t, double,
                           const uint8_t *, uint32_t, uint32_t, uint32_t *t) {
  t[0] = 512;
  t[1] = t[2] = 0;
  return 0;
}
uint32_t __wrap_ap4_activate(uint64_t, uint32_t, uint32_t) { return 0; }
uint32_t __wrap_ap4_deactivate(uint64_t) { return 0; }
uint32_t __wrap_ap3_transition(uint64_t, uint32_t) { return 0; }
uint32_t __wrap_ap3_close(uint64_t) {
  ++closes;
  return 0;
}
uint32_t __wrap_ap10_notices(uint64_t, uint32_t *p) {
  p[0] = 0;
  return 0;
}
uint32_t __wrap_ap11_gui_generation(uint64_t, uint64_t *out) {
  *out = generation;
  return 0;
}
uint32_t __wrap_ap11_gui_command(uint64_t, uint64_t g, ap11_gui_message_t *m) {
  if (g != generation)
    return 5;
  m->revision = revision++;
  commands.push_back(*m);
  return 0;
}
uint32_t __wrap_ap11_gui_take(uint64_t, uint64_t g, ap11_gui_message_t *m) {
  if (g != generation)
    return 5;
  *m = ap11_gui_message_t{};
  if (!events.empty()) {
    *m = events.front();
    events.pop_front();
  }
  return 0;
}
uint32_t __wrap_ap11_gui_capabilities(uint64_t, uint64_t g, uint32_t c) {
  if (g != generation)
    return 5;
  caps = c;
  return 0;
}
uint32_t __wrap_ap11_gui_failure(uint64_t, uint64_t g, uint32_t c) {
  if (g != generation)
    return 5;
  if (c && !gui_fault)
    gui_fault = c;
  return gui_fault;
}
uint32_t __wrap_ap8_validate(const uint8_t *, const uint8_t *, uint32_t) {
  return 0;
}
uint32_t __wrap_ap4_state(uint64_t, const uint8_t *, uint32_t, uint8_t *out,
                          uint32_t cap, uint32_t *n) {
  ++state_calls;if(save_code)return save_code;
  check(cap >= 132, "bounded state output");
  std::memset(out, 0, 132);
  out[112] = 1;
  std::memcpy(out + 124, &dsp, 8);
  *n = 132;
  return 0;
}
uint32_t __wrap_ap10_process(uint64_t, uint32_t n, const ap8_event_t *e,
                             uint32_t count, const ap10_context_t *, uint64_t,
                             const float *, const float *, float *, float *,
                             uint64_t *silence, ap7_delivery_t *delivery) {
  check(n == 0, "stopped flush is zero frames");
  for (uint32_t i = 0; i < count; ++i)
    if (e[i].kind == 2) {
      check(e[i].id == 0, "real parameter ID");
      dsp = e[i].value;
    }
  *silence = 0;
  *delivery = {};
  return 0;
}
uint32_t __wrap_ap10_take_results(uint64_t, ap10_results_t *p) {
  *p = ap10_results_t{};
  return 0;
}
}
int main() {
  Host host;
  auto processor = std::make_unique<AP2::Processor>();
  auto *c = new AP8::Controller;
  host.controller = c;
  host.processor = processor.get();
  auto *context = static_cast<IHostApplication *>(&host);
  check(processor->initialize(context) == kResultOk &&
            c->initialize(context) == kResultOk,
        "SDK initialization");
  check(c->setComponentHandler(static_cast<IComponentHandler *>(&host)) ==
            kResultOk,
        "host handlers");
  check(processor->connect(c) == kResultOk &&
            c->connect(processor.get()) == kResultOk && caps == 7,
        "production connection and generation capabilities");
  ProcessSetup setup{kRealtime, kSample32, 128, 48000};
  check(processor->setupProcessing(setup) == kResultOk &&
            processor->setActive(true) == kResultOk &&
            processor->setProcessing(true) == kResultOk,
        "start processing");
  check(state_calls==0&&commands.size()==1&&commands[0].kind==AP11::Refresh&&commands[0].flags==0,"fresh native bootstrap requires no opaque saving or restart");
  commands.clear();
  c->panelOpen();
  check(commands.size() == 1 && commands[0].kind == AP11::Open,
        "native entry opens same session without inventing a refresh");
  c->panelOpen({123, 456});
  host.tick();
  check(commands.size() == 2 && commands[1].kind == AP11::Open &&
            commands[1].user_time == 123 && commands[1].requestor_x11 == 456 &&
            commands[1].activation > commands[0].activation &&
            host.restarts == 0 && host.gestures.empty() && dsp == .5,
        "repeat focus preserves click context without invalidating parameters");
  commands.clear();
  host.reentrant = true;
  event(AP11::GroupBegin);
  event(AP11::Begin);
  event(AP11::Value, .25);
  event(AP11::Value, .75);
  event(AP11::End);
  event(AP11::GroupEnd);
  event(AP11::Dirty, 1);
  host.tick();
  check(host.gestures == std::vector<uint32_t>({105, 101, 102, 102, 103, 106}),
        "ordered gestures survive reentrant polling");
  check(dsp == .75 && host.dirty == 1 && commands.empty(),
        "host flush applies edits once without controller echo");
  // A host automation update changes the real controller only. The host still
  // owns the separate DSP automation submission.
  check(c->setParamNormalized(0, .4) == kResultOk && commands.size() == 1 &&
            commands[0].kind == AP11::Set && dsp == .75,
        "reverse update is not duplicate DSP");
  host.audio(.4);
  check(dsp == .4, "host playback DSP submission");
  ap11_gui_message_t begin{};
  begin.kind = AP11::RefreshBegin;
  begin.count = 1;
  begin.revision = revision++;
  begin.flags = 20;
  events.push_back(begin);
  ap11_gui_message_t param{};
  param.kind = AP11::Parameter;
  param.id = 0;
  param.revision = begin.revision;
  param.value = .6;
  param.flags = 1;
  param.steps = 0;
  std::copy_n(u"Actual preset gain", 19, param.title);
  events.push_back(param);
  auto end = begin;
  end.kind = AP11::RefreshEnd;
  events.push_back(end);
  host.tick();
  ParameterInfo info{};
  check(c->getParameterInfo(0, info) == kResultOk && info.title[0] == u'A' &&
            host.restarts == 20 && dsp == .6,
        "vendor metadata/value refresh through SDK restart");
  // An unavailable refresh still completes every ID and preserves only stale
  // presentation. It neither forwards invalid data nor poisons another control.
  begin.revision=revision++;param.revision=begin.revision;end.revision=begin.revision;
  param.result=1;param.value=0;
  events.push_back(begin);events.push_back(param);events.push_back(end);host.tick();
  check(!c->readbackAvailable(0)&&c->getParamNormalized(0)==.6&&gui_fault==0,"unavailable refresh completes and retains explicitly stale display");
  param.result=0;param.value=.65;begin.revision=revision++;param.revision=begin.revision;end.revision=begin.revision;
  events.push_back(begin);events.push_back(param);events.push_back(end);host.tick();
  check(c->readbackAvailable(0)&&c->getParamNormalized(0)==.65,"later genuine value becomes available");
  save_code=5;LVBState::Stream refused;
  check(processor->getState(&refused)==kResultFalse&&refused.bytes.empty(),"ordinary save refusal is truthful");
  check(std::strstr(c->panelStatus(),"Saving unavailable")!=nullptr,"fresh audition exposes save-unavailable status");
  host.audio(.7);check(dsp==.7,"audio continues after failed save");
  c->panelOpen();check(commands.back().kind==AP11::Open,"editor continues after refused save");
  save_code=0;
  LVBState::Stream projected;projected.bytes.resize(136);projected.bytes[64]=32;projected.bytes[112]=1;projected.bytes[116]=2;
  check(c->setComponentState(&projected)==kResultOk&&!c->readbackAvailable(0)&&c->getParamNormalized(0)==.65,"native tagged state apply preserves unavailable ID and stale projection");
  // Saving drains a complete accepted edit before taking the state barrier.
  event(AP11::Begin);
  event(AP11::Value, .9);
  event(AP11::End);
  LVBState::Stream saved;
  check(processor->getState(&saved) == kResultOk && dsp == .9,
        "stopped editing and immediate save drains UI and host flush");
  c->panelClose();
  c->panelOpen();
  check(closes == 0 && commands.back().kind == AP11::Open,
        "view close/reopen retains processing instance");
  c->disconnect(processor.get());
  // The host may capture state after only one side of the connection has
  // disconnected its controller; the processor still has a live SDK peer.
  // A final window-close acknowledgement is still queued in the shared UI ring.
  // It cannot be delivered to the departed peer and must not poison GUI state.
  gui_fault = 0;
  events.clear();
  event(AP11::EditorStatus);
  LVBState::Stream afterDisconnect;
  check(processor->getState(&afterDisconnect) == kResultOk && gui_fault == 0 &&
            events.size() == 1,
        "post-disconnect state capture does not consume or fail UI delivery");
  events.clear();
  check(c->connect(processor.get()) == kResultOk, "restore controller peer");
  // The opposite SDK disconnect order must also deliver the close request.
  // Its response no longer has a processor-to-controller route.
  check(processor->disconnect(c) == kResultOk, "processor disconnects first");
  auto commandsBeforeRetire = commands.size();
  check(c->disconnect(processor.get()) == kResultOk && gui_fault == 0 &&
            commands.size() == commandsBeforeRetire + 1 &&
            commands.back().kind == AP11::Close && caps == 0,
        "processor-first retirement closes view without a false host failure");
  check(processor->connect(c) == kResultOk &&
            c->connect(processor.get()) == kResultOk,
        "restore both SDK peers");
  event(AP11::Begin);
  host.tick();
  gui_fault = AP11::Backlog;
  host.tick();
  check(host.gestures.back() == AP11::End,
        "overload explicitly completes active gesture");
  host.audio(.3);
  check(dsp == .3, "GUI failure leaves native audio submission usable");
  ++generation;
  check(c->setParamNormalized(0, .2) == kResultFalse,
        "failed UI update refuses");
  check(processor->setProcessing(false) == kResultOk &&
            processor->setActive(false) == kResultOk,
        "stop");
  c->disconnect(processor.get());
  processor->disconnect(c);
  c->terminate();
  check(host.timers.empty(), "timer detached before controller release");
  c->release();
  processor->terminate();
  check(closes == 1, "only final teardown closes DSP");
  std::cout << "AP11 production native SDK gestures, echo, refresh, zero-frame "
               "save, failure, lifetime PASS\n";
}
