// AP6 uses only public SDK interfaces, including the same controller action as
// the desktop host. Windows peers are substituted by recovery_proxy.py locally.
class RecoveryHandler final : public IComponentHandler {
  uint32 refs = 1;
public:
  IEditController *controller;
  double cached = 1.;
  explicit RecoveryHandler(IEditController *c) : controller(c) {}
  tresult PLUGIN_API queryInterface(const TUID id, void **out) override {
    if (!out) return kInvalidArgument;
    if (FUnknownPrivate::iidEqual(id, IComponentHandler::iid) || FUnknownPrivate::iidEqual(id, FUnknown::iid)) {
      *out = static_cast<IComponentHandler *>(this); addRef(); return kResultOk;
    }
    *out = nullptr; return kNoInterface;
  }
  uint32 PLUGIN_API addRef() override { return ++refs; }
  uint32 PLUGIN_API release() override { auto n = --refs; if (!n) delete this; return n; }
  tresult PLUGIN_API beginEdit(ParamID) override { return kResultOk; }
  tresult PLUGIN_API performEdit(ParamID, ParamValue) override { return kResultOk; }
  tresult PLUGIN_API endEdit(ParamID) override { return kResultOk; }
  tresult PLUGIN_API restartComponent(int32 flags) override {
    if (flags & kParamValuesChanged) cached = controller->getParamNormalized(0);
    return kResultOk;
  }
};
struct RecoveryAudio {
  std::atomic<bool> stop{false}, failed{false};
  std::atomic<uint64_t> recovered_samples{0};
  std::exception_ptr error;
};
void recoveryAudio(IAudioProcessor &processor, RecoveryAudio &state, bool missing) {
  try {
    Block b;
    uint64_t before = 0, after = 0, calls = 0;
    bool saw_failure = false;
    ok(callback([&] { return processor.setProcessing(true); }), "recovery initial start");
    const auto begin = Clock::now();
    while (!state.stop && calls < 48000 * 15 / 256) {
      b.prepare(256, .875, calls == 0, false, 0); // deliberately uncaptured edit
      for (int ch = 0; ch < 2; ++ch) for (int i = 0; i < 256; ++i) b.in[ch][i] = .125f * float(ch + 1);
      std::this_thread::sleep_until(begin + std::chrono::nanoseconds(calls * 256 * 1000000000ULL / 48000));
      auto result = callback([&] { return processor.process(b.data); });
      if (result != kResultOk) {
        for (int ch = 0; ch < 2; ++ch) for (int i = 0; i < 256; ++i)
          need(b.out[ch][i] == 0.f, "recovery failure silence");
        saw_failure = true; state.failed = true;
      } else {
        auto position = saw_failure ? after : before;
        // Complete stored state is gain .25 AND gain-reduction .125. A gain-only
        // replacement would produce .25 and must fail this independent oracle.
        float factor = saw_failure ? .125f : (missing ? .875f : .75f);
        for (int ch = 0; ch < 2; ++ch) for (int i = 0; i < 256; ++i)
          need(b.out[ch][i] == (position + unsigned(i) < 1024 ? 0.f : .125f * float(ch + 1) * factor),
               "recovery complete-state output or restart alignment");
        if (saw_failure) { after += 256; state.recovered_samples += 512; }
        else before += 256;
      }
      ++calls;
    }
    need(saw_failure, "controlled endpoint failure reached native callback");
    ok(callback([&] { return processor.setProcessing(false); }), "recovery stop");
  } catch (...) { state.error = std::current_exception(); }
}
template<class Factory>
void recoveryCases(const Factory &factory, HostApplication *host, const std::string &scenario) {
  const bool missing = scenario == "recovery-missing";
  const bool success = scenario == "recovery-complete" || scenario == "recovery-late";
  auto classes = factory.classInfos();
  auto create = [&](const char *file, double gain) {
    IndependentInstance i;
    i.component = factory.template createInstance<IComponent>(classes[0].ID());
    ok(i.component->initialize(host), "recovery initialize");
    i.processor = FUnknownPtr<IAudioProcessor>(i.component);
    i.controller = factory.template createInstance<IEditController>(classes[1].ID());
    ok(i.controller->initialize(host), "recovery controller");
    for (int index = 0; index < 3; ++index) {
      ParameterInfo info{};
      ok(i.controller->getParameterInfo(index, info), "parameter info");
      need(info.id == (index == 0 ? 0u : 0x41503600u + unsigned(index)), "independent parameter ids");
    }
    if (file) restore(*i.component, *i.controller, loadBytes(file), gain);
    ProcessSetup setup{kRealtime, kSample32, 256, 48000.};
    ok(i.processor->setupProcessing(setup), "recovery setup");
    ok(i.component->setActive(true), "recovery activate");
    return i;
  };
  auto a = create(missing ? nullptr : "instance-a.state", .25);
  auto b = create("instance-b.state", .75);
  auto handler = owned(new RecoveryHandler(a.controller));
  ok(a.controller->setComponentHandler(handler), "recovery host handler");
  FUnknownPtr<IConnectionPoint> component(a.component), controller(a.controller);
  need(component && controller, "SDK connection points");
  ok(component->connect(controller), "processor connection");
  ok(controller->connect(component), "controller connection");
  ok(a.controller->setParamNormalized(0, .875), "uncaptured controller edit");
  handler->cached = .875;
  RecoveryAudio first; IndependentAudio second(.75);
  std::thread ta([&] { recoveryAudio(*a.processor, first, missing); });
  std::thread tb([&] { independentAudio(*b.processor, second, .75, 17, false); });
  std::exception_ptr error;
  try {
    const auto end = Clock::now() + std::chrono::seconds(5);
    while (!first.failed) { need(Clock::now() < end, "recovery fault deadline"); std::this_thread::sleep_for(std::chrono::milliseconds(1)); }
    uint64_t sibling_before = second.frames;
    LVBState::Stream failed_save({}, LVBState::payloadLimit + LVBState::overhead);
    need(a.component->getState(&failed_save) != kResultOk, "failed capture cannot replace confirmed snapshot");
    auto result = a.controller->setParamNormalized(0x41503601, 1.);
    need(success ? result == kResultOk : result != kResultOk, "explicit recovery result");
    waitFrames(second, sibling_before + 8192);
    if (success) {
      need(a.controller->getParamNormalized(0) == .25 && handler->cached == .25, "restored controller and host cache");
      const auto ready = Clock::now() + std::chrono::seconds(5);
      while (first.recovered_samples < 8192) { need(Clock::now() < ready, "recovered processing deadline"); std::this_thread::sleep_for(std::chrono::milliseconds(1)); }
      auto saved = snapshot(*a.component, *a.controller, .25, "recovered_complete_state");
      need(saved == loadBytes("instance-a.state"), "non-gain complete payload round trip");
    } else {
      need(first.recovered_samples == 0, "failed recovery never resumes defaults");
      need(a.component->getState(&failed_save) != kResultOk, "failed recovery cannot masquerade as save");
    }
  } catch (...) { error = std::current_exception(); }
  first.stop = true; second.stop = true;
  ta.join(); tb.join();
  if (first.error) std::rethrow_exception(first.error);
  if (second.error) std::rethrow_exception(second.error);
  if (error) std::rethrow_exception(error);
  ok(controller->disconnect(component), "controller disconnect");
  ok(component->disconnect(controller), "processor disconnect");
  if (success) ok(a.component->setActive(false), "recovered deactivate");
  auto ended = a.component->terminate();
  need(success ? ended == kResultOk : ended != kResultOk, "recovery terminate");
  ok(b.component->setActive(false), "survivor deactivate");
  ok(b.component->terminate(), "survivor terminate");
  ok(a.controller->setComponentHandler(nullptr), "release handler");
  ok(a.controller->terminate(), "recovery controller terminate");
  ok(b.controller->terminate(), "survivor controller terminate");
  std::cout << "{\"event\":\"ap6_recovery\",\"case\":\"" << scenario
            << "\",\"recovered_samples\":" << first.recovered_samples.load()
            << ",\"sibling_samples\":" << second.samples << ",\"maximum_error\":0,\"callback_effects\":0}" << std::endl;
}
