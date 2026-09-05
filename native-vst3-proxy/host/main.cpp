// Independent VST3 host. No private bridge header, mapping offsets or DSP
// fallback.
#include "pluginterfaces/base/funknown.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/module.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include <algorithm>
#include <array>
#include <bit>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <thread>
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
const char *stage = "load";
void need(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}
void ok(tresult result, const char *message) {
  need(result == kResultOk, message);
}
struct Block {
  std::array<std::array<float, 258>, 2> input{}, output{};
  std::array<float *, 2> in{}, out{};
  AudioBusBuffers ib, ob;
  ProcessData data;
  ParameterChanges parameters{2};
  Block(int n, double gain, bool empty = false, bool inplace = false) {
    for (int ch = 0; ch < 2; ++ch) {
      input[ch].fill(0.f);
      output[ch].fill(std::bit_cast<float>(uint32_t(0x7fc12345)));
      input[ch][0] = input[ch][n + 1] = output[ch][0] = output[ch][n + 1] =
          std::bit_cast<float>(uint32_t(0x4b123456));
      in[ch] = input[ch].data() + 1;
      out[ch] = (inplace ? input[ch] : output[ch]).data() + 1;
    }
    ib.numChannels = ob.numChannels = 2;
    ib.channelBuffers32 = in.data();
    ob.channelBuffers32 = out.data();
    data.processMode = kOffline;
    data.symbolicSampleSize = kSample32;
    data.numSamples = n;
    data.numInputs = data.numOutputs = 1;
    data.inputs = &ib;
    data.outputs = &ob;
    int32 index = 0, point = 0;
    auto *q = parameters.addParameterData(0, index);
    if (!empty)
      ok(q->addPoint(0, gain, point), "host parameter");
    data.inputParameterChanges = &parameters;
  }
};
void words(const float *samples, int n) {
  std::cout << '[';
  for (int i = 0; i < n; ++i) {
    if (i)
      std::cout << ',';
    std::cout << std::bit_cast<uint32_t>(samples[i]);
  }
  std::cout << ']';
}
void positive(IAudioProcessor &processor, bool reopen) {
  uint64_t seed = 0;
  std::ifstream random("/dev/urandom", std::ios::binary);
  random.read(reinterpret_cast<char *>(&seed), sizeof(seed));
  need(bool(random), "host randomness");
  std::cout << "{\"event\":\"ap2_host_ready\",\"seed\":" << seed
            << ",\"seed_after_activation\":true,\"reopen\":"
            << (reopen ? "true" : "false") << "}" << std::endl;
  constexpr int lengths[] = {1,   16, 63, 256, 16, 63, 1,
                             256, 16, 63, 17,  31, 47};
  constexpr double gains[] = {.5,  .25, .75, .5, .75, .25, .5,
                              .75, 0.,  .5,  .5, .25, .75};
  constexpr uint64_t silence[] = {0, 0, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0};
  int calls = 0, samples = 0;
  for (int b = 0; b < (reopen ? 1 : 13); ++b) {
    int n = reopen ? 19 : lengths[b];
    double gain = reopen ? 1. : gains[b];
    if (b == 11 && !reopen) {
      ProcessData flush;
      flush.processMode = kOffline;
      flush.symbolicSampleSize = kSample32;
      ParameterChanges p(1);
      int32 index = 0, point = 0;
      p.addParameterData(0, index)->addPoint(0, .25, point);
      flush.inputParameterChanges = &p;
      stage = "zero_frame_flush";
      ok(processor.process(flush), "zero-frame parameter update");
      std::cout
          << "{\"event\":\"ap2_host_flush\",\"gain\":0.25,\"audio_calls\":"
          << calls << "}" << std::endl;
    }
    bool inplace = b == 12;
    Block block(n, gain, reopen || b == 10 || b == 11, inplace);
    if (reopen || b == 11)
      block.data.inputParameterChanges = nullptr;
    block.ib.silenceFlags = reopen ? 0 : silence[b];
    uint64_t x = seed ^ (uint64_t(b + 1) * 0x9e3779b97f4a7c15ULL);
    for (int ch = 0; ch < 2; ++ch)
      for (int i = 0; i < n; ++i) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        block.in[ch][i] = (block.ib.silenceFlags & (uint64_t(1) << ch))
                              ? 0.f
                              : float(int(x % 129) - 64) / 128.f;
      }
    // Discriminating non-silent channels independent of the random seed.
    for (int ch = 0; ch < 2; ++ch)
      if (!(block.ib.silenceFlags & (uint64_t(1) << ch)))
        block.in[ch][0] = ch ? -.5f : .25f;
    if (b == 10 && !reopen) {
      // Each invalid request has an otherwise valid gain change. None may
      // commit it.
      auto reject = [&](Block &bad) {
        need(processor.process(bad.data) != kResultOk,
             "malformed process admitted");
      };
      {
        Block bad(16, .25);
        bad.data.processMode = kRealtime;
        reject(bad);
      }
      {
        Block bad(16, .25);
        bad.data.numInputs = 0;
        reject(bad);
      }
      {
        Block bad(16, .25);
        bad.ib.numChannels = 1;
        reject(bad);
      }
      {
        Block bad(16, .25);
        bad.ib.silenceFlags = uint64_t(1) << 40;
        reject(bad);
      }
      {
        Block bad(16, .25);
        bad.in[0][0] = .5f;
        bad.ib.silenceFlags = 1;
        reject(bad);
      }
      {
        Block bad(16, .25);
        bad.out[1] = bad.out[0];
        reject(bad);
      }
      {
        Block bad(16, .25);
        int32 q = 0, point = 0;
        bad.parameters.addParameterData(2, q)->addPoint(0, 1., point);
        reject(bad);
      }
      {
        Block bad(16, .25);
        int32 q = 0, point = 0;
        bad.parameters.addParameterData(55, q)->addPoint(0, .5, point);
        reject(bad);
      }
      {
        Block bad(16, .25);
        int32 point = 0;
        bad.parameters.getParameterData(0)->addPoint(1, .75, point);
        reject(bad);
      }
      {
        Block bad(16, .25);
        bad.data.symbolicSampleSize = kSample64;
        reject(bad);
      }
      std::cout
          << "{\"event\":\"ap2_host_rejections\",\"count\":10,\"audio_calls\":"
          << calls << "}" << std::endl;
    }
    auto before = block.input;
    stage = "host_process";
    auto result = processor.process(block.data);
    ++calls;
    double maximum = 0.;
    bool valid =
        result == kResultOk && (block.ob.silenceFlags & ~uint64_t(3)) == 0;
    for (int ch = 0; ch < 2; ++ch) {
      const auto &returned = inplace ? block.input[ch] : block.output[ch];
      valid = valid && std::bit_cast<uint32_t>(returned[0]) == 0x4b123456 &&
              std::bit_cast<uint32_t>(returned[n + 1]) == 0x4b123456;
      for (int i = 0; i < n; ++i) {
        double actual = block.out[ch][i];
        double expected = double(before[ch][i + 1]) * gain;
        double error = std::abs(actual - expected);
        maximum = std::max(maximum, error);
        valid =
            valid && std::isfinite(actual) && error == 0. &&
            (!(block.ob.silenceFlags & (uint64_t(1) << ch)) || actual == 0.);
      }
      if (!inplace)
        valid = valid && block.input[ch] == before[ch];
    }
    std::cout << "{\"event\":\"ap2_host_block\",\"block\":" << b
              << ",\"frames\":" << n << ",\"gain\":" << gain
              << ",\"input_silence_flags\":" << block.ib.silenceFlags
              << ",\"output_silence_flags\":" << block.ob.silenceFlags
              << ",\"result\":" << result
              << ",\"in_place\":" << (inplace ? "true" : "false")
              << ",\"max_error\":" << maximum
              << ",\"comparison_ok\":" << (valid ? "true" : "false")
              << ",\"input_bits\":[";
    words(before[0].data() + 1, n);
    std::cout << ',';
    words(before[1].data() + 1, n);
    std::cout << "],\"output_bits\":[";
    words(block.out[0], n);
    std::cout << ',';
    words(block.out[1], n);
    std::cout << "]}" << std::endl;
    need(valid, "actual host buffer comparison");
    samples += n * 2;
  }
  std::cout << "{\"event\":\"ap2_host_compared\",\"samples\":" << samples
            << ",\"audio_calls\":" << calls << ",\"max_error\":0}" << std::endl;
}
} // namespace
int run(int argc, char **argv) {
  try {
    need(argc == 3, "usage: ap2-offline-host BUNDLE CASE");
    std::string scenario = argv[2], error;
    auto module = VST3::Hosting::Module::create(argv[1], error);
    need(bool(module), "SDK module load");
    auto host = owned(new HostApplication);
    module->getFactory().setHostContext(host);
    auto classes = module->getFactory().classInfos();
    need(classes.size() == 1 && classes[0].category() == kVstAudioEffectClass,
         "processor-only factory");
    need(classes[0].ID().toString() == "84E8DE5F92554F5396FAE4133C935A18",
         "processor identity");
    auto component =
        module->getFactory().createInstance<IComponent>(classes[0].ID());
    need(bool(component), "factory instance");
    stage = "initialize";
    ok(component->initialize(host), "initialize");
    FUnknownPtr<IAudioProcessor> processor(component);
    need(bool(processor), "query processor");
    TUID controller{};
    need(component->getControllerClassId(controller) != kResultOk,
         "controller unsupported");
    need(component->getBusCount(kAudio, kInput) == 1 &&
             component->getBusCount(kAudio, kOutput) == 1 &&
             component->getBusCount(kEvent, kInput) == 0,
         "bus counts");
    BusInfo info{};
    ok(component->getBusInfo(kAudio, kInput, 0, info), "input BusInfo");
    need(info.channelCount == 2, "stereo BusInfo");
    need(processor->canProcessSampleSize(kSample32) == kResultOk &&
             processor->canProcessSampleSize(kSample64) != kResultOk &&
             processor->getLatencySamples() == 0 &&
             processor->getTailSamples() == 0,
         "processor capabilities");
    ProcessSetup setup{kOffline, kSample32, 256, 48000.};
    if (scenario == "discovery") {
      Block b(1, .5);
      need(processor->process(b.data) != kResultOk, "process before setup");
      need(processor->setProcessing(true) != kResultOk,
           "start before activation");
      for (auto mode : {kRealtime, kPrefetch}) {
        auto bad = setup;
        bad.processMode = mode;
        need(processor->setupProcessing(bad) != kResultOk,
             "unsupported realtime setup");
      }
      auto bad = setup;
      bad.symbolicSampleSize = kSample64;
      need(processor->setupProcessing(bad) != kResultOk,
           "unsupported sample size");
      bad = setup;
      bad.maxSamplesPerBlock = 257;
      need(processor->setupProcessing(bad) != kResultOk, "oversized setup");
      SpeakerArrangement mono = SpeakerArr::kMono;
      need(processor->setBusArrangements(&mono, 1, &mono, 1) != kResultOk,
           "mono setup");
    } else {
      stage = "setup";
      ok(processor->setupProcessing(setup), "setup");
      ok(component->activateBus(kAudio, kInput, 0, true), "input active");
      ok(component->activateBus(kAudio, kOutput, 0, true), "output active");
      stage = "activate";
      auto active = component->setActive(true);
      if (scenario == "unavailable") {
        need(active != kResultOk, "unavailable backend cannot succeed");
      } else {
        ok(active, "Windows activation");
        auto duplicate =
            module->getFactory().createInstance<IComponent>(classes[0].ID());
        ok(duplicate->initialize(host), "second initialize");
        FUnknownPtr<IAudioProcessor> second(duplicate);
        ok(second->setupProcessing(setup), "second setup");
        need(duplicate->setActive(true) != kResultOk,
             "double attachment refused");
        duplicate->terminate();
        second = nullptr;
        duplicate = nullptr;
        std::exception_ptr failure;
        bool failed = scenario == "disconnect" || scenario == "timeout" ||
                      scenario == "invalid" || scenario == "flags" ||
                      scenario == "silence" || scenario == "nonfinite";
        std::thread worker([&] {
          try {
            stage = "start";
            ok(processor->setProcessing(true), "Windows start");
            if (failed) {
              Block b(16, .5);
              std::fill_n(b.in[0], 16, .5f);
              std::fill_n(b.in[1], 16, -.5f);
              stage = "fault_process";
              need(processor->process(b.data) != kResultOk,
                   "backend failure must fail VST3");
              for (int ch = 0; ch < 2; ++ch)
                for (int i = 0; i < 16; ++i)
                  need(b.out[ch][i] == 0.f, "defensive output clear");
              need(processor->process(b.data) != kResultOk,
                   "failed session refuses replay");
              need(processor->setProcessing(false) != kResultOk,
                   "failed stop explicit");
            } else {
              if (scenario != "zero")
                positive(*processor, scenario == "reopen");
              stage = "stop";
              ok(processor->setProcessing(false), "Windows stop");
            }
          } catch (...) {
            failure = std::current_exception();
          }
        });
        worker.join();
        if (failure)
          std::rethrow_exception(failure);
        if (!failed) {
          stage = "deactivate";
          ok(component->setActive(false), "Windows deactivate");
        }
      }
    }
    stage = "terminate";
    auto terminated = component->terminate();
    if (scenario == "discovery" || scenario == "positive" ||
        scenario == "reopen" || scenario == "zero")
      ok(terminated, "terminate");
    processor = nullptr;
    component = nullptr;
    host = nullptr;
    stage = "unload";
    module.reset();
    std::cout << "{\"event\":\"ap2_host_closed\",\"case\":\"" << scenario
              << "\",\"terminate_result\":" << terminated
              << ",\"references_released\":true,\"module_unloaded\":true}"
              << std::endl;
    return 0;
  } catch (const std::exception &e) {
    std::cout << "{\"event\":\"ap2_host_error\",\"stage\":\"" << stage
              << "\",\"detail\":\"" << e.what() << "\"}" << std::endl;
    return 1;
  }
}

int main(int argc, char **argv) {
  if (argc != 3 || std::string(argv[2]) != "batch")
    return run(argc, argv);
  char positive_case[] = "positive";
  char reopen_case[] = "reopen";
  char *first[] = {argv[0], argv[1], positive_case};
  int result = run(3, first);
  if (result)
    return result;
  // Test orchestration only: the first module and all SDK references are gone.
  // The external supervisor supplies a fresh exact binding after owned cleanup.
  const char *directory = std::getenv("LVB_AP2_SESSION_DIR");
  const char *next = std::getenv("LVB_AP2_REOPEN_SESSION");
  if (!directory || !next)
    return 2;
  auto gate = std::filesystem::path(directory) / "ap2.reopen";
  auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
  while (!std::filesystem::exists(gate)) {
    if (std::chrono::steady_clock::now() > deadline)
      return 3;
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
  }
  if (setenv("LVB_AP2_SESSION", next, 1))
    return 4;
  char *second[] = {argv[0], argv[1], reopen_case};
  return run(3, second);
}
