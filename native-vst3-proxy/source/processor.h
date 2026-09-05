#pragma once
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <atomic>
#include <cstdint>
#include <thread>
namespace AP2 {
inline const Steinberg::FUID controllerID(0xD1444DE3, 0x38814391, 0xA916DC9C,
                                          0xFCC67008);
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
  Steinberg::uint32 PLUGIN_API getLatencySamples() override {
    return preview_ || queued_ ? 1024 : 0;
  }
  Steinberg::uint32 PLUGIN_API getTailSamples() override { return 0; }
  Steinberg::tresult PLUGIN_API
  getControllerClassId(Steinberg::TUID id) override {
    if (preview_) {
      controllerID.toTUID(id);
      return Steinberg::kResultOk;
    }
    return Steinberg::kNotImplemented;
  }
  Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream *) override;
  Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream *) override;

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
  std::atomic<Phase> phase_{New};
  bool stateSession();
  std::atomic_flag busy_ = ATOMIC_FLAG_INIT;
  uint64_t handle_ = 0;
  int maximum_ = 0;
  unsigned blocks_ = 0;
  std::atomic<uint64_t> callback_rejections_{0};
  uint64_t frames_ = 0, zero_gain_blocks_ = 0;
  double gain_min_ = 1., gain_max_ = 0.;
  int requested_maximum_ = 0, requested_mode_ = -1;
  double requested_rate_ = 0.;
  double gain_ = 1.;
  int process_mode_ = Steinberg::Vst::kOffline;
  bool queued_ = false;
#ifdef AP3_PREVIEW
  const bool preview_ = true;
#else
  const bool preview_ = false;
#endif
  bool input_active_ = true, output_active_ = true;
  std::thread::id owner_;
};
} // namespace AP2
