#pragma once

#include "linux_vst_bridge/wf0_probe/census.h"
#include "linux_vst_bridge/wf0_probe/events.h"

#include "pluginterfaces/base/ipluginbase.h"

namespace linux_vst_bridge::wf0 {

struct FactoryCensusResult {
    int exit_code{0};
    CensusRecord census;
    Steinberg::IPluginFactory2* factory2{nullptr};
    Steinberg::IPluginFactory3* factory3{nullptr};
    bool base_acquired{false};
};

FactoryCensusResult enumerate_factory(Steinberg::IPluginFactory* factory,
                                      EventWriter& events, int max_classes);
int release_factory_interfaces(Steinberg::IPluginFactory* factory,
                               FactoryCensusResult& result, EventWriter& events,
                               int primary_exit);

} // namespace linux_vst_bridge::wf0
