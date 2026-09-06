// AP4 cases extend the existing SDK-loaded consumer. Peer substitution is local
// test instrumentation only; live output always comes from Windows AGain.
// Host fragmentation is independent of the proxy's memory stream. It forwards
// actual SDK reads/writes with a deliberately small accepted extent.
class FragmentedStream final : public IBStream {
public:
  LVBState::Stream storage;
  bool fail_write = false, fail_eof = false;
  explicit FragmentedStream(std::vector<uint8_t> bytes = {})
      : storage(std::move(bytes), LVBState::payloadLimit + LVBState::overhead) {
  }
  tresult PLUGIN_API queryInterface(const TUID id, void **out) override {
    return storage.queryInterface(id, out);
  }
  uint32 PLUGIN_API addRef() override { return storage.addRef(); }
  uint32 PLUGIN_API release() override { return storage.release(); }
  tresult PLUGIN_API read(void *p, int32 n, int32 *count) override {
    if (fail_eof && storage.position == storage.bytes.size()) {
      if (count)
        *count = 0;
      return kInternalError;
    }
    return storage.read(p, std::min(n, 3), count);
  }
  tresult PLUGIN_API write(void *p, int32 n, int32 *count) override {
    if (fail_write) {
      if (count)
        *count = 0;
      return kResultFalse;
    }
    return storage.write(p, std::min(n, 5), count);
  }
  tresult PLUGIN_API seek(int64 n, int32 m, int64 *p) override {
    return storage.seek(n, m, p);
  }
  tresult PLUGIN_API tell(int64 *p) override { return storage.tell(p); }
};

