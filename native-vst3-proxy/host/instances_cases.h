// Two standard VST3 objects in one module/process. Each audio thread has its own
// buffers, reference history and input sequence; lifecycle stays on the owner.
struct IndependentAudio {
  std::atomic<bool> stop{false};
  std::atomic<double> gain;
  std::atomic<uint64_t> frames{0};
  uint64_t samples = 0, input_hash = 14695981039346656037ULL,
           output_hash = 14695981039346656037ULL;
  bool fault = false;
  std::exception_ptr error;
  explicit IndependentAudio(double g) : gain(g) {}
};
void independentAudio(IAudioProcessor &p, IndependentAudio &result,
                      double restored, unsigned salt, bool expect_fault) {
  try {
    Block block;
    std::array<std::array<float, 4096>, 2> expected{};
    uint64_t position = 0, tail = UINT64_MAX;
    double applied = restored;
    ok(callback([&] { return p.setProcessing(true); }), "independent start");
    const auto start = Clock::now();
    while (position < 48000 * 15 && position < tail) {
      if (result.stop.load() && tail == UINT64_MAX) tail = position + 1024;
      double requested = result.gain.load();
      block.prepare(256, requested, requested != applied, false, 0);
      applied = requested;
      for (int ch = 0; ch < 2; ++ch) for (int i = 0; i < 256; ++i) {
        float input = tail != UINT64_MAX ? 0.f :
            float(int((position + unsigned(i) + salt * unsigned(ch + 1)) % (17 + salt)) - 16) / 256.f;
        block.in[ch][i] = input;
        expected[ch][(position + unsigned(i)) % 4096] = input * float(applied);
        hash(result.input_hash, input);
      }
      std::this_thread::sleep_until(start + std::chrono::nanoseconds(position * 1000000000ULL / 48000));
      auto r = callback([&] { return p.process(block.data); });
      if (r != kResultOk) {
        need(expect_fault, "healthy sibling callback failed");
        for (int ch = 0; ch < 2; ++ch) for (int i = 0; i < 256; ++i)
          need(block.out[ch][i] == 0.f, "failed instance silence");
        need(callback([&] { return p.process(block.data); }) != kResultOk, "failed instance replay refused");
        result.fault = true;
        break;
      }
      for (int ch = 0; ch < 2; ++ch) for (int i = 0; i < 256; ++i) {
        float reference = position + unsigned(i) < 1024 ? 0.f :
            expected[ch][(position + unsigned(i) - 1024) % 4096];
        need(block.out[ch][i] == reference, "independent route/state sample mismatch");
        hash(result.output_hash, block.out[ch][i]); ++result.samples;
      }
      position += 256;
      result.frames.store(position);
    }
    need(result.fault == expect_fault, "fault disposition");
    auto r = callback([&] { return p.setProcessing(false); });
    need(r == kResultOk, "independent stop, including a failed endpoint");
  } catch (...) { result.error = std::current_exception(); }
}
void waitFrames(IndependentAudio &audio, uint64_t count) {
  auto until = Clock::now() + std::chrono::seconds(5);
  while (audio.frames.load() < count) {
    need(Clock::now() < until, "sibling audio progress timeout");
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
}
struct IndependentInstance {
  IPtr<IComponent> component;
  FUnknownPtr<IAudioProcessor> processor;
  IPtr<IEditController> controller;
};
template<class Factory>
void instanceCases(const Factory &factory, HostApplication *host, const std::string &scenario) {
  auto classes = factory.classInfos();
  auto create = [&](const char *file, double gain) {
    IndependentInstance i;
    i.component = factory.template createInstance<IComponent>(classes[0].ID());
    need(bool(i.component), "sibling component");
    ok(i.component->initialize(host), "sibling initialize");
    i.processor = FUnknownPtr<IAudioProcessor>(i.component);
    i.controller = factory.template createInstance<IEditController>(classes[1].ID());
    need(bool(i.processor) && bool(i.controller), "sibling interfaces");
    ok(i.controller->initialize(host), "sibling controller");
    restore(*i.component, *i.controller, loadBytes(file), gain);
    ProcessSetup setup{kRealtime, kSample32, 256, 48000.};
    ok(i.processor->setupProcessing(setup), "sibling setup");
    need(i.processor->getLatencySamples() == 1024, "per-instance latency");
    ok(i.component->setActive(true), "sibling activate");
    return i;
  };
  auto close = [&](IndependentInstance &i, bool failed) {
    if (!failed) ok(i.component->setActive(false), "sibling deactivate");
    auto result = i.component->terminate();
    need(failed ? result != kResultOk : result == kResultOk, "sibling termination result");
    i.processor = nullptr; i.component = nullptr;
    ok(i.controller->terminate(), "sibling controller terminate"); i.controller = nullptr;
  };
  const bool fail = scenario == "instances-failure";
  IndependentAudio a(.25), b(.75);
  auto one = create("instance-a.state", .25);
  IndependentInstance two;
  // In the healthy case B's peer deliberately delays startup while A runs.
  std::thread first, second;
  struct JoinThreads {
    IndependentAudio &a, &b;
    std::thread &first, &second;
    ~JoinThreads() {
      a.stop.store(true); b.stop.store(true);
      if (first.joinable()) first.join();
      if (second.joinable()) second.join();
    }
  } joined{a, b, first, second};
  if (!fail) first = std::thread([&] { independentAudio(*one.processor, a, .25, 3, false); });
  if (!fail) waitFrames(a, 2048);
  two = create("instance-b.state", .75);
  second = std::thread([&] { independentAudio(*two.processor, b, .75, 17, false); });
  if (fail) first = std::thread([&] { independentAudio(*one.processor, a, .25, 3, true); });
  // Always release audio threads on a failed assertion; never std::terminate.
  std::exception_ptr error;
  try {
    waitFrames(b, 4096);
    if (!fail) {
      a.gain.store(.5);
      waitFrames(a, a.frames.load() + 4096);
      auto saved_a = snapshot(*one.component, *one.controller, .5, "independent_a");
      auto saved_b = snapshot(*two.component, *two.controller, .75, "independent_b");
      need(saved_a != saved_b, "independent saved state");
      a.stop.store(true);
    }
    first.join();
    if (a.error) std::rethrow_exception(a.error);
    if (fail) {
      LVBState::Stream s({}, LVBState::payloadLimit + LVBState::overhead);
      need(one.component->getState(&s) != kResultOk, "failed state cannot use defaults");
    }
    uint64_t before = b.frames.load();
    close(one, fail); // healthy test's peer delays this close by 0.5 seconds
    waitFrames(b, before + 48000);
    snapshot(*two.component, *two.controller, .75, "survivor_after_sibling_close");
  } catch (...) { error = std::current_exception(); }
  a.stop.store(true); b.stop.store(true);
  if (first.joinable()) first.join();
  second.join();
  if (a.error) std::rethrow_exception(a.error);
  if (b.error) std::rethrow_exception(b.error);
  if (error) std::rethrow_exception(error);
  close(two, false);
  need(a.input_hash != b.input_hash && a.output_hash != b.output_hash, "distinct input/output routes");
  std::cout << "{\"event\":\"ap5_two_instances\",\"case\":\"" << scenario
            << "\",\"same_native_process\":true,\"callback_threads\":2,\"callback_effects\":0,"
            << "\"a_samples\":" << a.samples << ",\"b_samples\":" << b.samples
            << ",\"maximum_error\":0,\"a_failed\":" << (a.fault ? "true" : "false")
            << ",\"survivor_after_close_frames\":48000,\"latency_samples\":1024}"
            << std::endl;
}
