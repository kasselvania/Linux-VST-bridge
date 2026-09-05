#pragma once
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
namespace linux_vst_bridge::wf0 {
class HostCallbackSink;
class EventWriter;
struct OfflineResult { bool success; bool quiescent; };
OfflineResult run_offline_processing(Steinberg::Vst::IComponent& component,
    Steinberg::Vst::IAudioProcessor& processor, HostCallbackSink& callbacks, EventWriter& events);
}
