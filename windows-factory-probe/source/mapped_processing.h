#pragma once
#include "offline_processing.h"
#include "ap1_protocol.h"
#include <memory>
#include <string>
namespace linux_vst_bridge::wf0 {
class MappedSession final : public ExternalProcessing {
public:
    MappedSession(const std::wstring& directory, const std::string& session, EventWriter&, bool hosted=false, bool sustained=false, bool stateful=false, bool commercial=false, bool performance=false);
    ~MappedSession();
    Steinberg::tresult request_restart(int32_t) override;
    bool commercial() const override;
    bool performance() const override;
 const BusLayout* bus_layout() const override;
    void bind_processor(Steinberg::Vst::IAudioProcessor*) override;
    double sample_rate() const override;
    void bind_controller(Steinberg::Vst::IEditController*,bool) override;
    bool hosted() const override;
    bool sustained() const override;
    bool stateful() const override;
    void bind_component(Steinberg::Vst::IComponent*) override;
    void service_owner() override;
    bool initial_transition() override;
    uint32_t process_mode() const override;
    bool activation_again() override;
    uint16_t next_transition() override;
    uint32_t lifecycle_request(uint16_t) override;
    void lifecycle_ack(uint16_t) override;
    void before_process() override;
    void after_process() override;
    void ready() override;
    bool next(ExternalBlock&,float*,float*) override;
    void done(const float*,const float*,uint64_t,uint64_t=0,const ap10_results_t* = nullptr) override;
    void finish(bool success);
private:
    struct Impl;std::unique_ptr<Impl> impl_;
};
}
