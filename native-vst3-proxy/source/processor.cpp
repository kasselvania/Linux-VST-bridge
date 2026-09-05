#include "processor.h"
#include "../../vst-state/stream.h"
#include "ap2_backend.h"
#include "ap3_backend.h"
#include "ap4_backend.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <limits>
#include <sys/stat.h>
#include <unistd.h>
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
// Optional bounded test report, written only during non-RT termination. The
// DAW's own plug-in host may redirect stdout; this preserves the same facts.
void diagnostic_report(const char *text, size_t size) {
  std::fwrite(text, 1, size, stdout);
  const char *path = std::getenv("LVB_AP3_REPORT");
  if (!path)
    return;
  int fd = ::open(path, O_WRONLY | O_CREAT | O_APPEND | O_NOFOLLOW | O_CLOEXEC,
                  0600);
  struct stat st{};
  bool valid = fd >= 0 && !::fstat(fd, &st) && S_ISREG(st.st_mode) &&
               st.st_uid == ::getuid() && (st.st_mode & 077) == 0 &&
               st.st_size >= 0 && st.st_size + static_cast<off_t>(size) <= 8192;
  if (!valid || ::write(fd, text, size) != static_cast<ssize_t>(size))
    std::fputs("AP3 diagnostic persistence failed\n", stderr);
  if (fd >= 0)
    ::close(fd);
}
void report_stats(uint64_t handle) {
  ap3_stats_t s{};
  if (!ap3_stats(handle, &s)) {
    char text[512];
    auto n = std::snprintf(
        text, sizeof(text),
        "{\"event\":\"ap3_proxy_stats\",\"fault\":%llu,\"first_position\":%llu,"
        "\"processed\":%llu,\"request_high\":%llu,\"result_high\":%llu,"
        "\"position\":%llu,\"epoch\":%llu}\n",
        (unsigned long long)s.fault, (unsigned long long)s.first_position,
        (unsigned long long)s.processed, (unsigned long long)s.request_high,
        (unsigned long long)s.result_high, (unsigned long long)s.position,
        (unsigned long long)s.epoch);
    if (n > 0 && static_cast<size_t>(n) < sizeof(text))
      diagnostic_report(text, static_cast<size_t>(n));
  }
}
bool parameters(IParameterChanges *p, double &gain, bool &changed) {
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
      changed = true;
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
bool Processor::stateSession() {
  if (!preview_)
    return false;
  if (handle_)
    return true;
  queued_ = true;
  if (ap4_open(&handle_)) {
    phase_ = Failed;
    report();
    return false;
  }
  return true;
}
namespace {
void stateReport(const char *operation, const std::vector<uint8_t> &blob,
                 double gain) {
  char hex[65]{};
  for (size_t i = 0; i < 32; ++i)
    std::snprintf(hex + i * 2, 3, "%02x", blob[72 + i]);
  char text[256];
  auto n =
      std::snprintf(text, sizeof(text),
                    "{\"event\":\"ap4_native_state\",\"operation\":\"%s\","
                    "\"payload_bytes\":%zu,\"sha256\":\"%s\",\"gain\":%.9g}\n",
                    operation, blob.size() - LVBState::overhead, hex, gain);
  if (n > 0 && static_cast<size_t>(n) < sizeof(text))
    diagnostic_report(text, static_cast<size_t>(n));
}
} // namespace
tresult PLUGIN_API Processor::getState(IBStream *stream) {
  if (!preview_)
    return kNotImplemented;
  if (!stream || owner_ != std::this_thread::get_id() || phase_ == New ||
      phase_ == Failed || phase_ == Terminated)
    return kResultFalse;
  try {
    if (!stateSession())
      return kResultFalse;
    std::vector<uint8_t> blob(LVBState::payloadLimit + LVBState::overhead);
    uint32_t size = 0;
    if (ap4_state(handle_, nullptr, 0, blob.data(),
                  static_cast<uint32_t>(blob.size()), &size)) {
      phase_ = Failed;
      report();
      return kResultFalse;
    }
    blob.resize(size);
    double gain = 0.;
    if (ap4_validate(blob.data(), size, &gain) ||
        !LVBState::transfer(stream, blob.data(), blob.size(), true))
      return kResultFalse;
    stateReport("get", blob, gain);
    return kResultOk;
  } catch (...) {
    return kResultFalse;
  }
}
tresult PLUGIN_API Processor::setState(IBStream *stream) {
  if (!preview_)
    return kNotImplemented;
  if (!stream || owner_ != std::this_thread::get_id() || phase_ == New ||
      phase_ == Running || phase_ == Failed || phase_ == Terminated)
    return kResultFalse;
  try {
    std::vector<uint8_t> blob;
    double restored = 0.;
    if (!LVBState::readEnvelope(stream, blob) ||
        ap4_validate(blob.data(), static_cast<uint32_t>(blob.size()),
                     &restored))
      return kResultFalse;
    if (!stateSession())
      return kResultFalse;
    std::vector<uint8_t> readback(LVBState::payloadLimit + LVBState::overhead);
    uint32_t size = 0;
    if (ap4_state(handle_, blob.data(), static_cast<uint32_t>(blob.size()),
                  readback.data(), static_cast<uint32_t>(readback.size()),
                  &size)) {
      phase_ = Failed;
      report();
      return kResultFalse;
    }
    readback.resize(size);
    if (readback != blob) {
      phase_ = Failed;
      return kResultFalse;
    }
    gain_ = restored;
    stateReport("set", readback, restored);
    return kResultOk;
  } catch (...) {
    phase_ = Failed;
    return kResultFalse;
  }
}
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
      (phase_ != Initialized && phase_ != Setup && phase_ != Deactivated) ||
      media != kAudio || index != 0 ||
      (direction != kInput && direction != kOutput))
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
      (phase_ != Initialized && phase_ != Setup && phase_ != Deactivated) ||
      ni != 1 || no != 1 || !in || !out || *in != SpeakerArr::kStereo ||
      *out != SpeakerArr::kStereo)
    return kResultFalse;
  return AudioEffect::setBusArrangements(in, ni, out, no);
}
tresult PLUGIN_API Processor::canProcessSampleSize(int32 size) {
  return size == kSample32 ? kResultTrue : kResultFalse;
}
tresult PLUGIN_API Processor::setupProcessing(ProcessSetup &setup) {
  Guard g(busy_);
  requested_maximum_ = setup.maxSamplesPerBlock;
  requested_rate_ = setup.sampleRate;
  requested_mode_ = setup.processMode;
  if (!g.held || owner_ != std::this_thread::get_id() ||
      (phase_ != Initialized && phase_ != Setup && phase_ != Deactivated) ||
      (setup.processMode != kOffline &&
       !(preview_ && setup.processMode == kRealtime)) ||
      setup.symbolicSampleSize != kSample32 || setup.sampleRate != 48000. ||
      setup.maxSamplesPerBlock < 1 || setup.maxSamplesPerBlock > 256)
    return kResultFalse;
  auto r = AudioEffect::setupProcessing(setup);
  if (r == kResultOk) {
    maximum_ = setup.maxSamplesPerBlock;
    process_mode_ = setup.processMode;
    queued_ = preview_ || process_mode_ == kRealtime;
    phase_ = Setup;
  }
  return r;
}
tresult PLUGIN_API Processor::setActive(TBool active) {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id())
    return kResultFalse;
  if (active) {
    if ((phase_ != Setup && phase_ != Deactivated) || !input_active_ ||
        !output_active_)
      return kResultFalse;
    if (preview_
            ? (!stateSession() ||
               ap4_activate(handle_, static_cast<uint32_t>(maximum_),
                            static_cast<uint32_t>(process_mode_)))
            : (queued_ ? ap3_open(static_cast<uint32_t>(maximum_), &handle_)
                       : ap2_open(static_cast<uint32_t>(maximum_), &handle_))) {
      report();
      phase_ = Failed;
      return kResultFalse;
    }
    phase_ = Active;
    return kResultOk;
  }
  if (phase_ != Stopped && !(preview_ && phase_ == Active))
    return kResultFalse;
  if (preview_ ? ap4_deactivate(handle_)
               : (queued_ ? ap3_transition(handle_, 14)
                          : ap2_transition(handle_, 14))) {
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
  auto reject = [&] {
    callback_rejections_.fetch_add(1, std::memory_order_relaxed);
    return failure(d, maximum_);
  };
  if (!g.held) {
    callback_rejections_.fetch_add(1, std::memory_order_relaxed);
    return kResultFalse;
  }
  if (phase_ != Running || d.processMode != process_mode_ ||
      d.symbolicSampleSize != kSample32 || d.numSamples < 0 ||
      d.numSamples > maximum_)
    return reject();
  double pending = gain_;
  bool changed = false;
  if (!parameters(d.inputParameterChanges, pending, changed) ||
      (d.inputEvents && d.inputEvents->getEventCount() != 0))
    return reject();
  if (d.numSamples == 0) {
    if (d.numInputs != 0 || d.numOutputs != 0 || d.inputs || d.outputs)
      return reject();
    if (preview_ && changed) {
      float dummy = 0.f;
      uint64_t flags = 0;
      if (ap3_process(handle_, 0, pending, 0, &dummy, &dummy, &dummy, &dummy,
                      &flags)) {
        phase_ = Failed;
        return reject();
      }
    }
    gain_ = pending;
    return kResultOk;
  }
  if ((!queued_ && blocks_ >= 64) || !outputs(d, maximum_) ||
      d.numInputs != 1 || !d.inputs || d.inputs[0].numChannels != 2 ||
      !d.inputs[0].channelBuffers32 || !d.inputs[0].channelBuffers32[0] ||
      !d.inputs[0].channelBuffers32[1] ||
      (d.inputs[0].silenceFlags & ~uint64(3)))
    return reject();
  auto **in = d.inputs[0].channelBuffers32;
  auto **out = d.outputs[0].channelBuffers32;
  if (overlap(out[0], out[1], d.numSamples) ||
      overlap(in[0], in[1], d.numSamples) ||
      overlap(out[0], in[1], d.numSamples) ||
      overlap(out[1], in[0], d.numSamples) ||
      (in[0] != out[0] && overlap(in[0], out[0], d.numSamples)) ||
      (in[1] != out[1] && overlap(in[1], out[1], d.numSamples)))
    return reject();
  for (int ch = 0; ch < 2; ++ch)
    for (int i = 0; i < d.numSamples; ++i)
      if (!std::isfinite(in[ch][i]) ||
          ((d.inputs[0].silenceFlags & (uint64(1) << ch)) && in[ch][i] != 0.f))
        return reject();
  uint64_t silence = 0;
  auto r =
      queued_
          ? static_cast<int32_t>(ap3_process(
                handle_, static_cast<uint32_t>(d.numSamples),
                preview_ && !changed ? std::numeric_limits<double>::quiet_NaN()
                                     : pending,
                d.inputs[0].silenceFlags, in[0], in[1], out[0], out[1],
                &silence))
          : ap2_process(handle_, static_cast<uint32_t>(d.numSamples), pending,
                        d.inputs[0].silenceFlags, in[0], in[1], out[0], out[1],
                        &silence);
  if (r) {
    if (!queued_)
      report();
    phase_ = Failed;
    return reject();
  }
  gain_ = pending;
  frames_ += d.numSamples;
  zero_gain_blocks_ += pending == 0.;
  gain_min_ = std::min(gain_min_, pending);
  gain_max_ = std::max(gain_max_, pending);
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
  if (preview_ && std::getenv("LVB_AP3_REPORT")) {
    char text[768];
    auto n = std::snprintf(
        text, sizeof(text),
        "{\"event\":\"ap3_proxy_lifecycle\",\"phase\":%d,\"requested_maximum\":"
        "%d,"
        "\"requested_rate\":%.0f,\"requested_mode\":%d,\"frames\":%llu,"
        "\"blocks\":%u,\"callback_rejections\":%llu,\"zero_gain_blocks\":%llu,"
        "\"gain_min\":%.9g,\"gain_"
        "max\":%.9g,"
        "\"clean\":%s}\n",
        static_cast<int>(phase_.load()), requested_maximum_, requested_rate_,
        requested_mode_, (unsigned long long)frames_, blocks_,
        (unsigned long long)callback_rejections_.load(
            std::memory_order_relaxed),
        (unsigned long long)zero_gain_blocks_, gain_min_, gain_max_,
        clean ? "true" : "false");
    if (n > 0 && static_cast<size_t>(n) < sizeof(text))
      diagnostic_report(text, static_cast<size_t>(n));
  }
  phase_ = Terminated;
  return clean ? r : kResultFalse;
}
} // namespace AP2
