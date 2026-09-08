#pragma once
#include <cstdint>
#include "ap8_events.h"
#include "../../native-vst3-proxy/include/ap10_results.h"
#include "bus_layout.h"
#include "pluginterfaces/vst/ivstprocesscontext.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
namespace linux_vst_bridge::wf0 {
class HostCallbackSink;
class EventWriter;
struct ExternalBlock { Steinberg::Vst::ProcessContext context{};uint64_t gui_revision=0;bool has_context=false; int frames; double gain; unsigned silence; bool gain_present=true;std::array<InputEvent,event_capacity> events{};size_t event_count=0; };
// Private C++ edge interface; no object/layout crosses the process boundary.
class ExternalProcessing {
public:
    virtual ~ExternalProcessing() = default;
    virtual Steinberg::tresult request_restart(int32_t) {return Steinberg::kNotImplemented;}
    virtual Steinberg::tresult editor_edit(uint32_t,uint32_t=0,double=0) {return Steinberg::kNotImplemented;}
    virtual void editor_name(const char*) {}
    virtual const BusLayout* bus_layout() const {return nullptr;}
    virtual bool performance() const {return false;}
    virtual bool returned_results() const {return false;}
    virtual void bind_processor(Steinberg::Vst::IAudioProcessor*) {}
    virtual double sample_rate() const {return 48000.;}
    virtual bool commercial() const {return false;}
    virtual void bind_controller(Steinberg::Vst::IEditController*,bool) {}
    virtual bool hosted() const { return false; }
    virtual bool sustained() const { return false; }
    virtual bool stateful() const { return false; }
    virtual void bind_component(Steinberg::Vst::IComponent*) {}
    virtual void service_owner() {}
    virtual bool initial_transition() {return true;}
    virtual uint32_t process_mode() const {return 0;}
    virtual bool activation_again() {return false;}

    // Owner-thread selection after a stopped interval; the chosen frame stays
    // pending until the corresponding lifecycle_request consumes it.
    virtual uint16_t next_transition() { return 14; }
    virtual uint32_t lifecycle_request(uint16_t) { return 256; }
    virtual void lifecycle_ack(uint16_t) {}
    virtual void before_process() {}
    virtual void after_process() {}
    virtual void ready() = 0;
    virtual bool next(ExternalBlock&, float* left, float* right) = 0;
    virtual void done(const float* left, const float* right, uint64_t silence, uint64_t process_ns = 0, const ap10_results_t* results = nullptr) = 0;
};
struct OfflineResult { bool success; bool quiescent; };
OfflineResult run_offline_processing(Steinberg::Vst::IComponent& component,
    Steinberg::Vst::IAudioProcessor& processor, HostCallbackSink& callbacks, EventWriter& events, ExternalProcessing* external = nullptr);
}
