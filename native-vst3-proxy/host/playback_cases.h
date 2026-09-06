// Deterministic peer gates are test-driver work, outside the audited callback.
// They make one exact presentation span late without depending on OS timing.
template<class Factory>
void playbackCases(const Factory &factory, HostApplication *host, const std::string &scenario) {
  auto classes = factory.classInfos();
  auto create = [&](const char *file, double gain) {
    IndependentInstance i;
    i.component = factory.template createInstance<IComponent>(classes[0].ID());
    ok(i.component->initialize(host), "playback initialize");
    i.processor = FUnknownPtr<IAudioProcessor>(i.component);
    i.controller = factory.template createInstance<IEditController>(classes[1].ID());
    ok(i.controller->initialize(host), "playback controller");
    restore(*i.component, *i.controller, loadBytes(file), gain);
    ProcessSetup setup{kRealtime, kSample32, 256, 48000.};
    ok(i.processor->setupProcessing(setup), "playback setup");
    need(i.processor->getLatencySamples() == 1024, "unchanged playback latency");
    ok(i.component->setActive(true), "playback activate");
    return i;
  };
  auto a = create("instance-a.state", .25), b = create("instance-b.state", .75);
  IndependentAudio sibling(.75);
  std::thread other([&] { independentAudio(*b.processor, sibling, .75, 17, false); });
  std::exception_ptr error;
  uint64_t position = 0, gaps = 0, samples = 0;
  auto store = std::string(std::getenv("LVB_AP4_STATE_STORE"));
  auto through = [&](uint64_t target) {
    auto end = Clock::now() + std::chrono::seconds(3);
    for (;;) {
      uint64_t value = 0; std::ifstream(store + "/a-through") >> value;
      if (value >= target) return;
      need(Clock::now() < end, "delayed peer progress");
      std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
  };
  try {
    waitFrames(sibling, 2048);
    ok(callback([&] { return a.processor->setProcessing(true); }), "playback start");
    Block block;
    std::array<std::array<float, 32768>, 2> history{};
    auto process = [&](int n, bool missing, bool edit = false) {
      block.prepare(n, .5, edit, false, 0);
      for (int ch = 0; ch < 2; ++ch) for (int j = 0; j < n; ++j) {
        auto index = position + unsigned(j);
        float value = float(1 + index % 31 + 37 * unsigned(ch)) / 128.f;
        block.in[ch][j] = value;
        history[ch][index] = value * (edit || position >= 4096 ? .375f : .125f);
      }
      ok(callback([&] { return a.processor->process(block.data); }), "underrun remains successful");
      for (int ch = 0; ch < 2; ++ch) for (int j = 0; j < n; ++j) {
        auto index = position + unsigned(j);
        float expected = index < 1024 || missing ? 0.f : history[ch][index - 1024];
        need(block.out[ch][j] == expected, "late output absolute alignment / ordered controls");
        ++samples;
      }
      if (missing) gaps += unsigned(n);
      position += unsigned(n);
    };
    for (int i = 0; i < 4; ++i) process(256, false);
    if (scenario == "playback-partial") process(128, true);
    else for (int i = 0; i < 3; ++i) process(256, true);
    { std::ofstream release(store + "/release-a"); release << "release\n"; }
    through(position);
    // The next read must expire only the past prefix of the first valid result.
    process(scenario == "playback-partial" ? 128 : 256, false);
    through(position);
    auto state = snapshot(*a.component, *a.controller, .25, "unchanged_after_gap");
    need(state == loadBytes("instance-a.state"), "gap must preserve complete state");
    while (position < 4096) { process(256, false); through(position); }
    process(256, false, true); through(position);
    while (position < 8192) { process(256, false); through(position); }
    snapshot(*a.component, *a.controller, .5, "ordered_edit_after_gap");
    ok(callback([&] { return a.processor->setProcessing(false); }), "playback stop");
    ok(a.component->setActive(false), "playback deactivate");
    auto before = sibling.frames.load();
    ok(a.component->terminate(), "same connection closes normally");
    waitFrames(sibling, before + 4096);
  } catch (...) { error = std::current_exception(); }
  sibling.stop = true; other.join();
  if (sibling.error) std::rethrow_exception(sibling.error);
  if (error) std::rethrow_exception(error);
  ok(b.component->setActive(false), "sibling deactivate");
  ok(b.component->terminate(), "sibling terminate");
  ok(a.controller->terminate(), "playback controller terminate");
  ok(b.controller->terminate(), "sibling controller terminate");
  std::cout << "{\"event\":\"ap7_playback\",\"case\":\"" << scenario
            << "\",\"missing_frames\":" << gaps << ",\"compared_samples\":" << samples
            << ",\"sibling_samples\":" << sibling.samples
            << ",\"terminal_failures\":0,\"maximum_error\":0,\"callback_effects\":0}" << std::endl;
}