std::string stateHex(const std::vector<uint8_t> &bytes) {
  std::string out;
  const char *h = "0123456789abcdef";
  for (auto b : bytes) {
    out += h[b >> 4];
    out += h[b & 15];
  }
  return out;
}
std::vector<uint8_t> snapshot(IComponent &c, IEditController &controller,
                              double expected, const char *label) {
  LVBState::Stream s({}, LVBState::payloadLimit + LVBState::overhead);
  ok(c.getState(&s), "component getState");
  need(s.quiescent(), "host stream references");
  s.position = 0;
  ok(controller.setComponentState(&s), "controller component state");
  need(controller.getParamNormalized(0) == expected,
       "restored controller gain");
  std::cout << "{\"event\":\"ap4_host_state\",\"case\":\"" << label
            << "\",\"envelope_hex\":\"" << stateHex(s.bytes)
            << "\",\"controller_gain\":" << expected << "}" << std::endl;
  return s.bytes;
}
void saveBytes(const std::vector<uint8_t> &bytes, const char *name) {
  const char *directory = std::getenv("LVB_AP4_STATE_STORE");
  need(directory, "private state store");
  std::ofstream f(std::string(directory) + "/" + name,
                  std::ios::binary | std::ios::trunc);
  f.write(reinterpret_cast<const char *>(bytes.data()),
          static_cast<std::streamsize>(bytes.size()));
  f.close();
  need(bool(f), "host state persistence");
}
std::vector<uint8_t> loadBytes(const char *name) {
  const char *directory = std::getenv("LVB_AP4_STATE_STORE");
  need(directory, "private state store");
  std::ifstream f(std::string(directory) + "/" + name,
                  std::ios::binary | std::ios::ate);
  need(bool(f), "host saved stream open");
  auto n = f.tellg();
  need(n > 0 &&
           n <= std::streamoff(LVBState::payloadLimit + LVBState::overhead),
       "host saved stream bound");
  std::vector<uint8_t> bytes(static_cast<size_t>(n));
  f.seekg(0);
  f.read(reinterpret_cast<char *>(bytes.data()), n);
  need(bool(f), "host saved stream read");
  return bytes;
}
void restore(IComponent &c, IEditController &controller,
             const std::vector<uint8_t> &bytes, double gain) {
  LVBState::Stream stream(bytes, LVBState::payloadLimit + LVBState::overhead);
  ok(c.setState(&stream), "component restore");
  stream.position = 0;
  ok(controller.setComponentState(&stream),
     "processor-first controller restore");
  need(controller.getParamNormalized(0) == gain, "controller restored value");
  need(snapshot(c, controller, gain, "restore_readback") == bytes,
       "complete component state readback");
}
// Explicit source-backed synthetic fixtures. These are inputs, never results.
std::vector<uint8_t> fixture(const std::vector<uint8_t> &base,
                             const char *hex) {
  auto bytes = base;
  need(bytes.size() == 116, "reference envelope size");
  for (size_t i = 0; i < 44; ++i) {
    unsigned v = 0;
    need(std::sscanf(hex + i * 2, "%2x", &v) == 1, "fixture hex");
    bytes[72 + i] = static_cast<uint8_t>(v);
  }
  return bytes;
}
struct StateProgress {
  std::atomic<uint64_t> frames{0};
  std::atomic<bool> mute{false}, flushed{false}, done{false};
};
void stateAudio(IAudioProcessor &p, double gain, double factor,
                bool initial_update, int frames, StateProgress &progress) {
  Block b;
  std::array<std::array<double, 4096>, 2> expected{};
  std::vector<uint64_t> durations;
  durations.reserve(static_cast<size_t>(frames / 256 + 8));
  uint64_t seed = 0;
  std::ifstream random("/dev/urandom", std::ios::binary);
  random.read(reinterpret_cast<char *>(&seed), 8);
  need(bool(random), "fresh input seed");
  uint64_t x = seed, position = 0, samples = 0,
           input_hash = 14695981039346656037ULL, output_hash = input_hash,
           overruns = 0;
  const double initial_factor = factor;
  uint64_t mute_position = UINT64_MAX;
  double max_error = 0.;
  bool flushed = false;
  ok(callback([&] { return p.setProcessing(true); }), "state audio start");
  auto start = Clock::now();
  for (unsigned block = 0; position < static_cast<uint64_t>(frames + 1024);
       ++block) {
    if (progress.mute.load(std::memory_order_acquire) && !flushed) {
      ProcessData d;
      d.processMode = kRealtime;
      d.symbolicSampleSize = kSample32;
      ParameterChanges q(1);
      int32 i = 0, j = 0;
      q.addParameterData(0, i)->addPoint(0, 0., j);
      d.inputParameterChanges = &q;
      ok(callback([&] { return p.process(d); }), "parameter-only mute");
      mute_position = position;
      gain = factor = 0.;
      flushed = true;
      progress.flushed.store(true, std::memory_order_release);
    }
    int n = std::min(256, frames + 1024 - int(position));
    bool tail = position >= static_cast<uint64_t>(frames),
         inplace = block % 7 == 3;
    auto flags = tail || block % 17 == 8 ? 3u : block % 13 == 4 ? 1u : 0u;
    b.prepare(n, gain, initial_update && block == 0, inplace, flags);
    for (int i = 0; i < n; ++i)
      for (int ch = 0; ch < 2; ++ch) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        float v = (flags & (1u << ch)) ? 0.f : float(int(x % 65) - 32) / 256.f;
        b.in[ch][i] = v;
        expected[ch][(position + i) % 4096] = double(v) * factor;
        hash(input_hash, v);
      }
    auto before = b.input;
    std::this_thread::sleep_until(
        start + std::chrono::nanoseconds(position * 1000000000ULL / 48000));
    auto t = Clock::now();
    auto result = callback([&] { return p.process(b.data); });
    auto duration = uint64_t(
        std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() - t)
            .count());
    durations.push_back(duration);
    overruns += duration > uint64_t(n) * 1000000000ULL / 48000;
    ok(result, "state restored audio process");
    need((b.ob.silenceFlags & ~uint64_t(3)) == 0, "state audio silence bits");
    for (int ch = 0; ch < 2; ++ch) {
      if (!inplace)
        need(b.input[ch] == before[ch], "state input changed");
      const auto &plane = inplace ? b.input[ch] : b.output[ch];
      need(std::bit_cast<uint32_t>(plane[0]) == 0x4b123456 &&
               std::bit_cast<uint32_t>(plane[n + 1]) == 0x4b123456,
           "state output guards");
      for (int i = 0; i < n; ++i) {
        double want = position + i < 1024
                          ? 0.
                          : expected[ch][(position + i - 1024) % 4096];
        double actual = b.out[ch][i];
        double error = std::abs(actual - want);
        max_error = std::max(max_error, error);
        need(std::isfinite(actual) && error == 0., "restored sample mismatch");
        need(!(b.ob.silenceFlags & (uint64_t(1) << ch)) || actual == 0.,
             "state false silence claim");
        hash(output_hash, float(actual));
        ++samples;
      }
    }
    position += uint64_t(n);
    progress.frames.store(position, std::memory_order_release);
  }
  ok(callback([&] { return p.setProcessing(false); }), "state audio stop");
  std::sort(durations.begin(), durations.end());
  need(overruns == 0, "state callback overrun");
  std::cout << "{\"event\":\"ap4_host_compared\",\"samples\":" << samples
            << ",\"max_error\":" << max_error << ",\"seed\":" << seed
            << ",\"input_fnv1a64\":" << input_hash
            << ",\"output_fnv1a64\":" << output_hash
            << ",\"initial_gain_sent\":" << (initial_update ? "true" : "false")
            << ",\"initial_factor\":" << initial_factor
            << ",\"mute_position\":" << mute_position
            << ",\"restored_factor\":" << factor
            << ",\"parameter_only_mute\":" << (flushed ? "true" : "false")
            << ",\"callback_max_ns\":" << durations.back()
            << ",\"callback_p99_ns\":"
            << durations[(durations.size() - 1) * 99 / 100]
            << ",\"callback_effects\":0,\"callback_overruns\":" << overruns
            << ",\"active_frames\":" << frames << ",\"latency_samples\":1024}"
            << std::endl;
}
void stateCases(IComponent &c, IAudioProcessor &p, IEditController &controller,
                const std::string &scenario) {
  stage = "state_initial";
  auto initial = snapshot(c, controller, 1., "fresh_default");
  const bool capturing =
      scenario == "state-capture" || scenario == "state-local";
  auto saved = capturing ? initial
                         : loadBytes(scenario == "state-mute" ? "mute.state"
                                                              : "gain.state");
  double gain = capturing ? .25 : scenario == "state-mute" ? 0. : .25;
  if (!capturing)
    restore(c, controller, saved, gain);
  ProcessSetup setup{kRealtime, kSample32, 256, 48000.};
  ok(p.setupProcessing(setup), "state setup");
  ok(c.setActive(true), "state activate");
  StateProgress progress;
  std::exception_ptr failure;
  std::thread audio([&] {
    try {
      stateAudio(p, gain, gain, capturing,
                 scenario == "state-capture" ? 1440000 : 48000, progress);
    } catch (...) {
      failure = std::current_exception();
    }
    progress.done.store(true, std::memory_order_release);
  });
  auto wait = [&](auto test) {
    auto end = Clock::now() + std::chrono::seconds(35);
    while (!test() && !progress.done.load(std::memory_order_acquire) &&
           Clock::now() < end)
      std::this_thread::sleep_for(std::chrono::milliseconds(1));
    need(test(), "state audio observation unavailable");
  };
  try {
    if (capturing) {
      wait([&] { return progress.frames.load() >= 4096; });
      saved = snapshot(c, controller, .25, "overlapping_save");
      LVBState::Stream live_restore(saved, LVBState::payloadLimit +
                                               LVBState::overhead);
      need(c.setState(&live_restore) != kResultOk,
           "live state mutation accepted");
      saveBytes(saved, "gain.state");
      progress.mute.store(true, std::memory_order_release);
      wait([&] { return progress.flushed.load(); });
      saveBytes(snapshot(c, controller, 0., "parameter_only_mute"),
                "mute.state");
    }
  } catch (...) {
    if (audio.joinable())
      audio.join();
    throw;
  }
  audio.join();
  if (failure) {
    if (scenario == "state-corrupt-output") {
      LVBState::Stream unsaved;
      need(c.getState(&unsaved) != kResultOk,
           "failed callback instance allowed a state snapshot");
      need(unsaved.bytes.empty(), "failed snapshot wrote host bytes");
    }
    std::rethrow_exception(failure);
  }
  stage = "state_stopped";
  if (capturing) {
    // A failed host write cannot be published as a successful snapshot.
    FragmentedStream short_write;
    ok(c.getState(&short_write), "short host writes");
    FragmentedStream short_read(short_write.storage.bytes);
    ok(c.setState(&short_read), "short host reads");
    short_write.fail_write = true;
    need(c.getState(&short_write) != kResultOk, "failed host write accepted");
    LVBState::Stream arithmetic;
    need(arithmetic.seek(std::numeric_limits<int64>::max(),
                         IBStream::kIBSeekCur, nullptr) != kResultOk,
         "seek overflow accepted");
    need(arithmetic.seek(-1, IBStream::kIBSeekSet, nullptr) != kResultOk,
         "negative seek accepted");
    // Malformed input must not change the current good instance.
    auto good = snapshot(c, controller, 0., "before_invalid");
    for (size_t index : std::array<size_t, 6>{0, 8, 16, 32, 64, 72}) {
      auto bad = good;
      bad[index] ^= 1;
      LVBState::Stream s(bad, LVBState::payloadLimit + LVBState::overhead);
      need(c.setState(&s) != kResultOk, "malformed state accepted");
    }
    for (size_t length = 0; length < good.size(); ++length) {
      LVBState::Stream truncated(
          std::vector<uint8_t>(good.begin(), good.begin() + length),
          LVBState::payloadLimit + LVBState::overhead);
      need(c.setState(&truncated) != kResultOk, "truncated state accepted");
    }
    FragmentedStream failed_eof(good);
    failed_eof.fail_eof = true;
    need(c.setState(&failed_eof) != kResultOk, "stream error accepted as EOF");
    failed_eof.storage.position = 0;
    need(controller.setComponentState(&failed_eof) != kResultOk,
         "controller stream error accepted as EOF");
    auto oversized = good;
    oversized[64] = 1;
    oversized[66] = 0x10;
    LVBState::Stream too_big(oversized,
                             LVBState::payloadLimit + LVBState::overhead);
    need(c.setState(&too_big) != kResultOk, "oversized state accepted");
    need(snapshot(c, controller, 0., "after_invalid") == good,
         "invalid state mutated component");
    for (auto pair : std::array<std::pair<const char *, double>, 2>{
             std::pair{"6d800fadef24ab68bfe62e8ee1f381618824d4b3d5a19569ca3e1f2"
                       "4d75c0de20000003f0000003e00000000",
                       .375},
             std::pair{"ab684897cfa726c7e07eaa1c2be127723a65aa8aefd4f6f87fd48ce"
                       "c0aeec5220000803e0000003e01000000",
                       1.}}) {
      auto seeded = fixture(initial, pair.first);
      double display = pair.second == 1. ? .25 : .5;
      restore(c, controller, seeded, display);
      StateProgress done;
      std::exception_ptr error;
      std::thread t([&] {
        try {
          stateAudio(p, display, pair.second, false, 8192, done);
        } catch (...) {
          error = std::current_exception();
        }
      });
      t.join();
      if (error)
        std::rethrow_exception(error);
      need(snapshot(c, controller, display, "non_gain_resave") == seeded,
           "non-gain state lost");
    }
  } else {
    // A deliberate later edit, only after restored output has already passed.
    StateProgress done;
    std::exception_ptr error;
    std::thread t([&] {
      try {
        stateAudio(p, .5, .5, true, 8192, done);
      } catch (...) {
        error = std::current_exception();
      }
    });
    t.join();
    if (error)
      std::rethrow_exception(error);
    snapshot(c, controller, .5, "later_edit");
  }
  ok(c.setActive(false), "state deactivate");
}
