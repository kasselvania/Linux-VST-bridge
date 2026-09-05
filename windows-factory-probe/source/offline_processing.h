#pragma once
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
namespace linux_vst_bridge::wf0 {
class HostCallbackSink;
class EventWriter;
struct ExternalBlock { int frames; double gain; unsigned silence; };
// Private C++ edge interface; no object/layout crosses the process boundary.
class ExternalProcessing {
public:
    virtual ~ExternalProcessing() = default;
    virtual void ready() = 0;
    virtual bool next(ExternalBlock&, float* left, float* right) = 0;
    virtual void done(const float* left, const float* right, unsigned silence) = 0;
};
struct OfflineResult { bool success; bool quiescent; };
OfflineResult run_offline_processing(Steinberg::Vst::IComponent& component,
    Steinberg::Vst::IAudioProcessor& processor, HostCallbackSink& callbacks, EventWriter& events, ExternalProcessing* external = nullptr);
}
