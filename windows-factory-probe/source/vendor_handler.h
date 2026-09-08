#pragma once
#include "offline_processing.h"
#include <atomic>
#include <cstring>
namespace linux_vst_bridge::wf0 {
using namespace Steinberg;
using namespace Steinberg::Vst;
class VendorHandler final : public IComponentHandler,
                            public IComponentHandler2 {
  std::atomic<uint32> refs{1};

public:
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
  uint32 PLUGIN_API release() override { return --refs; }
  tresult PLUGIN_API beginEdit(ParamID id) override {
    return external ? external->editor_edit(101, id) : kNotImplemented;
  }
  tresult PLUGIN_API performEdit(ParamID id, ParamValue value) override {
    return external ? external->editor_edit(102, id, value) : kNotImplemented;
  }
  tresult PLUGIN_API endEdit(ParamID id) override {
    return external ? external->editor_edit(103, id) : kNotImplemented;
  }
  tresult PLUGIN_API setDirty(TBool state) override {
    return external ? external->editor_edit(104, 0, state ? 1. : 0.)
                    : kNotImplemented;
  }
  tresult PLUGIN_API requestOpenEditor(FIDString name) override {
    return external && name && !std::strcmp(name, ViewType::kEditor)
               ? external->editor_edit(107)
               : kNotImplemented;
  }
  tresult PLUGIN_API startGroupEdit() override {
    return external ? external->editor_edit(105) : kNotImplemented;
  }
  tresult PLUGIN_API finishGroupEdit() override {
    return external ? external->editor_edit(106) : kNotImplemented;
  }
  tresult PLUGIN_API restartComponent(int32 flags) override {
    return external ? external->request_restart(flags) : kNotImplemented;
  }
};
} // namespace linux_vst_bridge::wf0
