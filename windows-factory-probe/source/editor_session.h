#pragma once
#include "fault_status.h"
#include "vendor_view.h"
#include <chrono>
#include <vector>
namespace linux_vst_bridge::wf0 {
class EditorSession {
  using Clock = std::chrono::steady_clock;
  using Controller = Steinberg::Vst::IEditController;
  struct Parameter {
    uint32_t id;
    uint64_t accepted = 0;
    bool editing = false;
  };
  GuiChannel &channel_;
  Controller &controller_;
  std::vector<Parameter> parameters_;
  std::thread::id owner_ = std::this_thread::get_id();
  VendorView view_;
  FaultStatus* fault_=nullptr;
  Clock::time_point serviced_{};
  uint64_t floor_ = 0, close_cutoff_ = 0, refresh_revision_ = 0;
  uint32_t refresh_flags_ = 0, pending_refresh_ = 0;
  size_t refresh_cursor_ = 0;
  bool refreshing_ = false, refresh_started_ = false, host_update_ = false,
       group_ = false, was_open_ = false, ever_opened_ = false;
  std::wstring name_;
  ap11_gui_message_t last_open_{};
  uint32_t focus_result_ = AP11::FocusNotRequested;
  bool focus_pending_ = false;
  bool desktop_confirmed_ = false;
  Parameter *parameter(uint32_t id) {
    auto p =
        std::lower_bound(parameters_.begin(), parameters_.end(), id,
                         [](const auto &p, uint32_t v) { return p.id < v; });
    return p != parameters_.end() && p->id == id ? &*p : nullptr;
  }
  bool emit(uint32_t kind, uint32_t id = 0, double value = 0,
            uint64_t revision = 0, int32_t flags = 0) {
    ap11_gui_message_t m{};
    m.kind = kind;
    m.id = id;
    m.value = value;
    m.revision = revision;
    m.flags = flags;
    return channel_.send(m);
  }
  void status() {
    ap11_gui_message_t m{};
    m.kind = AP11::EditorStatus;
    m.count = view_.is_open() ? 1 : 0;
    m.result = view_.error();
    m.activation = last_open_.activation;
    m.user_time = last_open_.user_time;
    m.requestor_x11 = last_open_.requestor_x11;
    m.target_x11 = view_.is_open() ? view_.x11_window() : 0;
    m.view_epoch = uint32_t(view_.opens);
    m.focus_flags = (uint32_t(view_.focus_window) << 1) |
                    (uint32_t(view_.focus_keyboard) << 2);
    m.focus_result = focus_result_;
    channel_.open_state(view_.is_open());
    channel_.send(m);
    was_open_ = view_.is_open();
  }
  void refresh() {
    if (!refreshing_)
      return;
    if (!refresh_started_) {
      if (channel_.available() < 64)
        return;
      ap11_gui_message_t m{};
      m.kind = AP11::RefreshBegin;
      m.count = uint32_t(parameters_.size());
      m.revision = refresh_revision_;
      m.flags = int32_t(refresh_flags_);
      if (!channel_.send(m))
        return;
      refresh_started_ = true;
    }
    for (unsigned n = 0; n < 64 && refresh_cursor_ < parameters_.size() &&
                         channel_.available() > 64;
         ++n, ++refresh_cursor_) {
      Steinberg::Vst::ParameterInfo info{};
      if (controller_.getParameterCount() != int(parameters_.size()) ||
          controller_.getParameterInfo(int(refresh_cursor_), info) !=
              Steinberg::kResultOk ||
          !parameter(info.id)) {
        channel_.fail(AP11::Controller);
        return;
      }
      ap11_gui_message_t m{};
      m.kind = AP11::Parameter;
      m.id = info.id;
      m.revision = std::max(refresh_revision_, parameter(info.id)->accepted);
      m.value = controller_.getParamNormalized(info.id);
      m.flags = info.flags;
      m.steps = info.stepCount;
      std::memcpy(m.title, info.title, sizeof(m.title));
      std::memcpy(m.units, info.units, sizeof(m.units));
      if (!std::isfinite(m.value) || m.value < 0 || m.value > 1) {
        m.result=1; // explicitly unavailable readback; zero bytes carry no value
        m.value=0;
      }
      if (m.title[127] || m.units[127]) {
        channel_.fail(AP11::Controller);
        return;
      }
      if (!channel_.send(m))
        return;
    }
    if (refresh_cursor_ == parameters_.size() && channel_.available() > 64) {
      emit(AP11::RefreshEnd, 0, 0, refresh_revision_, int32_t(refresh_flags_));
      refreshing_ = false;
      if (pending_refresh_) {
        auto flags = pending_refresh_;
        pending_refresh_ = 0;
        request_refresh(flags);
      }
    }
  }

public:
  uint64_t gestures = 0, values = 0, ends = 0, host_updates = 0,
           stale_updates = 0, suppressed_echoes = 0;
  EditorSession(GuiChannel &channel, Controller &controller)
      : channel_(channel), controller_(controller) {
    auto n = controller.getParameterCount();
    ap1::require(n >= 0 && n <= 8192, "GUI parameter bound");
    parameters_.reserve(size_t(n));
    for (int i = 0; i < n; ++i) {
      Steinberg::Vst::ParameterInfo info{};
      ap1::require(controller.getParameterInfo(i, info) == Steinberg::kResultOk,
                   "GUI parameter metadata");
      parameters_.push_back({info.id});
    }
    std::sort(parameters_.begin(), parameters_.end(),
              [](const auto &a, const auto &b) { return a.id < b.id; });
    ap1::require(std::adjacent_find(parameters_.begin(), parameters_.end(),
                                    [](const auto &a, const auto &b) {
                                      return a.id == b.id;
                                    }) == parameters_.end(),
                 "GUI duplicate parameter ID");
    view_.diagnostic(
        [](void *p, uint32_t stage) {
          static_cast<GuiChannel *>(p)->view_stage(stage);
        },
        &channel_);
    view_.fault_diagnostic([](void *p, EXCEPTION_POINTERS *e) {
      static_cast<GuiChannel *>(p)->view_fault(e);
    });
    channel_.ready();
  }
  void fault_status(FaultStatus* f) { fault_=f; }
  void name(const std::wstring &name) { name_ = name; }
  bool host_value(uint32_t id, double value, uint64_t revision) {
    if (owner_ != std::this_thread::get_id())
      return false;
    auto *p = parameter(id);
    if (!p || !std::isfinite(value) || value < 0 || value > 1)
      return false;
    if (revision < std::max(p->accepted, floor_)) {
      ++stale_updates;
      return true;
    }
    host_update_ = true;
    Steinberg::tresult r = Steinberg::kResultFalse;
    try {
      r = controller_.setParamNormalized(id, value);
    } catch (...) {
      host_update_ = false;
      return false;
    }
    host_update_ = false;
    if (r != Steinberg::kResultOk)
      return false;
    p->accepted = revision;
    ++host_updates;
    return true;
  }
  Steinberg::tresult edit(uint32_t kind, uint32_t id, double value) {
    using namespace Steinberg;
    if (owner_ != std::this_thread::get_id()) {
      channel_.fail(AP11::WrongThread);
      return kResultFalse;
    }
    if (host_update_ &&
        (kind == AP11::Begin || kind == AP11::Value || kind == AP11::End)) {
      ++suppressed_echoes;
      return kResultOk;
    }
    // Host-originated updates remain authoritative when the editor channel
    // is unavailable. Suppress their nested gesture echoes before checking
    // GUI capability/failure, so a UI overload cannot poison audio refresh.
    if (channel_.closed() || channel_.failure() ||
        !(channel_.capabilities() & 1))
      return kNotImplemented;
    auto *p = parameter(id);
    if (kind == AP11::Begin) {
      if (!p || p->editing)
        return kResultFalse;
      if (!emit(kind, id))
        return kResultFalse;
      p->editing = true;
      ++gestures;
      return kResultOk;
    }
    if (kind == AP11::Value) {
      if (!p || !p->editing || !std::isfinite(value) || value < 0 || value > 1)
        return kResultFalse;
      auto revision = channel_.revision();
      if (!revision || !emit(kind, id, value, revision))
        return kResultFalse;
      p->accepted = revision;
      ++values;
      return kResultOk;
    }
    if (kind == AP11::End) {
      if (!p || !p->editing)
        return kResultFalse;
      if (!emit(kind, id))
        return kResultFalse;
      p->editing = false;
      ++ends;
      return kResultOk;
    }
    if (!(channel_.capabilities() & 2))
      return kNotImplemented;
    if (kind == AP11::GroupBegin) {
      if (group_ || !emit(kind))
        return kResultFalse;
      group_ = true;
      return kResultOk;
    }
    if (kind == AP11::GroupEnd) {
      if (!group_ || !emit(kind))
        return kResultFalse;
      group_ = false;
      return kResultOk;
    }
    if (kind == AP11::Dirty)
      return emit(kind, 0, value) ? kResultOk : kResultFalse;
    if (kind == AP11::RequestOpen && ever_opened_)
      return emit(kind) ? kResultOk : kResultFalse;
    return kNotImplemented;
  }
  Steinberg::tresult restart(int32_t flags) {
    using namespace Steinberg;
    using namespace Steinberg::Vst;
    if (owner_ != std::this_thread::get_id() || flags <= 0 ||
        (flags & ~(kParamValuesChanged | kParamTitlesChanged)) ||
        !(channel_.capabilities() & 1) || channel_.closed() ||
        channel_.failure())
      return kNotImplemented;
    // An actual vendor invalidation supersedes earlier accepted audio
    // refreshes.
    floor_ = channel_.revision();
    request_refresh(uint32_t(flags));
    return kResultOk;
  }
  void request_refresh(uint32_t flags) {
    // Publish one complete revision before a later refresh. Restarting a
    // partial stream would otherwise mix title/value snapshots at the host.
    if (refreshing_) {
      pending_refresh_ |= flags;
      return;
    }
    refreshing_ = true;
    refresh_started_ = false;
    refresh_cursor_ = 0;
    refresh_flags_ = flags;
    refresh_revision_ = channel_.revision();
  }
  bool close() {
    focus_pending_ = false;
    focus_result_ = AP11::FocusCancelled;
    if (!view_.close())
      return false;
    for (auto &p : parameters_)
      if (p.editing) {
        emit(AP11::End, p.id);
        p.editing = false;
        ++ends;
      }
    if (group_) {
      emit(AP11::GroupEnd);
      group_ = false;
    }
    return true;
  }
  void service(bool force = false) {
    ap1::require(owner_ == std::this_thread::get_id(), "GUI owner thread");
    auto now = Clock::now();
    if (!force && now - serviced_ < std::chrono::milliseconds(4))
      return;
    serviced_ = now;
    FaultStatus::Scope activity(fault_,2,22);
    channel_.heartbeat();
    if (channel_.closed() || channel_.failure()) {
      if (!close())
        channel_.fail(AP11::Removal);
      return;
    }
    auto close_epoch = channel_.close_requested();
    if (close_epoch != channel_.close_acknowledged()) {
      close_cutoff_ = channel_.close_cutoff();
      if (!close()) {
        channel_.fail(AP11::Removal);
        return;
      }
      channel_.close_ack(close_epoch);
      status();
    }
    for (unsigned n = 0; n < 64; ++n) {
      ap11_gui_message_t m{};
      if (!channel_.take(m))
        break;
      if (m.kind == AP11::Open) {
        if (channel_.commands_consumed() <= close_cutoff_) {
          status();
          continue;
        }
        ever_opened_ = true;
        last_open_ = m;
        focus_pending_ = false;
        focus_result_ = AP11::FocusPending;
        ++view_.focus_requests;
        view_.open(controller_);
        if (view_.window() && !name_.empty())
          SetWindowTextW(view_.window(), name_.c_str());
        status();
      } else if (m.kind == AP11::Focus) {
        if (m.activation != last_open_.activation || !view_.is_open() ||
            m.target_x11 != view_.x11_window() || m.view_epoch != view_.opens ||
            focus_result_ != AP11::FocusPending)
          continue;
        if (m.focus_result < AP11::FocusConfirmed ||
            m.focus_result > AP11::FocusCancelled) {
          channel_.fail(AP11::Protocol);
          break;
        }
        desktop_confirmed_ = m.focus_result == AP11::FocusConfirmed;
        focus_result_ = m.focus_result;
        focus_pending_ = true;
      } else if (m.kind == AP11::Set) {
        if (!m.revision || !host_value(m.id, m.value, m.revision)) {
          channel_.fail(AP11::Controller);
          break;
        }
      } else if (m.kind == AP11::Refresh)
        request_refresh(uint32_t(m.flags));
      else {
        channel_.fail(AP11::Protocol);
        break;
      }
    }
    bool pumped=false;
    {FaultStatus::Scope pumping(fault_,2,25);pumped=VendorView::pump();}
    if (!pumped) {
      channel_.fail(AP11::Closed);
      close();
      return;
    }
    if (focus_pending_) {
      focus_pending_ = false;
      if (desktop_confirmed_ && !view_.confirm_focus(true))
        focus_result_ = AP11::FocusDenied;
      // A negative native result remains negative even if Wine cached focus.
      if (!desktop_confirmed_)
        view_.confirm_focus(false);
      status();
    }
    if (view_.close_requested() || was_open_ != view_.is_open()) {
      if (!close())
        channel_.fail(AP11::Removal);
      status();
    }
    try {
      refresh();
    } catch (...) {
      channel_.fail(AP11::Controller);
      close();
    }
  }
  bool is_open() const { return view_.is_open(); }
  const VendorView &view() const { return view_; }
};
} // namespace linux_vst_bridge::wf0
