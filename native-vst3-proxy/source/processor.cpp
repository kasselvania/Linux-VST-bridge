#include "processor.h"
#include "ap2_backend.h"
#include "ap3_backend.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
namespace AP2 {
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
struct Guard {
  std::atomic_flag &flag;
  bool held;
  explicit Guard(std::atomic_flag &f) : flag(f), held(!f.test_and_set()) {}
  ~Guard() {
    if (held)
      flag.clear();
  }
};
void report() {
  uint8_t detail[385]{};
  ap2_error(detail, sizeof(detail));
  std::fprintf(stderr, "AP2 backend: %s\n", reinterpret_cast<char *>(detail));
}
void report_stats(uint64_t handle) {
  ap3_stats_t s{};
  if (!ap3_stats(handle, &s))
    std::fprintf(
        stdout,
        "{\"event\":\"ap3_proxy_stats\",\"fault\":%llu,\"first_position\":%llu,"
        "\"processed\":%llu,\"request_high\":%llu,\"result_high\":%llu,"
        "\"position\":%llu,\"epoch\":%llu}\n",
        (unsigned long long)s.fault, (unsigned long long)s.first_position,
        (unsigned long long)s.processed, (unsigned long long)s.request_high,
        (unsigned long long)s.result_high, (unsigned long long)s.position,
        (unsigned long long)s.epoch);
}
bool parameters(IParameterChanges *p, double &gain) {
  if (!p)
    return true;
  int32 count = p->getParameterCount();
  if (count < 0 || count > 2)
    return false;
  bool gain_seen = false, bypass_seen = false;
  for (int32 i = 0; i < count; ++i) {
    auto *q = p->getParameterData(i);
    if (!q)
      return false;
    int32 n = q->getPointCount();
    if (n < 0 || n > 1)
      return false;
    if (!n)
      continue;
    auto id = q->getParameterId();
    int32 offset = -1;
    ParamValue value = 0.;
    if (q->getPoint(0, offset, value) != kResultOk || offset != 0 ||
        !std::isfinite(value) || value < 0. || value > 1.)
      return false;
    if (id == 0 && !gain_seen) {
      gain = value;
      gain_seen = true;
    } else if (id == 2 && !bypass_seen && value == 0.)
      bypass_seen = true;
    else
      return false;
  }
  return true;
}
bool outputs(ProcessData &d, int maximum) {
  return d.symbolicSampleSize == kSample32 && d.numSamples > 0 &&
         d.numSamples <= maximum && d.numOutputs == 1 && d.outputs &&
         d.outputs[0].numChannels == 2 && d.outputs[0].channelBuffers32 &&
         d.outputs[0].channelBuffers32[0] && d.outputs[0].channelBuffers32[1];
}
tresult failure(ProcessData &d, int maximum) {
  if (outputs(d, maximum)) {
    for (int ch = 0; ch < 2; ++ch)
      std::fill_n(d.outputs[0].channelBuffers32[ch], d.numSamples, 0.f);
    d.outputs[0].silenceFlags = 3;
  }
  return kResultFalse;
}
bool overlap(const float *a, const float *b, int n) {
  auto x = reinterpret_cast<uintptr_t>(a), y = reinterpret_cast<uintptr_t>(b);
  auto bytes = static_cast<uintptr_t>(n) * sizeof(float);
  return x < y + bytes && y < x + bytes;
}
} // namespace
Processor::~Processor() {
  if (handle_)
    queued_ ? (void)ap3_close(handle_) : (void)ap2_close(handle_);
}
tresult PLUGIN_API Processor::initialize(FUnknown *context) {
  Guard g(busy_);
  if (!g.held || phase_ != New || ap2_abi_version() != 1)
    return kResultFalse;
  auto result = AudioEffect::initialize(context);
  if (result != kResultOk)
    return result;
  addAudioInput(STR16("Offline Stereo In"), SpeakerArr::kStereo);
  addAudioOutput(STR16("Offline Stereo Out"), SpeakerArr::kStereo);
  owner_ = std::this_thread::get_id();
  phase_ = Initialized;
  return kResultOk;
}
tresult PLUGIN_API Processor::activateBus(MediaType media,
                                          BusDirection direction, int32 index,
                                          TBool active) {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id() ||
      (phase_ != Initialized && phase_ != Setup) || media != kAudio ||
      index != 0 || (direction != kInput && direction != kOutput))
    return kResultFalse;
  auto r = AudioEffect::activateBus(media, direction, index, active);
  if (r == kResultOk)
    (direction == kInput ? input_active_ : output_active_) = active != 0;
  return r;
}
tresult PLUGIN_API Processor::setBusArrangements(SpeakerArrangement *in,
                                                 int32 ni,
                                                 SpeakerArrangement *out,
                                                 int32 no) {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id() ||
      (phase_ != Initialized && phase_ != Setup) || ni != 1 || no != 1 || !in ||
      !out || *in != SpeakerArr::kStereo || *out != SpeakerArr::kStereo)
    return kResultFalse;
  return AudioEffect::setBusArrangements(in, ni, out, no);
}
tresult PLUGIN_API Processor::canProcessSampleSize(int32 size) {
  return size == kSample32 ? kResultTrue : kResultFalse;
}
tresult PLUGIN_API Processor::setupProcessing(ProcessSetup &setup) {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id() ||
      (phase_ != Initialized && phase_ != Setup) ||
      (setup.processMode != kOffline &&
       !(preview_ && setup.processMode == kRealtime)) ||
      setup.symbolicSampleSize != kSample32 || setup.sampleRate != 48000. ||
      setup.maxSamplesPerBlock < 1 || setup.maxSamplesPerBlock > 256)
    return kResultFalse;
  auto r = AudioEffect::setupProcessing(setup);
  if (r == kResultOk) {
    maximum_ = setup.maxSamplesPerBlock;
    process_mode_ = setup.processMode;
    queued_ = process_mode_ == kRealtime;
    phase_ = Setup;
  }
  return r;
}
tresult PLUGIN_API Processor::setActive(TBool active) {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id())
    return kResultFalse;
  if (active) {
    if (phase_ != Setup || !input_active_ || !output_active_)
      return kResultFalse;
    if (queued_ ? ap3_open(static_cast<uint32_t>(maximum_), &handle_)
                : ap2_open(static_cast<uint32_t>(maximum_), &handle_)) {
      report();
      phase_ = Failed;
      return kResultFalse;
    }
    phase_ = Active;
    return kResultOk;
  }
  if (phase_ != Stopped)
    return kResultFalse;
  if (queued_ ? ap3_transition(handle_, 14) : ap2_transition(handle_, 14)) {
    report();
    phase_ = Failed;
    return kResultFalse;
  }
  phase_ = Deactivated;
  return kResultOk;
}
tresult PLUGIN_API Processor::setProcessing(TBool running) {
  Guard g(busy_);
  if (!g.held)
    return kResultFalse;
  if (running ? (phase_ != Active && !(queued_ && phase_ == Stopped))
              : phase_ != Running)
    return kResultFalse;
  if (queued_ ? ap3_transition(handle_, running ? 10 : 12)
              : ap2_transition(handle_, running ? 10 : 12)) {
    if (!queued_)
      report();
    phase_ = Failed;
    return kResultFalse;
  }
  phase_ = running ? Running : Stopped;
  return kResultOk;
}
tresult PLUGIN_API Processor::process(ProcessData &d) {
  Guard g(busy_);
  if (!g.held)
    return kResultFalse;
  if (phase_ != Running || d.processMode != process_mode_ ||
      d.symbolicSampleSize != kSample32 || d.numSamples < 0 ||
      d.numSamples > maximum_)
    return failure(d, maximum_);
  double pending = gain_;
  if (!parameters(d.inputParameterChanges, pending) ||
      (d.inputEvents && d.inputEvents->getEventCount() != 0))
    return failure(d, maximum_);
  if (d.numSamples == 0) {
    if (d.numInputs != 0 || d.numOutputs != 0 || d.inputs || d.outputs)
      return kResultFalse;
    gain_ = pending;
    return kResultOk;
  }
  if ((!queued_ && blocks_ >= 64) || !outputs(d, maximum_) ||
      d.numInputs != 1 || !d.inputs || d.inputs[0].numChannels != 2 ||
      !d.inputs[0].channelBuffers32 || !d.inputs[0].channelBuffers32[0] ||
      !d.inputs[0].channelBuffers32[1] ||
      (d.inputs[0].silenceFlags & ~uint64(3)))
    return failure(d, maximum_);
  auto **in = d.inputs[0].channelBuffers32;
  auto **out = d.outputs[0].channelBuffers32;
  if (overlap(out[0], out[1], d.numSamples) ||
      overlap(in[0], in[1], d.numSamples) ||
      overlap(out[0], in[1], d.numSamples) ||
      overlap(out[1], in[0], d.numSamples) ||
      (in[0] != out[0] && overlap(in[0], out[0], d.numSamples)) ||
      (in[1] != out[1] && overlap(in[1], out[1], d.numSamples)))
    return failure(d, maximum_);
  for (int ch = 0; ch < 2; ++ch)
    for (int i = 0; i < d.numSamples; ++i)
      if (!std::isfinite(in[ch][i]) ||
          ((d.inputs[0].silenceFlags & (uint64(1) << ch)) && in[ch][i] != 0.f))
        return failure(d, maximum_);
  uint64_t silence = 0;
  auto r = queued_ ? static_cast<int32_t>(ap3_process(
                         handle_, static_cast<uint32_t>(d.numSamples), pending,
                         d.inputs[0].silenceFlags, in[0], in[1], out[0], out[1],
                         &silence))
                   : ap2_process(handle_, static_cast<uint32_t>(d.numSamples),
                                 pending, d.inputs[0].silenceFlags, in[0],
                                 in[1], out[0], out[1], &silence);
  if (r) {
    if (!queued_)
      report();
    phase_ = Failed;
    return failure(d, maximum_);
  }
  gain_ = pending;
  ++blocks_;
  d.outputs[0].silenceFlags = silence;
  return kResultOk;
}
tresult PLUGIN_API Processor::terminate() {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id() || phase_ == New ||
      phase_ == Terminated)
    return kResultFalse;
  bool clean =
      phase_ == Initialized || phase_ == Setup || phase_ == Deactivated;
  if (handle_) {
    if (queued_)
      report_stats(handle_);
    clean =
        (queued_ ? ap3_close(handle_) == 0 : ap2_close(handle_) == 0) && clean;
    if (!clean)
      report();
    handle_ = 0;
  }
  auto r = AudioEffect::terminate();
  phase_ = Terminated;
  return clean ? r : kResultFalse;
}
} // namespace AP2
