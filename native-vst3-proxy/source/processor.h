#pragma once
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <atomic>
#include <cstdint>
#include <thread>
namespace AP2 {
class Processor final : public Steinberg::Vst::AudioEffect {
public:
  static Steinberg::FUnknown *create(void *) {
    try {
      return static_cast<Steinberg::Vst::IAudioProcessor *>(new Processor);
    } catch (...) {
      return nullptr;
    }
  }
  ~Processor() override;
  Steinberg::tresult PLUGIN_API initialize(Steinberg::FUnknown *) override;
  Steinberg::tresult PLUGIN_API terminate() override;
  Steinberg::tresult PLUGIN_API setActive(Steinberg::TBool) override;
  Steinberg::tresult PLUGIN_API activateBus(Steinberg::Vst::MediaType,
                                            Steinberg::Vst::BusDirection,
                                            Steinberg::int32,
                                            Steinberg::TBool) override;
  Steinberg::tresult PLUGIN_API
  setupProcessing(Steinberg::Vst::ProcessSetup &) override;
  Steinberg::tresult PLUGIN_API setProcessing(Steinberg::TBool) override;
  Steinberg::tresult PLUGIN_API setBusArrangements(
      Steinberg::Vst::SpeakerArrangement *, Steinberg::int32,
      Steinberg::Vst::SpeakerArrangement *, Steinberg::int32) override;
  Steinberg::tresult PLUGIN_API canProcessSampleSize(Steinberg::int32) override;
  Steinberg::tresult PLUGIN_API process(Steinberg::Vst::ProcessData &) override;
  Steinberg::uint32 PLUGIN_API getLatencySamples() override { return 0; }
  Steinberg::uint32 PLUGIN_API getTailSamples() override { return 0; }
  Steinberg::tresult PLUGIN_API getControllerClassId(Steinberg::TUID) override {
    return Steinberg::kNotImplemented;
  }
  Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream *) override {
    return Steinberg::kNotImplemented;
  }
  Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream *) override {
    return Steinberg::kNotImplemented;
  }

private:
  enum Phase {
    New,
    Initialized,
    Setup,
    Active,
    Running,
    Stopped,
    Deactivated,
    Failed,
    Terminated
  };
  Phase phase_ = New;
  std::atomic_flag busy_ = ATOMIC_FLAG_INIT;
  uint64_t handle_ = 0;
  int maximum_ = 0;
  unsigned blocks_ = 0;
  double gain_ = 1.;
  bool input_active_ = true, output_active_ = true;
  std::thread::id owner_;
};
} // namespace AP2
