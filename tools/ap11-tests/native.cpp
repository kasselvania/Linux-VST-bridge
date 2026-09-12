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
uint32_t gui_fault = 0, caps = 0, closes = 0, refused_sends = 0;
uint64_t generation = 7, revision = 1;
double dsp = .5;
uint32_t save_code=0, state_calls=0;
if1_terminal_t terminal_record{};
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
  uint32 dirty = 0, restarts = 0, reload_calls = 0;
  unsigned refuse_allocations = 0;
  tresult PLUGIN_API createInstance(TUID cid, TUID iid, void **out) override {
    if (refuse_allocations) { --refuse_allocations; *out = nullptr; return kResultFalse; }
    return HostApplication::createInstance(cid, iid, out);
  }
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
    if(flags & kReloadComponent) ++reload_calls;
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
uint32_t __wrap_if1_terminal(uint64_t,if1_terminal_t* out){*out=terminal_record;return 0;}
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
  if (refused_sends) { --refused_sends; return 3; }
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
uint32_t __wrap_ap13_process(uint64_t, uint32_t n, const ap8_event_t *e,
                             uint32_t count, const ap10_context_t *, uint64_t,
                             const float *, const float *, float *, float *,
                             uint64_t *silence, ap7_delivery_t *delivery, uint64_t entered_ns) {
  check(entered_ns != 0, "native callback entry is propagated");
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
void lifecycle_identity_regression() {
  uint64_t exhausted=UINT64_MAX-1;
  check(AP15::advance(exhausted)==UINT64_MAX && !AP15::advance(exhausted) &&
        exhausted==UINT64_MAX,"identity exhaustion never wraps or reuses zero");
  AP15::EditorLifecycle owner;
  const auto a = owner.allocate();
  ap11_gui_message_t open{}, close{};
  check(owner.begin(a,1,{111,222},open) && open.native_view==a && open.view_epoch==0,
        "native view and activation identities are explicit before binding");
  check(owner.state(a)==AP11::Absent,"preparing open does not claim delivery");
  owner.opened(open);
  auto status=open;status.kind=AP11::EditorStatus;status.count=1;status.view_epoch=1;
  status.lifecycle=AP11::AwaitingFocus;
  check(owner.status(status),"active editor epoch acknowledged");
  auto stale=status;stale.native_view++;
  check(!owner.status(stale),"status from another native view refused");
  stale=status;stale.activation++;
  check(!owner.status(stale),"status from another activation refused");
  status.count=0;status.lifecycle=AP11::ClosedByVendor;
  check(owner.status(status) && owner.state(a)==AP11::ClosedByVendor,"vendor close retires only editor");
  stale=status;stale.count=1;stale.lifecycle=AP11::Focused;
  check(!owner.status(stale),"late focus cannot resurrect a closed editor");
  check(owner.close(a,close) && close.native_view==a && close.view_epoch==1 &&
        owner.owner()==a,"pending retirement retains exact owner");
  owner.closed(close);
  check(!owner.close(a,close),"accepted retirement closes once");
  check(owner.accepts_result(close),"close command failure remains observable for retired owner");
  const auto b=owner.allocate();
  check(b>a && owner.begin(b,2,{},open),"fresh native view permits one replacement opening");
  owner.opened(open);
  check(!owner.accepts_result(close),"late retired close result cannot poison replacement");
  check(!owner.close(a,close) && owner.state(b)==AP11::Opening && !owner.status(status),
        "retired native view close and status cannot affect replacement");
  status=open;status.kind=AP11::EditorStatus;status.view_epoch=2;status.count=1;
  status.lifecycle=AP11::Focused;
  auto reused=status;reused.view_epoch=1;
  check(!owner.status(reused),"new native view cannot reuse a prior editor epoch");
  check(owner.status(status),"replacement editor advances epoch");
  status.view_epoch=1;
  check(!owner.status(status),"old editor epoch refused even with current activation");
  status.view_epoch=2;status.abi_version=2;
  check(!owner.status(status),"unsupported UI ABI refused");
  owner.sessionRetired();
  check(owner.state(b)==AP11::EditorFailed && !owner.begin(b,3,{},open),
        "old attached view keeps terminal state after processing replacement");
  const auto fresh=owner.allocate();
  check(owner.begin(fresh,4,{},open),"fresh native token can use replacement UI session");
  owner.opened(open);
  check(!owner.close(b,close) && !owner.status(status) && owner.owner()==fresh,
        "retired session cannot close or update replacement");
}
void terminal_instance_regression() {
 for(uint64_t failureClass : {1u,2u,3u}) {
  events.clear();commands.clear();gui_fault=0;refused_sends=0;terminal_record={};
  Host host;auto processor=std::make_unique<AP2::Processor>();auto* c=new AP8::Controller;
  host.controller=c;host.processor=processor.get();auto* context=static_cast<IHostApplication*>(&host);
  check(processor->initialize(context)==kResultOk&&c->initialize(context)==kResultOk,"IF1 initialize");
  check(c->setComponentHandler(static_cast<IComponentHandler*>(&host))==kResultOk,"IF1 handler");
  check(processor->connect(c)==kResultOk&&c->connect(processor.get())==kResultOk,"IF1 connection");
  auto token=c->allocateEditorView();check(c->panelOpen(token),"IF1 initial vendor owner");
  auto open=commands.back();open.kind=AP11::EditorStatus;open.count=1;open.view_epoch=1;open.lifecycle=AP11::Opened;
  events.push_back(open);host.tick();auto begin=open;begin.kind=AP11::Begin;events.push_back(begin);host.tick();
  terminal_record.words[0]=1;terminal_record.words[1]=123;terminal_record.words[2]=456;
  terminal_record.words[3]=generation;terminal_record.words[4]=2;terminal_record.words[5]=104687;
  terminal_record.words[6]=512;terminal_record.words[7]=9;terminal_record.words[14]=failureClass;
  terminal_record.words[15]=90;terminal_record.words[18]=failureClass;terminal_record.words[19]=failureClass;
  const auto count=commands.size();host.tick();
  check(host.reload_calls==1&&host.gestures.back()==AP11::End,"IF1 completes gesture and asks reload once");
  check(c->panelClose(token)&&!c->panelOpen(token)&&commands.size()==count,"IF1 dead generation never forwarded");
  auto* view=c->createView(ViewType::kEditor);ViewRect rect{};
  check(view&&view->getSize(&rect)==kResultOk&&rect.getWidth()==640,"IF1 native failure/recovery view instead of VendorPanel");view->release();
  for(unsigned i=0;i<4;++i){HostMessage poll;poll.setMessageID("AP10.poll");check(processor->notify(&poll)==kResultOk,"IF1 repeated bounded poll");}
  check(host.reload_calls==1&&commands.size()==count,"IF1 exact once and no late forwarding");
  check(c->setParamNormalized(0,.75)==kResultFalse,"IF1 refuses dead parameter forwarding");
  check(c->disconnect(processor.get())==kResultOk,"IF1 disconnect retires editor owner");
  processor->disconnect(c);check(c->terminate()==kResultOk&&host.timers.empty(),"IF1 no retained timer");c->release();processor->terminate();
 }
 terminal_record={};
}
int main() {
  lifecycle_identity_regression();
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
  auto refusedToken = c->allocateEditorView();
  host.refuse_allocations = 1;
  check(!c->panelOpen(refusedToken) && c->panelState(refusedToken)==AP11::Absent && commands.empty(),
        "initial message allocation refusal leaves no Opening/token owner");
  refused_sends=1;
  check(!c->panelOpen(refusedToken) && c->panelState(refusedToken)==AP11::Absent && commands.empty() && !gui_fault,
        "transient Open send refusal is truthful and retryable");
  check(c->panelOpen(refusedToken) && commands.size()==1,"retry accepts initial Open exactly once");
  host.refuse_allocations=1;
  check(c->panelClose(refusedToken) && commands.size()==2 && commands.back().kind==AP11::Close,
        "Close uses reserved retirement message despite allocation refusal");
  host.refuse_allocations=0;
  check(c->panelClose(refusedToken) && commands.size()==2,"duplicate accepted Close is a no-op");
  auto pendingToken=c->allocateEditorView();check(c->panelOpen(pendingToken),"pending-close fixture Open");
  const auto beforePending=commands.size();refused_sends=2;
  check(!c->panelClose(pendingToken) && c->panelState(pendingToken)==AP11::Closing &&
        c->disconnect(processor.get())!=kResultOk && commands.size()==beforePending,
        "failed close and disconnect preserve pending exact owner and connection");
  host.tick();
  check(commands.size()==beforePending+1 && commands.back().native_view==pendingToken &&
        commands.back().kind==AP11::Close && c->panelState(pendingToken)==AP11::Absent,
        "one bounded UI retry accepts exact Close");
  host.tick();check(commands.size()==beforePending+1,"accepted retry does not duplicate Close");
  auto termToken=c->allocateEditorView();check(c->panelOpen(termToken),"termination fixture Open");
  refused_sends=1;
  check(c->terminate()!=kResultOk && c->panelState(termToken)==AP11::Closing,
        "terminate refuses while exact editor retirement is undelivered");
  host.tick();check(c->panelState(termToken)==AP11::Absent,"termination pending close can retire safely");
  auto failedToken=c->allocateEditorView();check(c->panelOpen(failedToken),"contained retirement fixture Open");
  refused_sends=8;const auto beforeContainment=commands.size();
  check(!c->panelClose(failedToken),"close refusal retains ownership before bounded containment");
  for(unsigned i=0;i<8;++i) host.tick();
  check(gui_fault==AP11::Removal && c->panelState(failedToken)==AP11::Absent &&
        commands.size()==beforeContainment,"eight failed sends use explicit GUI failure retirement without duplicate Close");
  host.audio(.5);check(dsp==.5 && !closes,"retirement containment leaves DSP callable");
  gui_fault=0;++generation;
  HostMessage rebound;rebound.setMessageID("AP11.identity");rebound.getAttributes()->setInt("generation",generation);
  check(c->notify(&rebound)==kResultOk,"new UI generation can recover after explicit editor containment");
  commands.clear();
  auto viewToken = c->allocateEditorView();
  c->panelOpen(viewToken);
  check(commands.size() == 1 && commands[0].kind == AP11::Open,
        "native entry opens same session without inventing a refresh");
  c->panelOpen(viewToken, {123, 456});
  host.tick();
  check(commands.size() == 2 && commands[1].kind == AP11::Open &&
            commands[1].user_time == 123 && commands[1].requestor_x11 == 456 &&
            commands[1].activation > commands[0].activation &&
            host.restarts == 0 && host.gestures.empty() && dsp == .5,
        "repeat focus preserves click context without invalidating parameters");
  auto refusedEditor=commands.back();refusedEditor.kind=AP11::EditorStatus;
  refusedEditor.count=0;refusedEditor.result=AP11::NoView;refusedEditor.lifecycle=AP11::OpenRefused;
  events.push_back(refusedEditor);host.tick();
  check(c->panelState(viewToken)==AP11::OpenRefused && !gui_fault && !closes,
        "open refusal remains an editor result with the same processing session");
  host.audio(.5);
  check(dsp==.5,"audio remains callable after editor refusal");
  c->panelClose(viewToken);viewToken=c->allocateEditorView();c->panelOpen(viewToken);
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
  c->panelOpen(viewToken);check(commands.back().kind==AP11::Open,"editor continues after refused save");
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
  c->panelClose(viewToken);
  viewToken = c->allocateEditorView();
  c->panelOpen(viewToken);
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
  viewToken=c->allocateEditorView();c->panelOpen(viewToken);
  check(commands.back().kind==AP11::Open && commands.back().native_view==viewToken,
        "a fresh owned editor is active before opposite disconnect order");
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
  auto a=c->allocateEditorView();check(c->panelOpen(a),"generation A opens");
  auto old=commands.back();old.kind=AP11::EditorStatus;old.count=1;old.view_epoch=1;old.lifecycle=AP11::Opened;
  events.push_back(old);host.tick();
  auto edit=old;edit.kind=AP11::Begin;events.push_back(edit);host.tick();
  ++generation;
  HostMessage identity;identity.setMessageID("AP11.identity");identity.getAttributes()->setInt("generation",generation);
  check(c->notify(&identity)==kResultOk && c->panelState(a)==AP11::EditorFailed && !gui_fault,
        "processing generation replacement retains terminal A without poisoning DSP");
  check(c->panelClose(a),"old generation removal acknowledges already retired UI mapping");
  auto b=c->allocateEditorView();check(b>a && c->panelOpen(b),"generation B opens with fresh native token");
  auto current=commands.back();current.kind=AP11::EditorStatus;current.count=1;current.view_epoch=1;current.lifecycle=AP11::Opened;
  events.push_back(current);host.tick();
  const auto gesturesBefore=host.gestures;const auto valueBefore=c->getParamNormalized(0);const auto dirtyBefore=host.dirty;
  for(auto kind:{AP11::GroupBegin,AP11::Begin,AP11::Value,AP11::End,AP11::GroupEnd,AP11::Dirty}) {
    edit=old;edit.kind=kind;edit.value=1;edit.revision=revision++;events.push_back(edit);
  }
  events.push_back(old);host.tick();
  check(host.gestures==gesturesBefore && c->getParamNormalized(0)==valueBefore && host.dirty==dirtyBefore &&
        c->panelState(b)==AP11::Opened && !gui_fault,"delayed A callbacks and status cannot mutate B");
  check(!c->panelOpen(a) && c->panelClose(a) && c->panelState(b)==AP11::Opened,
        "stale A lifecycle calls leave B unchanged");
  event(AP11::Begin);
  host.tick();
  gui_fault = AP11::Backlog;
  host.tick();
  check(host.gestures.back() == AP11::End,
        "overload explicitly completes active gesture");
  host.audio(.3);
  check(dsp == .3, "GUI failure leaves native audio submission usable");
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
  terminal_instance_regression();
  std::cout << "AP11 production native SDK gestures, echo, refresh, zero-frame "
               "save, failure, lifetime PASS\n";
}
