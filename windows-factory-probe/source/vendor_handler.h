#pragma once
#include "offline_processing.h"
#include <atomic>
#include <cstring>
#include <thread>
namespace linux_vst_bridge::wf0 {
using namespace Steinberg;
using namespace Steinberg::Vst;
struct VendorEditSink {
  virtual tresult scoped_edit(uint64_t, uint32_t, uint32_t, uint32_t, double) = 0;
  virtual tresult scoped_restart(uint64_t, uint32_t, int32_t) = 0;
  virtual ~VendorEditSink() = default;
};
class VendorHandler final : public IComponentHandler,
                            public IComponentHandler2 {
  std::atomic<uint32> refs{1};
  VendorEditSink *sink_ = nullptr;
  const uint64_t native_ = 0;
  const uint32_t epoch_ = 0;
  bool owned_ = false;
  const std::thread::id owner_ = std::this_thread::get_id();
  tresult edit(uint32_t kind, uint32_t id = 0, double value = 0) {
    if (owned_ && owner_ != std::this_thread::get_id()) return kResultFalse;
    if (owned_) return sink_ ? sink_->scoped_edit(native_, epoch_, kind, id, value) : kNotImplemented;
    return external ? external->editor_edit(kind, id, value) : kNotImplemented;
  }

public:
  VendorHandler() = default;
  VendorHandler(VendorEditSink &sink, uint64_t native, uint32_t epoch)
      : sink_(&sink), native_(native), epoch_(epoch), owned_(true) {}
  void retire() { sink_ = nullptr; }
  ExternalProcessing *external = nullptr;
  tresult PLUGIN_API queryInterface(const TUID id, void **out) override {
    if (!out)
      return kInvalidArgument;
    *out = nullptr;
    if (FUnknownPrivate::iidEqual(id, IComponentHandler::iid) ||
        FUnknownPrivate::iidEqual(id, FUnknown::iid)) {
      *out = static_cast<IComponentHandler *>(this);
      addRef();
      return kResultOk;
    }
    if (FUnknownPrivate::iidEqual(id, IComponentHandler2::iid)) {
      *out = static_cast<IComponentHandler2 *>(this);
      addRef();
      return kResultOk;
    }
    return kNoInterface;
  }
  uint32 PLUGIN_API addRef() override { return ++refs; }
  uint32 PLUGIN_API release() override { auto n = --refs; if (!n && owned_) delete this; return n; }
  tresult PLUGIN_API beginEdit(ParamID id) override {
    return edit(101, id);
  }
  tresult PLUGIN_API performEdit(ParamID id, ParamValue value) override {
    return edit(102, id, value);
  }
  tresult PLUGIN_API endEdit(ParamID id) override {
    return edit(103, id);
  }
  tresult PLUGIN_API setDirty(TBool state) override {
    return edit(104, 0, state ? 1. : 0.);
  }
  tresult PLUGIN_API requestOpenEditor(FIDString name) override {
    return name && !std::strcmp(name, ViewType::kEditor)
               ? edit(107)
               : kNotImplemented;
  }
  tresult PLUGIN_API startGroupEdit() override {
    return edit(105);
  }
  tresult PLUGIN_API finishGroupEdit() override {
    return edit(106);
  }
  tresult PLUGIN_API restartComponent(int32 flags) override {
    if (owned_ && owner_ != std::this_thread::get_id()) return kResultFalse;
    if (owned_) return sink_ ? sink_->scoped_restart(native_, epoch_, flags) : kNotImplemented;
    return external ? external->request_restart(flags) : kNotImplemented;
  }
};
} // namespace linux_vst_bridge::wf0
