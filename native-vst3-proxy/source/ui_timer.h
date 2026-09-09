#pragma once
#include "public.sdk/source/common/pluginview.h"
#include <atomic>
#include <thread>
namespace AP15 {
// A registration owns its handler independently of the view and independently
// of whether the host takes a reference. Failed unregister keeps that exact
// registration alive. An abandoned view detaches its callback; only the small
// inert handler/loop receipt remains until the host accepts unregister.
class UiTimer final : public Steinberg::Linux::ITimerHandler {
  std::atomic<Steinberg::uint32> refs_{1};
  Steinberg::IPtr<Steinberg::Linux::IRunLoop> loop_;
  void *context_;
  void (*callback_)(void *);
  const std::thread::id owner_ = std::this_thread::get_id();
  bool registered_ = false, stopping_ = false;
public:
  UiTimer(Steinberg::Linux::IRunLoop *loop, void *context, void (*callback)(void *))
      : loop_(loop), context_(context), callback_(callback) {}
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id, void **out) override {
    if (!out) return Steinberg::kInvalidArgument;
    *out = nullptr;
    if (!Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::FUnknown::iid) &&
        !Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::Linux::ITimerHandler::iid))
      return Steinberg::kNoInterface;
    *out = static_cast<Steinberg::Linux::ITimerHandler *>(this);
    addRef(); return Steinberg::kResultOk;
  }
  Steinberg::uint32 PLUGIN_API addRef() override { return ++refs_; }
  Steinberg::uint32 PLUGIN_API release() override {
    const auto n = --refs_; if (!n) delete this; return n;
  }
  bool start() {
    if (owner_ != std::this_thread::get_id()) return false;
    if (registered_) return true;
    addRef(); // safety registration reference, including hosts with weak handlers
    if (loop_->registerTimer(this, 10) != Steinberg::kResultOk) { release(); return false; }
    registered_ = true;
    return true;
  }
  bool stop() {
    if (owner_ != std::this_thread::get_id()) return false;
    if (!registered_) return true;
    if (stopping_) return false;
    stopping_ = true;
    const auto result = loop_->unregisterTimer(this);
    stopping_ = false;
    if (result != Steinberg::kResultOk) return false;
    registered_ = false;
    release();
    return true;
  }
  void detach() { context_ = nullptr; stop(); }
  void PLUGIN_API onTimer() override {
    if (owner_ != std::this_thread::get_id()) return;
    addRef();
    if (!stopping_) { if (context_) callback_(context_); else stop(); }
    release();
  }
};
}
