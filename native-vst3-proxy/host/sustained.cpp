// AP3 independent SDK consumer. Only standard VST3 interfaces process audio.
#include "../../vst-state/stream.h"
#include "pluginterfaces/base/funknown.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/module.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <bit>
#include <chrono>
#include <cmath>
#include <dlfcn.h>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <thread>
#include <vector>
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
const char *stage = "load";
uint64_t last_position = 0;
void need(bool v, const char *m) {
  if (!v)
    throw std::runtime_error(m);
}
void ok(tresult v, const char *m) { need(v == kResultOk, m); }
using Clock = std::chrono::steady_clock;
using Mark = void (*)();
using Count = uint64_t (*)();
Mark audit_begin = nullptr;
Count audit_end = nullptr;
template <class F> tresult callback(F f) {
  audit_begin();
  auto result = f();
  auto effects = audit_end();
  need(effects == 0, "callback allocation/blocking/IO/log effect");
  return result;
}
struct Block {
  std::array<std::array<float, 258>, 2> input{}, output{};
  std::array<float *, 2> in{}, out{};
  AudioBusBuffers ib{}, ob{};
  ProcessData data;
  ParameterChanges parameters{1};
  Block() {
    for (int ch = 0; ch < 2; ++ch)
      in[ch] = input[ch].data() + 1;
    ib.numChannels = ob.numChannels = 2;
    ib.channelBuffers32 = in.data();
    ob.channelBuffers32 = out.data();
    data.processMode = kRealtime;
    data.symbolicSampleSize = kSample32;
    data.numInputs = data.numOutputs = 1;
    data.inputs = &ib;
    data.outputs = &ob;
    int32 index = 0, point = 0;
    parameters.addParameterData(0, index)->addPoint(0, 1., point);
  }
  void prepare(int n, double gain, bool update, bool inplace, uint64_t flags) {
    for (int ch = 0; ch < 2; ++ch) {
      input[ch].fill(0.f);
      output[ch].fill(std::bit_cast<float>(uint32_t(0x7fc12345)));
      input[ch][0] = input[ch][n + 1] = output[ch][0] = output[ch][n + 1] =
          std::bit_cast<float>(uint32_t(0x4b123456));
      out[ch] = (inplace ? input[ch] : output[ch]).data() + 1;
    }
    data.numSamples = n;
    ib.silenceFlags = flags;
    ob.silenceFlags = 0;
    parameters.clearQueue();
    if (update) {
      int32 index = 0, point = 0;
      parameters.addParameterData(0, index)->addPoint(0, gain, point);
    }
    data.inputParameterChanges = update ? &parameters : nullptr;
  }
};
void hash(uint64_t &h, float x) {
  uint32_t word = std::bit_cast<uint32_t>(x == 0.f ? 0.f : x);
  for (int i = 0; i < 4; ++i) {
    h ^= (word >> (8 * i)) & 255;
    h *= 1099511628211ULL;
  }
}
void interval(IAudioProcessor &p, int block_size, int active_frames, bool fault,
              bool flood, bool persistence) {
  Block block;
  std::array<std::array<double, 4096>, 2> reference{};
  std::vector<uint64_t> durations;
  durations.reserve(size_t(active_frames) + 1024);
  uint64_t seed = 0;
  std::ifstream random("/dev/urandom", std::ios::binary);
  random.read(reinterpret_cast<char *>(&seed), sizeof(seed));
  need(bool(random), "host input seed");
  // Selection is after Windows activation, never an endpoint-side fixed recipe.
  uint64_t x = seed, position = 0, compared = 0,
           input_hash = 14695981039346656037ULL, output_hash = input_hash,
           overruns = 0, zero_gain = 0, one_silent = 0, inplace_count = 0,
           flushes = 0;
  double maximum = 0., gain = persistence ? .25 : 1.;
  bool saw_fault = false;
  std::cout << "{\"event\":\"ap3_host_ready\",\"seed\":" << seed
            << ",\"seed_after_activation\":true}" << std::endl;
  stage = "start";
  ok(callback([&] { return p.setProcessing(true); }), "queued start");
  const auto start = Clock::now();
  for (uint64_t b = 0; position < uint64_t(active_frames + 1024); ++b) {
    int n = block_size ? block_size : std::array<int, 4>{1, 16, 63, 256}[b % 4];
    n = std::min(n, active_frames + 1024 - int(position));
    if (position < uint64_t(active_frames))
      n = std::min(n, active_frames - int(position));
    last_position = position;
    bool tail = position >= uint64_t(active_frames), update = b % 5 != 1,
         inplace = b % 7 == 3;
    if (b == 0)
      update = false;
    if (update)
      gain = std::array<double, 4>{.5, .25, 0., .75}[(b / 5) % 4];
    if (b % 19 == 18) {
      ProcessData flush;
      flush.processMode = kRealtime;
      flush.symbolicSampleSize = kSample32;
      ParameterChanges q(1);
      int32 i = 0, j = 0;
      q.addParameterData(0, i)->addPoint(0, .25, j);
      flush.inputParameterChanges = &q;
      ok(callback([&] { return p.process(flush); }), "zero-frame gain flush");
      gain = .25;
      update = false;
      ++flushes;
    }
    uint64_t flags = tail || b % 17 == 8 ? 3 : b % 13 == 4 ? 1 : 0;
    block.prepare(n, gain, update, inplace, flags);
    for (int i = 0; i < n; ++i)
      for (int ch = 0; ch < 2; ++ch) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        float input =
            flags & (uint64_t(1) << ch) ? 0.f : float(int(x % 65) - 32) / 256.f;
        block.in[ch][i] = input;
        reference[ch][(position + i) % 4096] = double(input) * gain;
        hash(input_hash, input);
      }
    auto before = block.input;
    if (!flood)
      std::this_thread::sleep_until(
          start + std::chrono::nanoseconds(position * 1000000000ULL / 48000));
    stage = "process";
    auto t = Clock::now();
    auto result = callback([&] { return p.process(block.data); });
    auto elapsed = uint64_t(
        std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() - t)
            .count());
    durations.push_back(elapsed);
    overruns += elapsed > uint64_t(n) * 1000000000ULL / 48000;
    if (result != kResultOk) {
      need(fault, "queued process failed");
      for (int ch = 0; ch < 2; ++ch)
        for (int i = 0; i < n; ++i)
          need(block.out[ch][i] == 0.f, "fault output clear");
      need(callback([&] { return p.process(block.data); }) != kResultOk,
           "fault replay refused");
      saw_fault = true;
      break;
    }
    for (int ch = 0; ch < 2; ++ch) {
      const auto &returned = inplace ? block.input[ch] : block.output[ch];
      need(std::bit_cast<uint32_t>(returned[0]) == 0x4b123456 &&
               std::bit_cast<uint32_t>(returned[n + 1]) == 0x4b123456,
           "output guard");
      if (!inplace)
        need(block.input[ch] == before[ch], "input unchanged");
      need((block.ob.silenceFlags & ~uint64_t(3)) == 0, "output silence bits");
      for (int i = 0; i < n; ++i) {
        double expected = position + i < 1024
                              ? 0.
                              : reference[ch][(position + i - 1024) % 4096];
        double actual = block.out[ch][i];
        double error = std::abs(actual - expected);
        maximum = std::max(maximum, error);
        need(std::isfinite(actual) && error == 0.,
             "sample alignment or numerical mismatch");
        need(!(block.ob.silenceFlags & (uint64_t(1) << ch)) || actual == 0.,
             "false output silence claim");
        hash(output_hash, float(actual));
        ++compared;
      }
    }
    zero_gain += gain == 0. && flags == 0;
    one_silent += flags == 1;
    inplace_count += inplace;
    position += uint64_t(n);
  }
  if (fault)
    need(saw_fault, "injected fault must reach proxy boundary");
  if (!fault) {
    ProcessData flush;
    flush.processMode = kRealtime;
    flush.symbolicSampleSize = kSample32;
    ParameterChanges q(1);
    int32 i = 0, j = 0;
    q.addParameterData(0, i)->addPoint(0, .25, j);
    flush.inputParameterChanges = &q;
    ok(callback([&] { return p.process(flush); }), "retained gain flush");
  }
  stage = "stop";
  auto stopped = callback([&] { return p.setProcessing(false); });
  need(fault ? stopped != kResultOk : stopped == kResultOk,
       "queued stop disposition");
  std::sort(durations.begin(), durations.end());
  need(!durations.empty(), "callback observations");
  std::cout << "{\"event\":\"ap3_host_compared\",\"frames_per_callback\":"
            << block_size << ",\"active_frames\":" << active_frames
            << ",\"samples\":" << compared << ",\"max_error\":" << maximum
            << ",\"latency_samples\":1024,\"callback_median_ns\":"
            << durations[durations.size() / 2] << ",\"callback_p99_ns\":"
            << durations[(durations.size() - 1) * 99 / 100]
            << ",\"callback_max_ns\":" << durations.back()
            << ",\"callback_overruns\":" << overruns
            << ",\"callback_effects\":0,\"input_fnv1a64\":" << input_hash
            << ",\"output_fnv1a64\":" << output_hash
            << ",\"zero_gain_blocks\":" << zero_gain
            << ",\"one_silent_blocks\":" << one_silent
            << ",\"inplace_blocks\":" << inplace_count
            << ",\"zero_frame_flushes\":" << flushes
            << ",\"fault_observed\":" << (saw_fault ? "true" : "false") << "}"
            << std::endl;
  // Pacing jitter at 1 frame is not the measured 128/256-frame acceptance case.
  if (!fault && active_frames == 1440000)
    need(overruns == 0, "measured callback deadline overrun");
  // Leave a known gain for the next processing interval in this same instance.
}
#include "state_cases.h"
#include "instances_cases.h"
} // namespace
int main(int argc, char **argv) {
  try {
    need(argc == 3, "usage: ap3-sustained-host BUNDLE CASE");
    std::string scenario = argv[2], error;
    audit_begin =
        reinterpret_cast<Mark>(dlsym(RTLD_DEFAULT, "ap3_audit_begin"));
    audit_end = reinterpret_cast<Count>(dlsym(RTLD_DEFAULT, "ap3_audit_end"));
    need(audit_begin && audit_end, "callback instrumentation required");
    // Positive control: prove the exact interposer counts an allocation/free.
    auto allocate =
        reinterpret_cast<void *(*)(size_t)>(dlsym(RTLD_DEFAULT, "malloc"));
    auto release =
        reinterpret_cast<void (*)(void *)>(dlsym(RTLD_DEFAULT, "free"));
    need(allocate && release, "allocator instrumentation symbols");
    audit_begin();
    void *allocation = allocate(32);
    release(allocation);
    need(audit_end() >= 2, "callback instrumentation positive control");
    auto module = VST3::Hosting::Module::create(argv[1], error);
    need(bool(module), "SDK module load");
    auto host = owned(new HostApplication);
    module->getFactory().setHostContext(host);
    auto classes = module->getFactory().classInfos();
    need(classes.size() == 2, "preview processor/controller factory");
    need(classes[0].ID().toString() == "84E8DE5F92554F5396FAE4133C935A18",
         "processor identity");
    if (scenario.starts_with("instances-")) {
      instanceCases(module->getFactory(), host, scenario);
      host = nullptr; module.reset(); return 0;
    }
    auto component =
        module->getFactory().createInstance<IComponent>(classes[0].ID());
    need(bool(component), "factory component");
    ok(component->initialize(host), "initialize");
    FUnknownPtr<IAudioProcessor> p(component);
    need(bool(p), "processor interface");
    TUID cid{};
    ok(component->getControllerClassId(cid), "bridge controller identity");
    need(FUID(cid) == FUID(0xD1444DE3, 0x38814391, 0xA916DC9C, 0xFCC67008),
         "bridge owned controller");
    auto controller =
        module->getFactory().createInstance<IEditController>(classes[1].ID());
    need(bool(controller), "controller factory");
    ok(controller->initialize(host), "controller initialize");
    ParameterInfo info{};
    ok(controller->getParameterInfo(0, info), "gain metadata");
    need(controller->getParameterCount() == 1 && info.id == 0 &&
             info.defaultNormalizedValue == 1.,
         "reference gain definition");
    const bool state_case = scenario.starts_with("state-");
    bool fault = false;
    std::exception_ptr failure;
    const char *failure_stage = nullptr;
    if (state_case) {
      stateCases(*component, *p, *controller, scenario);
    } else {
      ProcessSetup setup{kRealtime, kSample32, 256, 48000.};
      ok(p->setupProcessing(setup), "realtime setup");
      need(p->getLatencySamples() == 1024 && p->getTailSamples() == 0,
           "fixed transport latency");
      ok(component->activateBus(kAudio, kInput, 0, true), "input active");
      ok(component->activateBus(kAudio, kOutput, 0, true), "output active");
      stage = "activate";
      ok(component->setActive(true), "Windows activate");
      fault =
          scenario != "positive" && scenario != "core" && scenario != "reopen";
      std::thread audio([&] {
        try {
          if (scenario == "overflow") {
            bool rejected = false;
            for (int i = 0; i < 4096; ++i) {
              auto op = callback([&] { return p->setProcessing(true); });
              if (op != kResultOk) {
                rejected = true;
                break;
              }
              if (callback([&] { return p->setProcessing(false); }) !=
                  kResultOk) {
                rejected = true;
                break;
              }
            }
            need(rejected, "descriptor overflow must reach proxy boundary");
            need(callback([&] { return p->setProcessing(true); }) != kResultOk,
                 "overflow latch");
            return;
          }
          interval(*p, 128, scenario == "core" ? 1440000 : 300000, fault,
                   scenario == "overflow", false);
          if (!fault && scenario != "reopen")
            interval(*p, scenario == "core" ? 256 : 0,
                     scenario == "core" ? 1440000 : 8192, false, false, true);
        } catch (...) {
          failure_stage = stage;
          failure = std::current_exception();
        }
      });
      audio.join();
      if (!fault && !failure) {
        stage = "deactivate";
        ok(component->setActive(false), "Windows deactivate");
      }
    }
    stage = "terminate";
    auto terminated = component->terminate();
    if (!fault && !failure)
      ok(terminated, "clean terminate");
    p = nullptr;
    component = nullptr;
    ok(controller->terminate(), "controller terminate");
    controller = nullptr;
    host = nullptr;
    module.reset();
    std::cout << "{\"event\":\"ap3_host_closed\",\"terminate_result\":"
              << terminated
              << ",\"references_released\":true,\"module_unloaded\":true}"
              << std::endl;
    if (failure) {
      stage = failure_stage;
      std::rethrow_exception(failure);
    }
    return 0;
  } catch (const std::exception &e) {
    std::cout << "{\"event\":\"ap3_host_error\",\"stage\":\"" << stage
              << "\",\"detail\":\"" << e.what()
              << "\",\"last_position\":" << last_position << "}" << std::endl;
    return 1;
  }
}
