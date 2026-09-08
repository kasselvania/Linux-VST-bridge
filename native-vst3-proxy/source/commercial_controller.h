#pragma once
#include "../../vst-state/stream.h"
#include "ap10_backend.h"
#include "ap11_gui.h"
#include "ap8_descriptor.h"
#include "desktop_activation.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "vendor_panel.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cmath>
#include <thread>
namespace AP8 {
class Controller final : public Steinberg::Vst::EditController,
                         public AP11::PanelOwner {
  using Result = Steinberg::tresult;
  struct ParameterState {
    uint32_t id;
    uint64_t revision = 0;
    bool editing = false, refreshed = false, available = false;
  };
  struct Timer final : Steinberg::Linux::ITimerHandler {
    Controller *parent;
    std::atomic<Steinberg::uint32> refs{1};
    explicit Timer(Controller *p) : parent(p) {}
    Result PLUGIN_API queryInterface(const Steinberg::TUID id,
                                     void **out) override {
      if (!out)
        return Steinberg::kInvalidArgument;
      *out = nullptr;
      if (Steinberg::FUnknownPrivate::iidEqual(
              id, Steinberg::Linux::ITimerHandler::iid) ||
          Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::FUnknown::iid)) {
        *out = static_cast<Steinberg::Linux::ITimerHandler *>(this);
        addRef();
        return Steinberg::kResultOk;
      }
      return Steinberg::kNoInterface;
    }
    Steinberg::uint32 PLUGIN_API addRef() override { return ++refs; }
    Steinberg::uint32 PLUGIN_API release() override {
      auto n = --refs;
      if (!n)
        delete this;
      return n;
    }
    void PLUGIN_API onTimer() override {
      if (parent)
        parent->tick();
    }
  };

public:
  ~Controller() override { stopTimer(); }
  Result PLUGIN_API terminate() override {
    panelClose();
    finishGestures();
    capabilities(false);
    connected_ = false;
    pending_count_ = 0;
    generation_ = 0;
    stopTimer();
    return EditController::terminate();
  }
  static Steinberg::FUnknown *create(void *) {
    try {
      return static_cast<Steinberg::Vst::IEditController *>(new Controller);
    } catch (...) {
      return nullptr;
    }
  }
  Result PLUGIN_API initialize(Steinberg::FUnknown *host) override {
    auto r = EditController::initialize(host);
    if (r != Steinberg::kResultOk)
      return r;
    owner_ = std::this_thread::get_id();
    states_.reserve(std::size(AP8::parameters));
    for (const auto &p : AP8::parameters) {
      parameters.addParameter(p.title, p.units, p.steps, p.initial, p.flags,
                              p.id);
      states_.push_back({p.id}); // descriptor/default projection is stale until a live refresh
    }
    std::sort(states_.begin(), states_.end(),
              [](const auto &a, const auto &b) { return a.id < b.id; });
    Steinberg::FUnknownPtr<Steinberg::Linux::IRunLoop> loop(host);
    if (loop)
      panelLoop(loop);
    return Steinberg::kResultOk;
  }
  Result PLUGIN_API
  setComponentHandler(Steinberg::Vst::IComponentHandler *handler) override {
    if (!onOwner())
      return Steinberg::kResultFalse;
    finishGestures();
    auto r = EditController::setComponentHandler(handler);
    capabilities();
    return r;
  }
  Steinberg::IPlugView *PLUGIN_API
  createView(Steinberg::FIDString name) override {
    if (!onOwner() || !name ||
        std::strcmp(name, Steinberg::Vst::ViewType::kEditor))
      return nullptr;
    try {
      return new AP11::VendorPanel(this, *this, AP8::class_name);
    } catch (...) {
      return nullptr;
    }
  }
  Result PLUGIN_API connect(Steinberg::Vst::IConnectionPoint *peer) override {
    auto r = EditController::connect(peer);
    if (r == Steinberg::kResultOk) {
      connected_ = true;
      capabilities();
      request("AP11.bind");
      request();
    }
    return r;
  }
  Result PLUGIN_API
  disconnect(Steinberg::Vst::IConnectionPoint *peer) override {
    panelClose();
    finishGestures();
    capabilities(false);
    connected_ = false;
    generation_ = 0;
    pending_count_ = 0;
    status_ = "Disconnected";
    return EditController::disconnect(peer);
  }
  Result PLUGIN_API setParamNormalized(
      Steinberg::Vst::ParamID id, Steinberg::Vst::ParamValue value) override {
    if (!onOwner() || !std::isfinite(value) || value < 0 || value > 1 ||
        !state(id))
      return Steinberg::kResultFalse;
    auto r = EditController::setParamNormalized(id, value);
    if (r != Steinberg::kResultOk)
      return r;
    // The host may synchronously echo performEdit. That is already the real
    // vendor value; all other host-originated updates must reach its
    // controller.
    if (forwarding_ && id == forward_id_ && value == forward_value_)
      return r;
    ap11_gui_message_t m{};
    m.kind = AP11::Set;
    m.id = id;
    m.value = value;
    return command(m);
  }
  Result PLUGIN_API setComponentState(Steinberg::IBStream *s) override {
    if (!onOwner())
      return Steinberg::kResultFalse;
    try {
      std::vector<uint8_t> b;
      if (!LVBState::readEnvelope(s, b) || !apply(b))
        return Steinberg::kResultFalse;
      return connected_ ? request() : Steinberg::kResultOk;
    } catch (...) {
      return Steinberg::kResultFalse;
    }
  }
  Result PLUGIN_API notify(Steinberg::Vst::IMessage *m) override {
    using namespace Steinberg;
    if (!m || !m->getMessageID() || !onOwner())
      return kResultFalse;
    const char *id = m->getMessageID();
    if (!std::strcmp(id, "AP10.restart")) {
      int64 flags = 0;
      if (!componentHandler ||
          m->getAttributes()->getInt("flags", flags) != kResultOk ||
          flags <= 0 || (flags & ~10))
        return kResultFalse;
      return componentHandler->restartComponent(int32(flags));
    }
    if (!std::strcmp(id, "AP11.identity")) {
      int64 generation = 0;
      if (m->getAttributes()->getInt("generation", generation) != kResultOk ||
          generation < 1)
        return kResultFalse;
      if (generation_ && generation_ != uint64_t(generation)) {
        finishGestures();
        pending_count_ = 0;
        failure_ = 0;
        activation_.cancel();
        refreshing_ = false;
        for (auto &p : states_)
          p.revision = 0;
      }
      bool bootstrap=generation_!=uint64_t(generation);
      generation_ = uint64_t(generation);
      capabilities();
      auto count = pending_count_;
      pending_count_ = 0;
      for (size_t i = 0; i < count; ++i)
        if (command(pending_[i]) != kResultOk)
          return kResultFalse;
      if(bootstrap){ap11_gui_message_t refresh{};refresh.kind=AP11::Refresh;command(refresh);}
      return kResultOk;
    }
    if (!std::strncmp(id, "AP11.", 5)) {
      int64 generation = 0;
      if (m->getAttributes()->getInt("generation", generation) != kResultOk ||
          generation <= 0 || uint64_t(generation) != generation_)
        return kResultFalse;
      if (!std::strcmp(id, "AP11.failed")) {
        int64 code = 0;
        if (m->getAttributes()->getInt("code", code) != kResultOk || code < 1 ||
            code > 11)
          return kResultFalse;
        fail(uint32_t(code), false);
        return kResultOk;
      }
      const void *bytes = nullptr;
      uint32 size = 0;
      const char *key = !std::strcmp(id, "AP11.result") ? "command" : "event";
      if (m->getAttributes()->getBinary(key, bytes, size) != kResultOk ||
          size != sizeof(ap11_gui_message_t))
        return kResultFalse;
      ap11_gui_message_t event{};
      std::memcpy(&event, bytes, sizeof(event));
      if (!std::strcmp(id, "AP11.result")) {
        if (event.result) {
          fail(event.result == 5 ? AP11::Closed : AP11::Backlog);
          return kResultOk;
        }
        if (event.kind == AP11::Set) {
          auto *p = state(event.id);
          if (!p)
            return kResultFalse;
          p->revision = std::max(p->revision, event.revision);
        }
        return kResultOk;
      }
      if (std::strcmp(id, "AP11.event"))
        return kResultFalse;
      if (failure_)
        return kResultOk; // failure has explicitly ended accepted gestures
      auto r = deliver(event);
      if (r != kResultOk)
        fail(AP11::Host);
      return r;
    }
    if (std::strcmp(id, "AP8.readback"))
      return EditController::notify(m);
    const void *p = nullptr;
    uint32 n = 0;
    if (m->getAttributes()->getBinary("state", p, n) != kResultOk ||
        n > LVBState::payloadLimit + LVBState::overhead)
      return kResultFalse;
    try {
      const auto *b = static_cast<const uint8_t *>(p);
      return apply({b, b + n}) ? kResultOk : kResultFalse;
    } catch (...) {
      return kResultFalse;
    }
  }
  // Opaque vendor component/controller bytes remain in the complete bundle.
  Result PLUGIN_API getState(Steinberg::IBStream *s) override {
    uint8_t b[8] = {'L', 'V', 'B', 'C', 2, 0, 0, 0};
    return LVBState::transfer(s, b, 8, true) ? Steinberg::kResultOk
                                             : Steinberg::kResultFalse;
  }
  Result PLUGIN_API setState(Steinberg::IBStream *s) override {
    uint8_t b[8]{}, expected[8] = {'L', 'V', 'B', 'C', 2, 0, 0, 0};
    return LVBState::transfer(s, b, 8, false) && std::equal(b, b + 8, expected)
               ? Steinberg::kResultOk
               : Steinberg::kResultFalse;
  }
  void panelLoop(Steinberg::Linux::IRunLoop *loop) override {
    if (!onOwner() || timer_ || !loop)
      return;
    loop_ = loop;
    loop_->addRef();
    timer_ = new Timer(this);
    if (loop_->registerTimer(timer_, 10) != Steinberg::kResultOk) {
      stopTimer();
      status_ = "Host UI timer unavailable";
    }
    capabilities();
  }
  void panelOpen(AP11::ActivationContext activation = {}) override {
    if (!onOwner() || !connected_ || failure_)
      return;
    if (!timer_ || !componentHandler) {
      status_ = "Host editor control unavailable";
      return;
    }
    status_ = "Opening vendor editor...";
    ap11_gui_message_t m{};
    m.kind = AP11::Open;
    if (activation_serial_ == UINT64_MAX) {
      status_ = "Editor activation identity exhausted";
      return;
    }
    m.activation = ++activation_serial_;
    m.user_time = activation.user_time;
    m.requestor_x11 = activation.requestor_x11;
    activation_.begin(m);
    command(m);
    // Opening/focusing a view is not a parameter invalidation. An unsolicited
    // Refresh causes a host restart notification and, in Bitwig, a full state
    // capture on the serialized audio transport. Real vendor restart callbacks
    // still publish complete value/title refreshes through the UI queue.
  }
  void panelClose() override {
    activation_.cancel();
    if (onOwner() && connected_) {
      ap11_gui_message_t m{};
      m.kind = AP11::Close;
      command(m);
    }
  }
  const char *panelStatus() const override { return status_; }
  bool readbackAvailable(uint32_t id) {auto* p=state(id);return p&&p->available;}

private:
  uint64_t activation_serial_ = 0;
  AP11::DesktopActivation activation_;
  bool onOwner() const { return owner_ == std::this_thread::get_id(); }
  ParameterState *state(uint32_t id) {
    auto p =
        std::lower_bound(states_.begin(), states_.end(), id,
                         [](const auto &p, uint32_t v) { return p.id < v; });
    return p != states_.end() && p->id == id ? &*p : nullptr;
  }
  void finishGestures() {
    for (auto &p : states_)
      if (p.editing) {
        p.editing = false;
        if (componentHandler)
          componentHandler->endEdit(p.id);
      }
    if (group_) {
      group_ = false;
      if (componentHandler2)
        componentHandler2->finishGroupEdit();
    }
  }
  void fail(uint32_t code, bool publish = true) {
    if (failure_)
      return;
    failure_ = code;
    status_ = code == AP11::Closed
                  ? "Processing session closed"
                  : "Editor control failed; close and inspect diagnostics";
    finishGestures();
    if (publish && connected_ && generation_) {
      auto *m = allocateMessage();
      if (m) {
        m->setMessageID("AP11.failure");
        m->getAttributes()->setInt("generation", Steinberg::int64(generation_));
        m->getAttributes()->setInt("code", code);
        sendMessage(m);
        m->release();
      }
    }
  }
  Result deliver(const ap11_gui_message_t &m) {
    using namespace Steinberg;
    using namespace Steinberg::Vst;
    auto *p = state(m.id);
    switch (m.kind) {
    case AP11::Begin:
      if (!p || p->editing || !componentHandler)
        return kResultFalse;
      p->editing = true;
      return componentHandler->beginEdit(m.id);
    case AP11::Value: {
      if (!p || !p->editing || !componentHandler || !m.revision ||
          !std::isfinite(m.value) || m.value < 0 || m.value > 1)
        return kResultFalse;
      p->revision = std::max(p->revision, m.revision);
      EditController::setParamNormalized(m.id, m.value);
      forwarding_ = true;
      forward_id_ = m.id;
      forward_value_ = m.value;
      auto r = componentHandler->performEdit(m.id, m.value);
      forwarding_ = false;
      return r;
    }
    case AP11::End:
      if (!p || !p->editing || !componentHandler)
        return kResultFalse;
      p->editing = false;
      return componentHandler->endEdit(m.id);
    case AP11::GroupBegin:
      if (group_ || !componentHandler2)
        return kResultFalse;
      group_ = true;
      return componentHandler2->startGroupEdit();
    case AP11::GroupEnd:
      if (!group_ || !componentHandler2)
        return kResultFalse;
      group_ = false;
      return componentHandler2->finishGroupEdit();
    case AP11::Dirty:
      if (!componentHandler2 || (m.value != 0 && m.value != 1))
        return kResultFalse;
      return componentHandler2->setDirty(m.value == 1);
    case AP11::RequestOpen:
      return componentHandler2
                 ? componentHandler2->requestOpenEditor(ViewType::kEditor)
                 : kNotImplemented;
    case AP11::EditorStatus:
      if (m.activation != activation_serial_)
        return kResultOk;
      if (!m.count)
        activation_.cancel();
      if (m.count && m.focus_result == AP11::FocusPending)
        activation_.target(m);
      if (m.result) {
        switch (m.result) {
        case AP11::NoView:
          status_ = "Vendor did not provide an editor";
          break;
        case AP11::Platform:
          status_ = "Vendor editor platform unsupported";
          break;
        case AP11::Size:
          status_ = "Vendor editor size request failed";
          break;
        default:
          status_ = "Vendor editor could not open or close";
          break;
        }
      } else if (!m.count)
        status_ = "Vendor editor closed";
      else if (m.focus_result == AP11::FocusConfirmed)
        status_ = "Focus transferred to vendor editor";
      else if (m.focus_result == AP11::FocusDenied)
        status_ = "Editor open; desktop focus request refused";
      else if (m.focus_result == AP11::FocusUnsupported)
        status_ = "Editor open; click Open to request focus";
      else if (m.focus_result == AP11::FocusCancelled)
        status_ = "Editor open; focus request cancelled";
      else
        status_ = "Vendor editor open; requesting focus...";
      return kResultOk;
    case AP11::RefreshBegin:
      if (refreshing_ || m.count != states_.size() || !m.revision ||
          (m.flags & ~20))
        return kResultFalse;
      refreshing_ = true;
      refresh_revision_ = m.revision;
      refresh_flags_ = m.flags;
      for (auto &s : states_)
        s.refreshed = false;
      return kResultOk;
    case AP11::Parameter: {
      if (!refreshing_ || !p || p->refreshed || m.result>1 || !std::isfinite(m.value) ||
          (m.result==0 ? (m.value < 0 || m.value > 1) : m.value!=0) || m.steps < 0 || m.title[127] ||
          m.units[127])
        return kResultFalse;
      p->refreshed = true;
      auto *parameter = parameters.getParameter(m.id);
      if (!parameter)
        return kResultFalse;
      auto &info = parameter->getInfo();
      std::memcpy(info.title, m.title, sizeof(info.title));
      std::memcpy(info.shortTitle, m.title, sizeof(info.shortTitle));
      info.shortTitle[127] = 0;
      std::memcpy(info.units, m.units, sizeof(info.units));
      info.stepCount = m.steps;
      info.flags = m.flags;
      if (m.revision >= p->revision) {
        p->revision = m.revision;
        p->available=m.result==0;
        if(p->available)EditController::setParamNormalized(m.id, m.value);
      }
      return kResultOk;
    }
    case AP11::RefreshEnd:
      if (!refreshing_ || m.revision != refresh_revision_ ||
          m.flags != refresh_flags_ ||
          std::any_of(states_.begin(), states_.end(),
                      [](const auto &s) { return !s.refreshed; }))
        return kResultFalse;
      refreshing_ = false;
      return !refresh_flags_ ? kResultOk : componentHandler
                 ? componentHandler->restartComponent(refresh_flags_)
                 : kResultFalse;
    default:
      return kNotImplemented;
    }
  }
  void tick() {
    if (!onOwner() || !connected_ || ticking_)
      return;
    ticking_ = true;
    request("AP10.poll");
    if (!generation_)
      request("AP11.bind");
    else
      request("AP11.poll");
    ap11_gui_message_t focus{};
    if (activation_.poll(focus)) {
      focus.kind = AP11::Focus;
      if (!focus.target_x11 && focus.focus_result == AP11::FocusDenied)
        status_ = "Vendor editor open/focus response timed out";
      // A focus denial is a UI result, never a failed audio/session channel.
      command(focus);
    }
    ticking_ = false;
  }
  void capabilities(bool attached = true) {
    if (!connected_)
      return;
    auto *m = allocateMessage();
    if (m) {
      m->setMessageID("AP10.capabilities");
      m->getAttributes()->setInt(
          "notifications", attached && componentHandler && timer_ ? 1 : 0);
      sendMessage(m);
      m->release();
    }
    if (!generation_)
      return;
    m = allocateMessage();
    if (m) {
      m->setMessageID("AP11.capabilities");
      m->getAttributes()->setInt("generation", Steinberg::int64(generation_));
      m->getAttributes()->setInt(
          "caps", attached ? ((componentHandler ? 1 : 0) |
                              (componentHandler2 ? 2 : 0) | (timer_ ? 4 : 0))
                           : 0);
      sendMessage(m);
      m->release();
    }
  }
  Result command(const ap11_gui_message_t &event) {
    if (!connected_)
      return Steinberg::kResultOk; // before connection, component state is
                                   // authoritative
    if (failure_ && event.kind != AP11::Close)
      return Steinberg::kResultFalse;
    if (!generation_) {
      if (event.kind ==
          AP11::Close) { // cancel only pending opens; preserve host values
        size_t n = 0;
        for (size_t i = 0; i < pending_count_; ++i)
          if (pending_[i].kind != AP11::Open &&
              pending_[i].kind != AP11::Refresh)
            pending_[n++] = pending_[i];
        pending_count_ = n;
        return Steinberg::kResultOk;
      }
      if (pending_count_ == pending_.size()) {
        fail(AP11::Backlog);
        return Steinberg::kResultFalse;
      }
      pending_[pending_count_++] = event;
      return Steinberg::kResultOk;
    }
    auto *m = allocateMessage();
    if (!m)
      return Steinberg::kResultFalse;
    m->setMessageID("AP11.command");
    m->getAttributes()->setInt("generation", Steinberg::int64(generation_));
    m->getAttributes()->setBinary("command", &event, sizeof(event));
    auto r = sendMessage(m);
    m->release();
    if (r != Steinberg::kResultOk)
      fail(AP11::Host);
    return r;
  }
  bool apply(const std::vector<uint8_t> &b) {
    if (ap8_validate(identity, b.data(), static_cast<uint32_t>(b.size())))
      return false;
    auto read = [](const uint8_t *p) {
      uint32_t n = 0;
      for (unsigned i = 0; i < 4; ++i)
        n |= uint32_t(p[i]) << (8 * i);
      return n;
    };
    const auto *p = b.data() + 104;
    auto n = read(p + 8);
    if (n != std::size(AP8::parameters))
      return false;
    const auto width=(read(p+12)&2)?16u:12u;
    p += 16 + read(p) + read(p + 4);
    // Verify every identity before mutating even the presentation projection.
    for(uint32_t i=0;i<n;++i)if(!state(read(p+i*width)))return false;
    for (uint32_t i = 0; i < n; ++i, p += width) {
      double value;
      std::memcpy(&value, p + width-8, 8);
      auto id = read(p);
      auto* projection=state(id);projection->available=width==12||read(p+4)==1;
      if(projection->available&&EditController::setParamNormalized(id,value)!=Steinberg::kResultOk)return false;
    }
    return true;
  }
  Result request(const char *id = "AP8.readback") {
    auto *m = allocateMessage();
    if (!m)
      return Steinberg::kResultFalse;
    m->setMessageID(id);
    if (generation_)
      m->getAttributes()->setInt("generation", Steinberg::int64(generation_));
    auto r = sendMessage(m);
    m->release();
    return r;
  }
  void stopTimer() {
    if (timer_) {
      timer_->parent = nullptr;
      if (loop_)
        loop_->unregisterTimer(timer_);
      timer_->release();
      timer_ = nullptr;
    }
    if (loop_) {
      loop_->release();
      loop_ = nullptr;
    }
  }
  Steinberg::Linux::IRunLoop *loop_ = nullptr;
  Timer *timer_ = nullptr;
  std::thread::id owner_;
  bool connected_ = false, ticking_ = false, group_ = false,
       forwarding_ = false, refreshing_ = false;
  uint32_t failure_ = 0, forward_id_ = 0;
  double forward_value_ = 0;
  uint64_t generation_ = 0, refresh_revision_ = 0;
  int32_t refresh_flags_ = 0;
  const char *status_ = "Vendor editor closed";
  std::vector<ParameterState> states_;
  std::array<ap11_gui_message_t, 512> pending_{};
  size_t pending_count_ = 0;
};
} // namespace AP8
