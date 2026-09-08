#pragma once
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <atomic>
#include <cstdint>
#include <thread>
#ifdef AP8_PREVIEW
#include "ap8_descriptor.h"
#include "output_results.h"
#include <vector>
#include <array>
#endif
namespace AP2 {
inline constexpr Steinberg::Vst::ParamID recoveryID = 0x41503601;
inline constexpr Steinberg::Vst::ParamID snapshotID = 0x41503602;
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
    return preview_ ? latency_ : queued_ ? 1024 : 0;
  }
  Steinberg::uint32 PLUGIN_API getTailSamples() override { return tail_; }
  Steinberg::tresult PLUGIN_API
  getControllerClassId(Steinberg::TUID id) override {
    if (preview_) {
#ifdef AP8_PREVIEW
      Steinberg::FUID(AP8_CONTROLLER_UID).toTUID(id);
#else
      controllerID.toTUID(id);
#endif
      return Steinberg::kResultOk;
    }
    return Steinberg::kNotImplemented;
  }
  Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream *) override;
  Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream *) override;
  Steinberg::tresult PLUGIN_API notify(Steinberg::Vst::IMessage *) override;

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
#ifdef AP8_PREVIEW
  AP10Results::Output returned_;
  bool deliverResults(Steinberg::Vst::ProcessData&);
  int eventOutputActive(int)const;
  bool notifications_=false;uint32_t vendor_latency_=0;
  std::array<bool,32> bus_active_{};
  bool setupBuses(uint32_t maximum,uint32_t mode,double rate,uint32_t* traits);
  // Written under busy_ by the callback, read only after quiescence at terminate.
  struct AdmissionFailure {
    uint32_t code=0, context_state=0;
    int32_t frames=0;
    uint64_t input_flags=0;
    double rate=0, cycle_start=0, cycle_end=0;
  } admission_failure_;
  std::vector<uint8_t> state_readback_;
  Steinberg::tresult readback();
#endif
  void snapshotStatus(const char *status);
  Steinberg::tresult recover(uint64_t revision);
  bool controller_synced_ = false;
  std::atomic<bool> want_active_{false}, want_processing_{false};
  Steinberg::tresult rejected(Steinberg::Vst::ProcessData &data);
  std::atomic<uint64_t> rejected_frames_{0}, discontinuities_{0};
  std::atomic<bool> last_callback_rejected_{false};
  void stateFailure(const char *operation, const char *stage);
  bool state_error_reported_ = false;
  std::atomic_flag busy_ = ATOMIC_FLAG_INIT;
  uint64_t handle_ = 0;
  char report_path_[4096]{};
  int maximum_ = 0;
  uint32_t latency_ = 1024, tail_ = 0;
  unsigned blocks_ = 0;
  std::atomic<uint64_t> callback_rejections_{0};
  uint64_t frames_ = 0, zero_gain_blocks_ = 0;
  uint64_t silent_callbacks_ = 0, silent_frames_ = 0;
  uint64_t priming_frames_ = 0, underrun_frames_ = 0, underrun_gaps_ = 0;
  uint64_t expired_frames_ = 0, delivered_frames_ = 0, underrun_callbacks_ = 0;
  uint64_t input_hint_adjustments_ = 0, input_hint_samples_ = 0;
  uint32_t input_hint_first_bits_ = 0;
  double input_hint_peak_ = 0.;
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
