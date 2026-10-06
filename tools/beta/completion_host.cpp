// Independent ordinary-SDK completion consumer. No product transport, result
// decoder, state decoder, scheduling mutator, or Windows producer is linked.
#include "../../vst-state/stream.h"
#include "sdk_buffers.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/module.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <bit>
#include <cerrno>
#include <charconv>
#include <chrono>
#include <cmath>
#include <cstring>
#include <cstdlib>
#include <dlfcn.h>
#include <fcntl.h>
#include <fstream>
#include <functional>
#include <iostream>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <sys/stat.h>
#include <thread>
#include <unistd.h>
using namespace Steinberg;
using namespace Steinberg::Vst;
using namespace CompletionSDK;
using Clock = std::chrono::steady_clock;
namespace {
constexpr int maximum = 1024, vendorLatency = 13;
constexpr size_t maximumCallbacks = 4096;
constexpr size_t sustainedCallbacks = 4000;
constexpr double tau = 6.2831853071795864769;
const char* stage = "arguments";
void need(bool value, const char* why) { if (!value) throw std::runtime_error(why); }
void ok(tresult result, const char* why) { need(result == kResultOk, why); }
uint64_t nowNs() { return uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now().time_since_epoch()).count()); }
int integer(const char* text, int low, int high) {
    const std::string_view input = text; int value = 0;
    const auto [end, error] = std::from_chars(input.data(), input.data()+input.size(), value);
    need(error == std::errc{} && end == input.data()+input.size() && value >= low && value <= high,
         "strict decimal argument bound"); return value;
}
bool classId(std::string_view value) {
    return value.size() == 32 && std::all_of(value.begin(), value.end(), [](char ch) {
        return (ch >= '0' && ch <= '9') || (ch >= 'A' && ch <= 'F');
    });
}
struct Options {
    const char* bundle = nullptr;
    bool instrument = false;
    std::string_view scenario;
    const char* prefix = nullptr;
    int block = 0, rate = 0, delay = 0;
    const char* processor = nullptr;
    const char* controller = nullptr;
    const char* capturePrefix = nullptr;
};
Options options(int argc, char** argv) {
    need(argc == 10 || argc == 11, "usage: completion-host BUNDLE instrument|effect SCENARIO STATE_PREFIX M SAMPLE_RATE D NATIVE_PROCESSOR_ID NATIVE_CONTROLLER_ID [CAPTURE_PREFIX]");
    Options o; o.bundle = argv[1]; o.instrument = std::string_view(argv[2]) == "instrument";
    need(o.instrument || std::string_view(argv[2]) == "effect", "exact fixture role");
    o.scenario = argv[3];
    need(o.scenario == "matrix" || o.scenario == "modes" || o.scenario == "sustained" || o.scenario == "offline"
        || o.scenario == "slow-offline" || o.scenario == "offline-failure"
        || o.scenario == "offline-timeout" || o.scenario == "abrupt-offline"
        || o.scenario == "state-record" || o.scenario == "state-recall"
        || o.scenario == "held-factory-refusal", "exact scenario");
    o.prefix = argv[4]; need(*o.prefix != '\0', "state output prefix");
    o.block = integer(argv[5], 1, maximum); o.rate = integer(argv[6], 44100, 192000);
    need(o.rate == 44100 || o.rate == 48000 || o.rate == 88200 || o.rate == 96000 || o.rate == 192000,
         "declared supported sample rate");
    o.delay = integer(argv[7], 0, 1024);
    need(o.delay == 0 || o.delay == 256 || o.delay == 512 || o.delay == 1024, "declared delivery D");
    if (o.scenario == "sustained") {
        need(o.block == 64 || o.block == 128 || o.block == 256, "sustained actual N must be 64, 128 or 256");
        need(o.delay == 0 || o.delay == 256, "sustained delivery D must be SameCallback zero or Buffered 256");
    }
    o.processor = argv[8]; o.controller = argv[9];
    need(classId(o.processor) && classId(o.controller) && std::string_view(o.processor) != o.controller,
         "exact distinct uppercase native class identities");
    need((o.scenario == "state-recall") == (argc == 11), "external recall requires one distinct capture prefix");
    o.capturePrefix = argc == 11 ? argv[10] : o.prefix;
    need(*o.capturePrefix != '\0' && (argc != 11 || std::string_view(o.capturePrefix) != o.prefix),
         "distinct nonempty capture prefix");
    return o;
}
using Mark = void(*)(); using Count = uint64_t(*)();
Mark auditBegin = nullptr; Count auditEnd = nullptr, auditLocalWakes = nullptr;
struct CallbackResult { tresult result{}; uint64_t begin{}, end{}, effects{}, localWakes{}; };
template<class Operation> CallbackResult call(Operation operation,std::exception_ptr* failure=nullptr) {
    CallbackResult result; result.begin = nowNs(); auditBegin();
    try { result.result = operation(); }
    catch (...) {
        result.effects = auditEnd(); result.localWakes = auditLocalWakes?auditLocalWakes():0; result.end = nowNs();
        if (!failure) throw;
        *failure = std::current_exception(); result.result = kInternalError; return result;
    }
    result.effects = auditEnd(); result.localWakes = auditLocalWakes?auditLocalWakes():0;
    result.end = nowNs(); return result;
}
struct Lifecycle {
    struct Step {
        bool attempted = false;
        tresult result = kResultOk;
        uint64_t effects = 0;
        std::exception_ptr exception;
    };
    IPtr<IComponent> component;
    IPtr<IEditController> controller;
    FUnknownPtr<IAudioProcessor> processor;
    FUnknownPtr<IConnectionPoint> cp, cc;
    bool componentInitialized = false, controllerInitialized = false;
    bool componentConnected = false, controllerConnected = false, active = false, processing = false;
    bool finished = false, clean = true;
    Step stopped, deactivated, componentDisconnected, controllerDisconnected, componentTerminated, controllerTerminated;
    const char* failedStep = "none";
    std::function<void(bool)> unload;
    ~Lifecycle() { finish(); }
    template<class Operation> void perform(bool& completed, Step& record, const char* name, Operation operation) noexcept {
        if (!completed || !clean) return;
        record = {}; record.attempted = true;
        try { record.result = operation(); }
        catch (...) { record.result = kInternalError; record.exception = std::current_exception(); }
        if (record.result == kResultOk && record.effects == 0 && !record.exception) completed = false;
        else { clean = false; failedStep = name; }
    }
    void stop() {
        if (!processing) return;
        need(clean, "previous lifecycle failure retained; no cleanup retry");
        perform(processing, stopped, "processing_stop", [&] {
            const auto result = call([&]{ return processor->setProcessing(false); });
            stopped.effects = result.effects; return result.result;
        });
        if (stopped.exception) std::rethrow_exception(stopped.exception);
        need(stopped.effects == 0, "stop callback effects"); ok(stopped.result, "processing stop");
    }
    void deactivate() {
        if (!active) return;
        need(clean, "previous lifecycle failure retained; no cleanup retry");
        perform(active, deactivated, "deactivate", [&]{return component->setActive(false);});
        if (deactivated.exception) std::rethrow_exception(deactivated.exception);
        ok(deactivated.result, "inactive transition");
    }
    bool finish() noexcept {
        if (finished) return clean;
        finished = true;
        perform(processing, stopped, "processing_stop", [&] {
            const auto result = call([&]{return processor->setProcessing(false);});
            stopped.effects = result.effects; return result.result;
        });
        perform(active, deactivated, "deactivate", [&]{return component->setActive(false);});
        perform(controllerConnected, controllerDisconnected, "controller_disconnect", [&]{return cc->disconnect(cp);});
        perform(componentConnected, componentDisconnected, "component_disconnect", [&]{return cp->disconnect(cc);});
        perform(componentInitialized, componentTerminated, "component_terminate", [&]{return component->terminate();});
        perform(controllerInitialized, controllerTerminated, "controller_terminate", [&]{return controller->terminate();});
        if (clean) { cc = nullptr; cp = nullptr; processor = nullptr; controller = nullptr; component = nullptr; }
        if (clean && unload) unload(true);
        std::cout << "{\"event\":\"host_retired\",\"processing_stop_result\":" << stopped.result
                  << ",\"processing_stop_attempted\":" << (stopped.attempted?"true":"false")
                  << ",\"deactivate_result\":" << deactivated.result << ",\"component_disconnect_result\":" << componentDisconnected.result
                  << ",\"controller_disconnect_result\":" << controllerDisconnected.result << ",\"terminate_result\":" << componentTerminated.result
                  << ",\"controller_terminate_result\":" << controllerTerminated.result << ",\"failed_step\":\"" << failedStep
                  << "\",\"processing_still_owned\":" << (processing?"true":"false") << ",\"active_still_owned\":" << (active?"true":"false")
                  << ",\"deactivate_attempted\":" << (deactivated.attempted?"true":"false")
                  << ",\"component_disconnect_attempted\":" << (componentDisconnected.attempted?"true":"false")
                  << ",\"controller_disconnect_attempted\":" << (controllerDisconnected.attempted?"true":"false")
                  << ",\"component_terminate_attempted\":" << (componentTerminated.attempted?"true":"false")
                  << ",\"controller_terminate_attempted\":" << (controllerTerminated.attempted?"true":"false")
                  << ",\"failed_lifetime_retained\":" << (clean?"false":"true") << ",\"module_unloaded\":" << (clean?"true":"false") << "}" << std::endl;
        return clean;
    }
};
struct Lifetime {
    VST3::Hosting::Module::Ptr module;
    IPtr<HostApplication> host;
    Lifecycle life;
};
struct LifetimeDeleter {
    void operator()(Lifetime* owner) const noexcept {
        if (owner->life.finish()) delete owner;
        // A refused SDK step never authorizes any later interface release,
        // host destruction or module unload. Keep the entire exact owner until
        // process exit, including early setup unwinding, so the outer catch
        // can still report the original error before the harness exits.
    }
};
using LifetimePtr = std::unique_ptr<Lifetime,LifetimeDeleter>;
LifetimePtr lifetime() {
    LifetimePtr owner(new Lifetime);
    owner->life.unload = [value=owner.get()](bool clean) { if (clean) { value->module.reset(); value->host = nullptr; } };
    return owner;
}
void point(Parameters& values, ParamID id, double value) {
    int32 index = 0; auto* queue = values.addParameterData(id, index);
    need(queue && queue->addPoint(0, value, index) == kResultOk, "bounded input parameter point");
}
struct Buffers {
    // A canary on each side of every borrowed audio lane catches writes past N.
    std::array<std::array<float, maximum+2>, 2> input{};
    std::array<std::array<float, maximum+2>, 4> output{};
    float* in[2]{input[0].data()+1, input[1].data()+1};
    float* out[4]{output[0].data()+1, output[1].data()+1, output[2].data()+1, output[3].data()+1};
    AudioBusBuffers inputs{}, outputs[2]{};
    Parameters sent, returned;
    Events notes, results;
    ProcessData data{};
    explicit Buffers(bool instrument) {
        inputs.numChannels = 2; inputs.channelBuffers32 = in;
        for (int bus = 0; bus < 2; ++bus) { outputs[bus].numChannels = 2; outputs[bus].channelBuffers32 = out+2*bus; }
        data.symbolicSampleSize = kSample32; data.inputParameterChanges = &sent; data.outputParameterChanges = &returned;
        data.inputEvents = &notes; data.outputEvents = &results;
        prepare(0, kOffline, instrument, false);
    }
    void prepare(int n, int mode, bool instrument, bool inPlace) {
        sent.clear(); returned.clear(); notes.clear(); results.clear();
        for (auto& lane : input) lane.fill(-999.f);
        for (auto& lane : output) lane.fill(-999.f);
        for (int ch = 0; ch < 2; ++ch) out[ch] = inPlace && !instrument ? in[ch] : output[size_t(ch)].data()+1;
        data.numSamples = n; data.processMode = mode;
        data.numInputs = n && !instrument ? 1 : 0; data.numOutputs = n ? 2 : 0;
        data.inputs = data.numInputs ? &inputs : nullptr; data.outputs = n ? outputs : nullptr;
        inputs.silenceFlags = 0;
        for (auto& bus : outputs) bus.silenceFlags = 0;
    }
    void guards(int n) {
        for (const auto& lane : input) need(lane[0] == -999.f && lane[size_t(n+1)] == -999.f, "input borrowed-storage canary");
        for (const auto& lane : output) need(lane[0] == -999.f && lane[size_t(n+1)] == -999.f, "output borrowed-storage canary");
        need(sent.quiescent() && returned.quiescent() && notes.quiescent() && results.quiescent(), "callback retained a lexical SDK sink");
    }
};
// Host sample-position oracle, derived from the fixture's declared equations,
// latency and ordinary SDK event/point offsets. No packet decoder is used.
struct Oracle {
    struct ExpectedEvent { uint64_t due{}; Event value{}; };
    struct ExpectedPoint { uint64_t due{}; ParamID id{}; double value{}; bool flush{}; };
    std::array<std::array<float,4>,4096> audio{};
    std::array<ExpectedEvent,2048> events{};
    std::array<ExpectedPoint,4096> points{};
    size_t eventCount = 0, pointCount = 0;
    uint64_t position = 0, mismatches = 0, nonfinite = 0, nonzero = 0, compared = 0;
    uint64_t returnedEvents = 0, returnedPoints = 0;
    double maxError = 0., gain = .625, colour = .125, phase = 0., increment = 0.;
    bool voice = false, instrument = false;
    int delay = 0;
    void reset(const Options& o, int rate) {
        audio = {}; eventCount = pointCount = 0; position = 0; phase = 0.; voice = false;
        instrument = o.instrument; delay = o.delay; increment = tau*440./double(rate);
    }
    template<class Row, size_t N> static void insert(std::array<Row,N>& rows, size_t& size, Row row) {
        need(size < N, "preallocated oracle bound"); size_t at = size;
        while (at && rows[at-1].due > row.due) { rows[at] = rows[at-1]; --at; }
        rows[at] = row; ++size;
    }
    void predict(Buffers& b, int mode, bool first, bool final, bool tail) {
        const int n = b.data.numSamples;
        if (first && n) { Event e{}; e.type = Event::kNoteOnEvent; e.noteOn = {2,69,0.f,.8f,0,42}; ok(b.notes.addEvent(e), "first input event"); }
        if (final && n) { Event e{}; e.type = Event::kNoteOffEvent; e.sampleOffset = n-1; e.noteOff = {2,69,0.f,42,0.f}; ok(b.notes.addEvent(e), "final input event"); }
        for (int i = 0; i < n; ++i) {
            if (first && !i) { voice = true; phase = 0.; }
            if (final && i == n-1) voice = false;
            const double input = tail ? 0. : double(int((position+uint64_t(i))%257)-128)/512.;
            b.in[0][i] = float(input); b.in[1][i] = float(-input*.5);
            const double signal = voice ? double(.8f)*(std::sin(phase)+colour*.5*std::sin(phase*2.)) : 0.;
            if (voice) phase = std::fmod(phase+increment,tau);
            const float left = float(gain*(instrument?signal/16.:double(b.in[0][i])*(.5+colour)));
            const float right = instrument ? left : float(gain*double(b.in[1][i])*(.5+colour));
            audio[size_t((position+uint64_t(i)+uint64_t(delay+vendorLatency))%audio.size())]
                = {left,right,left*.5f,right*-.5f};
        }
        const uint64_t resultDelay = n ? uint64_t(delay) : 0;
        for (int i = 0; i < b.notes.getEventCount(); ++i) {
            Event e{}; ok(b.notes.getEvent(i,e), "input note read");
            insert(events,eventCount,ExpectedEvent{position+uint64_t(e.sampleOffset)+resultDelay,e});
        }
        Event cc{}; cc.type = Event::kLegacyMIDICCOutEvent; cc.midiCCOut = {74,2,int8(mode),0};
        insert(events,eventCount,ExpectedEvent{position+resultDelay,cc});
        insert(points,pointCount,ExpectedPoint{position+resultDelay,32,double(mode)/2.,n==0});
        insert(points,pointCount,ExpectedPoint{position+uint64_t(std::max(0,n-1))+resultDelay,0,gain,n==0});
    }
    bool due(uint64_t at, bool flush, int n) const { return flush || (n ? at < position+uint64_t(n) : at <= position); }
    template<class Row,size_t N> static void remove(std::array<Row,N>& rows, size_t& size, size_t at) {
        for (size_t i = at+1; i < size; ++i) rows[i-1] = rows[i];
        --size;
    }
    void compare(Buffers& b) {
        const int n = b.data.numSamples;
        for (int i = 0; i < n; ++i) {
            auto& expected = audio[size_t((position+uint64_t(i))%audio.size())];
            for (int lane = 0; lane < 4; ++lane) {
                const float actual = b.out[lane][i]; const double error = std::abs(double(actual)-expected[size_t(lane)]);
                ++compared; nonfinite += !std::isfinite(actual); nonzero += actual != 0.f;
                mismatches += error > 1e-7; maxError = std::max(maxError,error);
            }
            expected = {};
        }
        for (int i = 0; i < b.results.getEventCount(); ++i) {
            Event actual{}; ok(b.results.getEvent(i,actual), "returned event read");
            size_t at = 0; while (at < eventCount && !due(events[at].due,false,n)) ++at;
            need(at < eventCount, "unexpected returned event"); const auto& expected = events[at];
            need(actual.busIndex == 0 && actual.type == expected.value.type
                && actual.sampleOffset == int32(expected.due > position?expected.due-position:0), "event identity/position");
            if (actual.type == Event::kLegacyMIDICCOutEvent) need(actual.midiCCOut.controlNumber == 74
                && actual.midiCCOut.channel == 2 && actual.midiCCOut.value == expected.value.midiCCOut.value
                && actual.midiCCOut.value2 == 0, "actual callback mode event");
            else if (actual.type == Event::kNoteOnEvent) need(actual.noteOn.noteId == 42 && actual.noteOn.channel == 2
                && actual.noteOn.pitch == 69 && actual.noteOn.velocity == .8f, "exact first note");
            else need(actual.type == Event::kNoteOffEvent && actual.noteOff.noteId == 42
                && actual.noteOff.channel == 2 && actual.noteOff.pitch == 69, "exact final note");
            remove(events,eventCount,at); ++returnedEvents;
        }
        for (int q = 0; q < b.returned.getParameterCount(); ++q) {
            auto* queue = b.returned.getParameterData(q); need(queue != nullptr, "returned queue identity");
            for (int p = 0; p < queue->getPointCount(); ++p) {
                int32 offset = -1; double value = -1.; ok(queue->getPoint(p,offset,value), "returned point read");
                size_t at = 0; while (at < pointCount && (points[at].id != queue->getParameterId()
                    || !due(points[at].due,points[at].flush,n))) ++at;
                need(at < pointCount, "unexpected returned parameter point"); const auto expected = points[at];
                need(offset == (expected.flush?0:int32(expected.due > position?expected.due-position:0))
                    && value == expected.value, "exact mode/gain point and position");
                remove(points,pointCount,at); ++returnedPoints;
            }
        }
        for (size_t i = 0; i < eventCount; ++i) need(!due(events[i].due,false,n), "missing due event in this callback");
        for (size_t i = 0; i < pointCount; ++i) need(!due(points[i].due,points[i].flush,n), "missing due point in this callback");
        position += uint64_t(n); b.guards(n);
    }
};
struct Row {
    uint64_t begin{},end{},effects{},samplePosition{},localWakes{};
    int n{},mode{},result{}; bool inPlace{},threw{};
    size_t outputOffset{},outputCount{};
};
struct WholeCallbackStatistics {
    size_t callbacks{};
    uint64_t median{},p99{},maximum{},overruns{};
};
constexpr size_t sustainedRowCapacity(int block,int delay) {
    return 4+sustainedCallbacks+size_t((delay+vendorLatency+block-1)/block)+1;
}
static_assert(sustainedRowCapacity(64,256)<=maximumCallbacks,
              "4000 sustained callbacks and the longest selected tail require prepared storage");
WholeCallbackStatistics wholeCallbackStatistics(
    const std::array<Row,maximumCallbacks>& rows,size_t begin,size_t count,uint64_t allowance) {
    need(count && begin <= rows.size() && count <= rows.size()-begin,
         "bounded completed sustained callback window");
    std::array<uint64_t,maximumCallbacks> durations{};
    WholeCallbackStatistics result{};result.callbacks=count;
    for(size_t i=0;i<count;++i) {
        const auto& row=rows[begin+i];need(row.end>=row.begin,"monotonic whole callback interval");
        const auto duration=row.end-row.begin;durations[i]=duration;result.overruns+=duration>allowance;
    }
    std::sort(durations.begin(),durations.begin()+count);
    result.median=durations[count/2];
    result.p99=durations[(count-1)*99/100];
    result.maximum=durations[count-1];
    return result;
}
struct OutputCapture {
    static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559
        && std::endian::native == std::endian::little, "retained output requires IEEE float32 little endian");
    static constexpr size_t lanes = 4, capacity = maximumCallbacks*size_t(maximum)*lanes;
    std::unique_ptr<float[]> values{new float[capacity]{}};
    size_t count = 0, writtenBytes = 0;
    int fd = -1;
    bool written = false;
    OutputCapture() {
        // Value initialization alone can leave zero pages lazy. Commit every
        // 4-KiB page before activation; capture then only copies into this owner.
        volatile float* pages = values.get();
        for (size_t i = 0; i < capacity; i += 1024) pages[i] = 0.f;
        pages[capacity-1] = 0.f;
    }
    ~OutputCapture() { if (fd >= 0) ::close(fd); }
    void prepare(const char* prefix) {
        need(fd < 0 && !written, "one prepared output file per run");
        const std::string path = std::string(prefix)+".output.f32le";
        fd = ::open(path.c_str(),O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC|O_NOFOLLOW,0600);
        need(fd >= 0, "private unique run output prefix required");
    }
    void copy(const Buffers& buffers,int n,Row& row) {
        need(n >= 0 && n <= maximum, "bounded actual output frames");
        const size_t length = size_t(n)*lanes;
        need(length <= capacity-count, "prepared whole output capacity");
        row.outputOffset = count; row.outputCount = length;
        for (int frame = 0; frame < n; ++frame)
            for (size_t lane = 0; lane < lanes; ++lane) values[count++] = buffers.out[lane][frame];
    }
    void write() {
        need(fd >= 0 && !written, "prepared private output file required");
        const auto* bytes = reinterpret_cast<const char*>(values.get());
        const size_t length = count*sizeof(float);
        while (writtenBytes < length) {
            const auto result = ::write(fd,bytes+writtenBytes,length-writtenBytes);
            if (result < 0 && errno == EINTR) continue;
            need(result > 0, "complete retained actual output write");
            writtenBytes += size_t(result);
        }
        const int closed = ::close(fd); fd = -1;
        need(closed == 0, "retained actual output close"); written = true;
    }
};
bool captureOverlaps(uint64_t begin,uint64_t end,const Row& audio) {
    return begin < audio.begin && audio.begin < end && end < audio.end
        && end-begin < 10000000000ULL && audio.end-begin > 10000000000ULL;
}
struct Exercise {
    Lifecycle& life; const Options& o;
    Buffers buffers;
    Oracle oracle;
    OutputCapture output;
    std::array<Row,maximumCallbacks> rows{};
    size_t rowCount = 0;
    size_t sustainedBegin = 0, sustainedCount = 0;
    bool sustainedAttempted = false;
    uint64_t callbackMax = 0, overruns = 0, rejected = 0, effects = 0, localWakes = 0;
    bool pendingAudio = false;
    Exercise(Lifecycle& owner,const Options& option):life(owner),o(option),buffers(option.instrument) {}
    void configure(int mode,int block,int rate) {
        life.stop();
        life.deactivate();
        ProcessSetup setup{mode,kSample32,block,double(rate)};
        ok(life.processor->setupProcessing(setup), "prepared processing setup");
        ok(life.component->setActive(true), "component activation"); life.active = true;
        need(life.processor->getLatencySamples() == uint32(o.delay+vendorLatency), "exact D plus vendor L SDK latency");
        auto result = call([&]{return life.processor->setProcessing(true);});
        if (result.result == kResultOk) life.processing = true;
        need(result.effects == 0, "start callback effects"); ok(result.result, "processing start");
        oracle.reset(o,rate); pendingAudio = false;
    }
    CallbackResult process(int n,int mode,int rate,bool first=false,bool final=false,bool tail=false,
                           double gain=-1.,double operation=-1.,bool expectedFailure=false) {
        need(rowCount < rows.size(), "bounded callback telemetry");
        need(n >= 0 && n <= maximum, "bounded actual process frames");
        const bool inPlace = !o.instrument && rowCount%2;
        buffers.prepare(n,mode,o.instrument,inPlace);
        if (gain >= 0.) { point(buffers.sent,0,gain); oracle.gain = gain; }
        if (operation >= 0.) point(buffers.sent,31,operation);
        oracle.predict(buffers,mode,first,final,tail);
        const auto samplePosition = oracle.position;
        std::exception_ptr failure;
        auto result = call([&]{return life.processor->process(buffers.data);},&failure);
        auto& row = rows[rowCount++];
        row = {result.begin,result.end,result.effects,samplePosition,result.localWakes,n,mode,result.result,inPlace,bool(failure)};
        // The audit has ended. Keep actual borrowed output, including failed
        // returns and throws, before an oracle/canary assertion can reject it.
        output.copy(buffers,n,row);
        const uint64_t duration = result.end-result.begin;
        const uint64_t allowance = mode == kOffline ? 60000000000ULL : n ? uint64_t(n)*1000000000ULL/uint64_t(rate) : 1000000ULL;
        callbackMax = std::max(callbackMax,duration); overruns += duration > allowance;
        rejected += result.result != kResultOk; effects += result.effects; localWakes += result.localWakes;
        if (failure) std::rethrow_exception(failure);
        need(result.effects == 0, "process callback allocation/blocking/filesystem effect");
        if (!expectedFailure) { ok(result.result, "SDK process refused"); oracle.compare(buffers); }
        else buffers.guards(n);
        pendingAudio |= n != 0;
        return result;
    }
    void tail(int mode,int block,int rate) {
        if (!pendingAudio) return;
        int left = o.delay+vendorLatency;
        while (left) { const int n = std::min(left,block); process(n,mode,rate,false,false,true); left -= n; }
        // This host-supplied flush checks results due exactly at the final sample
        // position. It does not invent future audio or hide a failed callback.
        process(0,mode,rate); pendingAudio = false;
    }
    void zeroes(int mode,int rate) {
        for (double gain : {.25,.5,.75,.625}) process(0,mode,rate,false,false,false,gain);
    }
    void offlineMatrix(int block,int rate) {
        configure(kOffline,block,rate); zeroes(kOffline,rate);
        for (int n = 0; n <= block; ++n) process(n,kOffline,rate,n==1);
        process(block,kOffline,rate);
        process(block,kOffline,rate,false,true);
        tail(kOffline,block,rate); life.stop();
    }
    void recalledStateAudio(int block,int rate) {
        configure(kOffline,block,rate);
        // The first nonzero audio after restore uses the restored values. No
        // parameter point may repair a missed component restore before this.
        process(block,kOffline,rate,true,true);
        tail(kOffline,block,rate);life.stop();
    }
    void modes(int block,int rate) {
        configure(kRealtime,block,rate); zeroes(kRealtime,rate);
        // Unpaced bursts are deliberate. No sleeping or larger hidden D makes
        // this a real-time timing qualification by itself.
        for (int b = 0; b < 96; ++b) {
            const int n = b%11 == 5 ? std::max(1,block-1) : block;
            process(n,b%2?kPrefetch:kRealtime,rate,b==0,b==95);
            if (b%23 == 7) process(0,b%2?kPrefetch:kRealtime,rate);
        }
        tail(kRealtime,block,rate); life.stop();
    }
    CallbackResult sustainedMain(int block,int rate,bool first,bool final,double operation=-1.) {
        try {
            auto result=process(block,kRealtime,rate,first,final,false,-1.,operation);
            sustainedCount=rowCount-sustainedBegin;
            return result;
        } catch (...) {
            // process() owns the row before it checks the returned status,
            // callback effects or output oracle. Keep that refused attempt in
            // the measured main window before preserving the original failure.
            sustainedCount=rowCount-sustainedBegin;
            throw;
        }
    }
    void sustained(int block,int rate) {
        configure(kRealtime,block,rate);zeroes(kRealtime,rate);
        // This is an unpaced SDK-host measurement. Every process call and all
        // output storage are prepared before entry; statistics are computed
        // only after processing and lifecycle retirement have quiesced.
        sustainedAttempted=true;sustainedBegin=rowCount;
        for(size_t b=0;b<sustainedCallbacks;++b) {
            sustainedMain(block,rate,b==0,b+1==sustainedCallbacks);
        }
        tail(kRealtime,block,rate);life.stop();
    }
    void report() const {
        std::cout << "{\"event\":\"completion_output\",\"schema\":1,\"file_suffix\":\".output.f32le\""
                  << ",\"format\":\"IEEE754-float32\",\"endianness\":\"little\",\"interleaving\":\"frame-major\""
                  << ",\"lanes\":[\"main-left\",\"main-right\",\"aux-left\",\"aux-right\"]"
                  << ",\"callback_capacity\":" << maximumCallbacks << ",\"maximum_frames\":" << maximum
                  << ",\"prepared_bytes\":" << OutputCapture::capacity*sizeof(float)
                  << ",\"frames\":" << output.count/OutputCapture::lanes << ",\"float_count\":" << output.count
                  << ",\"byte_count\":" << output.count*sizeof(float) << ",\"written_bytes\":" << output.writtenBytes
                  << ",\"write_complete\":" << (output.written?"true":"false")
                  << ",\"processing_still_owned\":" << (life.processing?"true":"false")
                  << ",\"no_borrowed_callback_inflight\":true}" << std::endl;
        std::cout << "{\"event\":\"completion_audio\",\"schema\":1,\"role\":\"" << (o.instrument?"instrument":"effect")
                  << "\",\"delivery_frames\":" << o.delay << ",\"vendor_latency_frames\":13,\"callbacks\":" << rowCount
                  << ",\"compared_samples\":" << oracle.compared << ",\"mismatches\":" << oracle.mismatches
                  << ",\"nonfinite\":" << oracle.nonfinite << ",\"nonzero\":" << oracle.nonzero
                  << ",\"max_error\":" << oracle.maxError << ",\"returned_events\":" << oracle.returnedEvents
                  << ",\"returned_points\":" << oracle.returnedPoints << ",\"rejected_callbacks\":" << rejected
                  << ",\"callback_max_ns\":" << callbackMax << ",\"whole_callback_overruns\":" << overruns
                  << ",\"callback_effects\":" << effects << ",\"callback_local_wakes\":" << localWakes
                  << ",\"local_wake_counter_available\":" << (auditLocalWakes?"true":"false") << ",\"callback_audited\":true}" << std::endl;
        std::cout << "{\"event\":\"completion_timing\",\"schema\":1,\"clock\":\"steady_monotonic\",\"rows\":[";
        for (size_t i = 0; i < rowCount; ++i) {
            const auto& r = rows[i]; if (i) std::cout << ',';
            std::cout << "{\"callback\":" << i << ",\"begin_ns\":" << r.begin << ",\"end_ns\":" << r.end
                      << ",\"N\":" << r.n << ",\"actual_mode\":" << r.mode << ",\"sample_position\":" << r.samplePosition
                      << ",\"result\":" << r.result << ",\"effects\":" << r.effects << ",\"local_wakes\":" << r.localWakes
                      << ",\"in_place\":" << (r.inPlace?"true":"false") << ",\"threw\":" << (r.threw?"true":"false")
                      << ",\"output_float_offset\":" << r.outputOffset << ",\"output_float_count\":" << r.outputCount
                      << ",\"output_byte_offset\":" << r.outputOffset*sizeof(float)
                      << ",\"output_byte_count\":" << r.outputCount*sizeof(float) << '}';
        }
        std::cout << "]}" << std::endl;
        if(sustainedAttempted) {
            const uint64_t allowance=uint64_t(o.block)*1000000000ULL/uint64_t(o.rate);
            if(sustainedCount) {
                const auto statistics=wholeCallbackStatistics(rows,sustainedBegin,sustainedCount,allowance);
                std::cout << "{\"event\":\"completion_sustained_timing\",\"schema\":1"
                          << ",\"callbacks\":" << statistics.callbacks << ",\"requested_callbacks\":" << sustainedCallbacks
                          << ",\"complete\":" << (statistics.callbacks==sustainedCallbacks?"true":"false")
                          << ",\"actual_frames\":" << o.block << ",\"sample_rate\":" << o.rate
                          << ",\"delivery_frames\":" << o.delay << ",\"process_mode\":" << kRealtime
                          << ",\"whole_callback_median_ns\":" << statistics.median
                          << ",\"whole_callback_p99_ns\":" << statistics.p99
                          << ",\"whole_callback_max_ns\":" << statistics.maximum
                          << ",\"local_n_over_fs_allowance_ns\":" << allowance
                          << ",\"local_n_over_fs_overruns\":" << statistics.overruns
                          << ",\"unpaced_sdk_host\":true,\"daw_deadline_claim\":false,\"soak_claim\":false}"
                          << std::endl;
            } else {
                std::cout << "{\"event\":\"completion_sustained_timing\",\"schema\":1,\"callbacks\":0"
                          << ",\"requested_callbacks\":" << sustainedCallbacks
                          << ",\"complete\":false,\"actual_frames\":" << o.block
                          << ",\"sample_rate\":" << o.rate << ",\"delivery_frames\":" << o.delay
                          << ",\"unpaced_sdk_host\":true,\"daw_deadline_claim\":false,\"soak_claim\":false}"
                          << std::endl;
            }
        }
    }
};
void writePrivate(const std::string& path,const uint8_t* bytes,size_t size,const char* why) {
    int fd=::open(path.c_str(),O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC|O_NOFOLLOW,0600);
    need(fd>=0,why);size_t written=0;
    try {
        while(written<size) {
            const auto result=::write(fd,bytes+written,size-written);
            if(result<0&&errno==EINTR)continue;
            need(result>0,why);written+=size_t(result);
        }
        const int result=::close(fd);fd=-1;need(result==0,why);
    } catch (...) { if(fd>=0)::close(fd);throw; }
}
std::optional<std::vector<uint8_t>> readPrivate(const std::string& path,size_t maximumSize,
                                                bool missingAllowed,const char* why) {
    int fd=::open(path.c_str(),O_RDONLY|O_NONBLOCK|O_CLOEXEC|O_NOFOLLOW);
    if(fd<0&&missingAllowed&&errno==ENOENT)return std::nullopt;
    need(fd>=0,why);
    try {
        struct stat info{};need(::fstat(fd,&info)==0&&S_ISREG(info.st_mode)&&info.st_uid==::getuid()
            &&(info.st_mode&0177)==0&&info.st_size>0&&uint64_t(info.st_size)<=maximumSize,why);
        std::vector<uint8_t> bytes(static_cast<size_t>(info.st_size));size_t read=0;
        while(read<bytes.size()) {
            const auto result=::read(fd,bytes.data()+read,bytes.size()-read);
            if(result<0&&errno==EINTR)continue;
            need(result>0,why);read+=size_t(result);
        }
        uint8_t extra=0;ssize_t result=0;
        do result=::read(fd,&extra,1);while(result<0&&errno==EINTR);
        need(result==0,why);const int closed=::close(fd);fd=-1;need(closed==0,why);return bytes;
    } catch (...) { if(fd>=0)::close(fd);throw; }
}
void writeState(const char* prefix,const char* suffix,const LVBState::Stream& state) {
    need(!state.failed && state.quiescent() && !state.bytes.empty(), "complete lexical opaque state required");
    writePrivate(std::string(prefix)+suffix,state.bytes.data(),state.bytes.size(),"private unique state output");
}
void originalStateRetained(const LVBState::Stream&,const std::vector<uint8_t>&);
struct ExternalState {
    std::string componentPath,controllerPath;
    std::vector<uint8_t> componentBytes,controllerBytes;
    LVBState::Stream component,controller;
    explicit ExternalState(const Options& o)
        :componentPath(std::string(o.prefix)+".component"),controllerPath(std::string(o.prefix)+".controller"),
         componentBytes(*readPrivate(componentPath,LVBState::payloadLimit+LVBState::overhead,false,"private component state input")),
         controllerBytes(*readPrivate(controllerPath,LVBState::payloadLimit+LVBState::overhead,false,"private controller state input")),
         component(componentBytes,LVBState::payloadLimit+LVBState::overhead),
         controller(controllerBytes,LVBState::payloadLimit+LVBState::overhead) {}
};
void restoreExternalState(IComponent& component,IEditController& controller,ExternalState& state) {
    state.component.position=0;ok(component.setState(&state.component),"external opaque component restore");
    state.component.position=0;ok(controller.setComponentState(&state.component),"external component/controller synchronization");
    need(controller.getParamNormalized(0)==.625&&controller.getParamNormalized(1)==.125,
         "meaningful external component values");
    state.controller.position=0;ok(controller.setState(&state.controller),"external opaque controller restore");
    need(controller.getParamNormalized(0)==.625&&controller.getParamNormalized(1)==.125,
         "meaningful external controller values");
    originalStateRetained(state.component,state.componentBytes);
    originalStateRetained(state.controller,state.controllerBytes);
}
bool factoryReleased(const Options& o) {
    const auto token=readPrivate(std::string(o.prefix)+".factory-release",8,true,"private factory release token");
    if(!token)return false;
    static constexpr std::array<uint8_t,8> expected{'r','e','l','e','a','s','e','\n'};
    need(std::equal(token->begin(),token->end(),expected.begin(),expected.end()),"exact factory release token");
    return true;
}
// Control-only ceiling for the retained six-class 9,120-second package-refresh
// envelope plus observer orchestration. The operator releases it immediately
// when Update Bridge completes; this is not an audio-processing allowance.
constexpr auto factoryWaitSeconds=9750;
constexpr auto factoryWaitBound=std::chrono::seconds(factoryWaitSeconds);
void holdFactory(const Options& o) {
    const std::string marker=std::string("{\"schema\":1,\"factory_cached\":true,\"processor\":\"")+o.processor
        +"\",\"controller\":\""+o.controller+"\",\"wait_bound_seconds\":"+std::to_string(factoryWaitSeconds)+"}\n";
    writePrivate(std::string(o.prefix)+".factory-ready",reinterpret_cast<const uint8_t*>(marker.data()),marker.size(),
                 "private unique factory-ready marker");
    std::cout<<"{\"event\":\"factory_held\",\"schema\":1,\"metadata_cached\":true,\"wait_bound_seconds\":"
             <<factoryWaitSeconds<<"}"<<std::endl;
    const auto deadline=Clock::now()+factoryWaitBound;
    while(!factoryReleased(o)) {
        need(Clock::now()<deadline,"factory release deadline");
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    std::cout<<"{\"event\":\"factory_released\",\"schema\":1}"<<std::endl;
}
std::array<uint8_t,32> stateSha256(const uint8_t* bytes,size_t length) {
    static constexpr std::array<uint32_t,64> constants{
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2};
    std::array<uint32_t,8> hash{0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,
                                0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19};
    const uint64_t bits=uint64_t(length)*8;
    const size_t blocks=(length+9+63)/64;
    for(size_t block=0;block<blocks;++block) {
        std::array<uint8_t,64> input{};const size_t begin=block*64;
        for(size_t i=0;i<64;++i) {
            const size_t position=begin+i;
            if(position<length)input[i]=bytes[position];
            else if(position==length)input[i]=0x80;
        }
        if(block+1==blocks)for(size_t i=0;i<8;++i)input[63-i]=uint8_t(bits>>(i*8));
        std::array<uint32_t,64> words{};
        for(size_t i=0;i<16;++i)words[i]=(uint32_t(input[i*4])<<24)|(uint32_t(input[i*4+1])<<16)
            |(uint32_t(input[i*4+2])<<8)|uint32_t(input[i*4+3]);
        for(size_t i=16;i<64;++i) {
            const auto s0=std::rotr(words[i-15],7)^std::rotr(words[i-15],18)^(words[i-15]>>3);
            const auto s1=std::rotr(words[i-2],17)^std::rotr(words[i-2],19)^(words[i-2]>>10);
            words[i]=words[i-16]+s0+words[i-7]+s1;
        }
        auto [a,b,c,d,e,f,g,h]=hash;
        for(size_t i=0;i<64;++i) {
            const auto s1=std::rotr(e,6)^std::rotr(e,11)^std::rotr(e,25);
            const auto choice=(e&f)^((~e)&g);
            const auto first=h+s1+choice+constants[i]+words[i];
            const auto s0=std::rotr(a,2)^std::rotr(a,13)^std::rotr(a,22);
            const auto majority=(a&b)^(a&c)^(b&c);
            const auto second=s0+majority;
            h=g;g=f;f=e;e=d+first;d=c;c=b;b=a;a=first+second;
        }
        hash[0]+=a;hash[1]+=b;hash[2]+=c;hash[3]+=d;
        hash[4]+=e;hash[5]+=f;hash[6]+=g;hash[7]+=h;
    }
    std::array<uint8_t,32> result{};
    for(size_t i=0;i<hash.size();++i)for(size_t j=0;j<4;++j)result[i*4+j]=uint8_t(hash[i]>>(24-j*8));
    return result;
}
uint32_t stateWord(const uint8_t* bytes) {
    return uint32_t(bytes[0])|(uint32_t(bytes[1])<<8)|(uint32_t(bytes[2])<<16)|(uint32_t(bytes[3])<<24);
}
struct CompletionStateView {
    std::array<size_t,4> valueOffsets{};
    std::array<double,4> values{};
};
std::array<uint8_t,16> completionFixtureClass(bool instrument) {
    return instrument ? std::array<uint8_t,16>{'L','V','B','B','C','M','P','1','I','N','S','T',0,0,0,1}
                      : std::array<uint8_t,16>{'L','V','B','B','C','M','P','1','E','F','F','X',0,0,0,1};
}
CompletionStateView completionState(const std::vector<uint8_t>& bytes,const Options& o) {
    constexpr size_t header=104,vendorBytes=24,parameterBytes=16;
    need(bytes.size()>=header,"completion state envelope extent");
    need(std::equal(bytes.begin(),bytes.begin()+8,reinterpret_cast<const uint8_t*>("LVBSTATE"))
        &&stateWord(bytes.data()+8)==3&&stateWord(bytes.data()+12)==header,"completion state envelope version");
    need(stateWord(bytes.data()+64)==bytes.size()-header&&stateWord(bytes.data()+68)==0,
         "completion state envelope length/reserved");
    const auto digest=stateSha256(bytes.data()+header,bytes.size()-header);
    need(std::equal(digest.begin(),digest.end(),bytes.begin()+72),"completion state envelope integrity");
    const auto fixtureClass=completionFixtureClass(o.instrument);
    need(std::equal(fixtureClass.begin(),fixtureClass.end(),bytes.begin()+16),"completion fixture class provenance");
    need(std::any_of(bytes.begin()+32,bytes.begin()+64,[](uint8_t value){return value!=0;}),
         "completion state module provenance");
    const auto* payload=bytes.data()+header;const size_t payloadBytes=bytes.size()-header;
    need(payloadBytes>=16,"completion commercial state header");
    const auto component=stateWord(payload),controller=stateWord(payload+4),parameters=stateWord(payload+8),flags=stateWord(payload+12);
    need(component==vendorBytes&&controller==vendorBytes&&parameters==4&&flags==3
        &&payloadBytes==16+component+controller+parameters*parameterBytes,"completion commercial state structure");
    std::array<uint8_t,vendorBytes> expected{'L','V','B','B',1,uint8_t(o.instrument),0,0};
    const double gain=.625,colour=.125;std::memcpy(expected.data()+8,&gain,8);std::memcpy(expected.data()+16,&colour,8);
    need(std::equal(expected.begin(),expected.end(),payload+16),"completion opaque component state");
    need(std::equal(expected.begin(),expected.end(),payload+16+component),"completion opaque controller state");
    CompletionStateView view;std::array<bool,4> seen{};const std::array<uint32_t,4> ids{0,1,31,32};
    const size_t records=16+component+controller;
    for(size_t record=0;record<parameters;++record) {
        const auto* value=payload+records+record*parameterBytes;const auto id=stateWord(value),available=stateWord(value+4);
        auto found=std::find(ids.begin(),ids.end(),id);need(found!=ids.end()&&available==1,"completion parameter identity/availability");
        const size_t index=size_t(found-ids.begin());need(!seen[index],"completion parameter identity uniqueness");seen[index]=true;
        view.valueOffsets[index]=header+records+record*parameterBytes+8;
        std::memcpy(&view.values[index],value+8,8);need(std::isfinite(view.values[index])&&view.values[index]>=0&&view.values[index]<=1,
                                                        "completion parameter normalized value");
    }
    need(std::all_of(seen.begin(),seen.end(),[](bool value){return value;}),"completion parameter identity census");
    need(view.values[0]==gain&&view.values[1]==colour&&view.values[3]==0.,"completion durable parameter mirror");
    return view;
}
void slowStateRecall(const Options& o,const std::vector<uint8_t>& original,const std::vector<uint8_t>& recalled) {
    const auto before=completionState(original,o),after=completionState(recalled,o);
    need(before.values[2]==1.&&after.values[2]==.25,"slow-offline fresh fixture operation mirror");
    need(before.valueOffsets[2]==after.valueOffsets[2],"slow-offline parameter record structure");
    need(std::equal(original.begin(),original.begin()+72,recalled.begin()),"slow-offline state provenance");
    need(original.size()==recalled.size(),"slow-offline state extent");
    for(size_t i=104;i<original.size();++i)
        if(i<before.valueOffsets[2]||i>=before.valueOffsets[2]+8)need(original[i]==recalled[i],"slow-offline unexpected state difference");
}
void originalStateRetained(const LVBState::Stream& original,const std::vector<uint8_t>& saved) {
    need(!original.failed&&original.quiescent(),"original state stream bounds/lifetime");
    need(original.bytes==saved,"original saved component state changed during recall");
}
void stateRoundtrip(Lifecycle& life,const Options& o,LVBState::Stream& original) {
    const auto saved=original.bytes;
    original.position = 0; ok(life.component->setState(&original), "original opaque component recall");
    original.position = 0; ok(life.controller->setComponentState(&original), "original controller synchronization");
    need(life.controller->getParamNormalized(0) == .625 && life.controller->getParamNormalized(1) == .125,
         "recognizable recalled parameters");
    LVBState::Stream recalled; ok(life.component->getState(&recalled), "recalled component capture");
    LVBState::Stream retained(saved,LVBState::payloadLimit+LVBState::overhead);
    writeState(o.prefix,".component",retained);writeState(o.prefix,".component.recalled",recalled);
    originalStateRetained(original,saved);
    if(o.scenario=="slow-offline")slowStateRecall(o,saved,recalled.bytes);
    else need(recalled.bytes==saved,"byte-exact component state recall");
    LVBState::Stream control; ok(life.controller->getState(&control), "controller capture");
    control.position = 0; ok(life.controller->setState(&control), "controller recall");
    LVBState::Stream controlAgain; ok(life.controller->getState(&controlAgain), "controller recapture");
    need(control.bytes == controlAgain.bytes, "byte-exact controller state recall");
    writeState(o.prefix,".controller",control);
    std::cout << "{\"event\":\"completion_state\",\"component_bytes\":" << original.bytes.size()
              << ",\"controller_bytes\":" << control.bytes.size() << ",\"component_recall\":true,\"controller_recall\":true}" << std::endl;
    if(o.scenario=="state-record") {
        const auto view=completionState(saved,o);
        need(view.values[0]==.625&&view.values[1]==.125,"meaningful external state record values");
        std::cout<<"{\"event\":\"external_state_recorded\",\"schema\":1,\"component_bytes\":"<<saved.size()
                 <<",\"controller_bytes\":"<<control.bytes.size()<<",\"meaningful_gain\":0.625,\"meaningful_colour\":0.125"
                 <<",\"input_bytes_owned_by_host\":true}"<<std::endl;
    }
}
void captureExternalRecall(Lifecycle& life,const Options& o,ExternalState& source) {
    LVBState::Stream current;ok(life.component->getState(&current),"current component state capture after external recall");
    const auto sourceView=completionState(source.componentBytes,o);
    const auto currentView=completionState(current.bytes,o);
    need(sourceView.values[0]==.625&&sourceView.values[1]==.125
        &&currentView.values[0]==.625&&currentView.values[1]==.125,"meaningful migrated component state values");
    need(current.bytes==source.componentBytes,"unchanged fixture exact external component state");
    current.position=0;ok(life.component->setState(&current),"current component state replay");
    current.position=0;ok(life.controller->setComponentState(&current),"current controller synchronization");
    need(life.controller->getParamNormalized(0)==.625&&life.controller->getParamNormalized(1)==.125,
         "meaningful current controller values");
    LVBState::Stream recalled;ok(life.component->getState(&recalled),"current component state recapture");
    need(recalled.bytes==current.bytes,"unchanged fixture exact current component roundtrip");
    LVBState::Stream control;ok(life.controller->getState(&control),"current controller state capture");
    need(control.bytes==source.controllerBytes,"unchanged fixture exact external controller state");
    control.position=0;ok(life.controller->setState(&control),"current controller state replay");
    LVBState::Stream controlAgain;ok(life.controller->getState(&controlAgain),"current controller state recapture");
    need(controlAgain.bytes==control.bytes,"unchanged fixture exact current controller roundtrip");
    originalStateRetained(source.component,source.componentBytes);
    originalStateRetained(source.controller,source.controllerBytes);
    const auto componentOnDisk=readPrivate(source.componentPath,LVBState::payloadLimit+LVBState::overhead,false,
                                           "external component state reread");
    const auto controllerOnDisk=readPrivate(source.controllerPath,LVBState::payloadLimit+LVBState::overhead,false,
                                            "external controller state reread");
    need(componentOnDisk&&*componentOnDisk==source.componentBytes
        &&controllerOnDisk&&*controllerOnDisk==source.controllerBytes,"external state input files changed");
    writeState(o.capturePrefix,".component",current);
    writeState(o.capturePrefix,".component.recalled",recalled);
    writeState(o.capturePrefix,".controller",control);
    std::cout<<"{\"event\":\"completion_state\",\"schema\":2,\"source_component_bytes\":"
             <<source.componentBytes.size()<<",\"source_controller_bytes\":"<<source.controllerBytes.size()
             <<",\"current_component_bytes\":"<<current.bytes.size()<<",\"current_controller_bytes\":"<<control.bytes.size()
             <<",\"external_input_unchanged\":true,\"meaningful_gain\":0.625,\"meaningful_colour\":0.125"
             <<",\"component_exact_for_unchanged_fixture\":true,\"controller_exact_for_unchanged_fixture\":true}"<<std::endl;
}
void slow(Exercise& run) {
    run.configure(kOffline,run.o.block,run.o.rate); run.zeroes(kOffline,run.o.rate);
    run.process(0,kOffline,run.o.rate,false,false,false,-1.,1.); // arm next capture's declared short delay
    LVBState::Stream before; ok(run.life.component->getState(&before), "armed idle state capture");
    // Re-arm after obtaining the independently retained comparison payload.
    run.process(0,kOffline,run.o.rate,false,false,false,-1.,1.);
    std::atomic<uint64_t> captureBegin{0};
    const auto audioRow = run.rowCount;
    std::exception_ptr audioFailure;
    std::thread audio([&] {
        try {
            while (!captureBegin.load(std::memory_order_acquire)) std::this_thread::yield();
            // getState is issued first. Exact transport Capture admission must
            // be joined from retained owner evidence; these timestamps alone
            // cannot claim it. The fixture reply stays pending for200ms.
            std::this_thread::sleep_until(Clock::time_point(std::chrono::nanoseconds(captureBegin.load()))+std::chrono::milliseconds(50));
            run.process(run.o.block,kOffline,run.o.rate,true,true,false,-1.,.25);
        } catch (...) { audioFailure = std::current_exception(); }
    });
    LVBState::Stream captured; std::exception_ptr captureFailure;
    uint64_t end = 0;
    const uint64_t begin = nowNs(); captureBegin.store(begin,std::memory_order_release);
    try { ok(run.life.component->getState(&captured), "capture while offline audio is pending"); }
    catch (...) { captureFailure = std::current_exception(); }
    end = nowNs();
    audio.join(); // borrowed SDK buffers and module stay alive through completion
    const bool recorded = run.rowCount == audioRow+1;
    const Row row = recorded ? run.rows[audioRow] : Row{};
    const bool inflight = row.begin < end && end < row.end;
    std::cout << "{\"event\":\"capture_overlap_observation\",\"capture_begin_ns\":" << begin
              << ",\"capture_end_ns\":" << end << ",\"audio_begin_ns\":" << row.begin
              << ",\"audio_end_ns\":" << row.end << ",\"audio_inflight_at_capture_return\":" << (inflight?"true":"false")
              << ",\"audio_callback_interval_recorded\":" << (recorded?"true":"false")
              << ",\"capture_issued_before_audio_begin\":" << (begin<row.begin?"true":"false")
              << ",\"capture_within_original_10s\":" << (end-begin<10000000000ULL?"true":"false")
              << ",\"audio_remaining_after_capture_start_ns\":" << (row.end>begin?row.end-begin:0)
              << ",\"transport_capture_admission_proven_by_sdk_timestamps\":false}" << std::endl;
    if (audioFailure) std::rethrow_exception(audioFailure);
    if (captureFailure) std::rethrow_exception(captureFailure);
    need(recorded && row.n == run.o.block && row.mode == kOffline && captureOverlaps(begin,end,row),
         "prompt capture inside the actual offline callback interval with audio beyond10s");
    need(captured.bytes == before.bytes && captured.quiescent(), "independent exact captured state payload");
    run.tail(kOffline,run.o.block,run.o.rate); run.life.stop();
    stateRoundtrip(run.life,run.o,captured);
}
[[noreturn]] void abrupt(Exercise& run) {
    run.configure(kOffline,run.o.block,run.o.rate); run.zeroes(kOffline,run.o.rate);
    std::atomic<uint64_t> began{0}; std::atomic<bool> done{false}; std::exception_ptr failure;
    std::thread audio([&] {
        began.store(nowNs(),std::memory_order_release);
        try { run.process(run.o.block,kOffline,run.o.rate,true,true,false,-1.,.25); }
        catch (...) { failure = std::current_exception(); }
        done.store(true,std::memory_order_release);
    });
    while (!began.load(std::memory_order_acquire)) std::this_thread::yield();
    const auto start = began.load();
    std::this_thread::sleep_until(Clock::time_point(std::chrono::nanoseconds(start))+std::chrono::milliseconds(500));
    if (done.load(std::memory_order_acquire)) {
        audio.join();
        if (failure) std::rethrow_exception(failure);
        throw std::runtime_error("offline audio ended before intentional disappearance");
    }
    std::cout << "{\"event\":\"intentional_consumer_disappearance\",\"audio_call_begin_ns\":" << start
              << ",\"disappearance_ns\":" << nowNs() << ",\"audio_inflight\":true"
              << ",\"sdk_teardown\":false,\"cancellation_success_claim\":false}" << std::endl;
    // Do not stop/deactivate concurrently with a borrowed process call. The
    // external supervisor proves containment of this exact consumer lifetime.
    std::_Exit(23);
}
} // namespace
int main(int argc,char** argv) {
    try {
        const auto o = options(argc,argv);
        std::optional<ExternalState> external;
        if(o.scenario=="state-recall") {stage="external_state_input";external.emplace(o);}
        auditBegin = reinterpret_cast<Mark>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
        auditEnd = reinterpret_cast<Count>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
        auditLocalWakes = reinterpret_cast<Count>(dlsym(RTLD_DEFAULT,"ap3_audit_local_wakes"));
        need(auditBegin && auditEnd, "preloaded callback audit required");
        auto allocate = reinterpret_cast<void*(*)(size_t)>(dlsym(RTLD_DEFAULT,"malloc"));
        auto release = reinterpret_cast<void(*)(void*)>(dlsym(RTLD_DEFAULT,"free"));
        need(allocate && release, "audit positive-control symbols");
        auditBegin(); auto* memory = allocate(32); release(memory); need(auditEnd() >= 2, "actual callback audit positive control");
        stage = "load"; std::string error;
        auto owner = lifetime(); auto& module = owner->module; auto& host = owner->host; auto& life = owner->life;
        module = VST3::Hosting::Module::create(o.bundle,error); need(bool(module), "SDK module load");
        host = owned(new HostApplication); module->getFactory().setHostContext(host);
        const auto classes = module->getFactory().classInfos(); need(classes.size() == 2, "exact two-class fixture");
        need(classes[0].ID().toString() == o.processor && classes[1].ID().toString() == o.controller, "exact declared native class pair");
        if(o.scenario=="held-factory-refusal") {stage="factory_gate";holdFactory(o);stage="post_update_instance_setup";}
        life.component = module->getFactory().createInstance<IComponent>(classes[0].ID()); need(bool(life.component), "component factory");
        ok(life.component->initialize(host), "component initialize"); life.componentInitialized = true;
        life.processor = FUnknownPtr<IAudioProcessor>(life.component); need(bool(life.processor), "processor interface");
        TUID controllerId{}; ok(life.component->getControllerClassId(controllerId), "declared controller class");
        char8 text[33]{}; FUID(controllerId).toString(text); need(classes[1].ID().toString() == text, "component/controller identity binding");
        life.controller = module->getFactory().createInstance<IEditController>(classes[1].ID()); need(bool(life.controller), "controller factory");
        ok(life.controller->initialize(host), "controller initialize"); life.controllerInitialized = true;
        life.cp = FUnknownPtr<IConnectionPoint>(life.component); life.cc = FUnknownPtr<IConnectionPoint>(life.controller);
        need(life.cp && life.cc, "ordinary connection interfaces");
        ok(life.cp->connect(life.cc), "component connect"); life.componentConnected = true;
        ok(life.cc->connect(life.cp), "controller connect"); life.controllerConnected = true;
        need(life.controller->getParameterCount() == 4, "exact fixture parameter census");
        for (int i = 0; i < 4; ++i) { ParameterInfo info{}; ok(life.controller->getParameterInfo(i,info), "parameter metadata");
            need(info.id == std::array<ParamID,4>{0,1,31,32}[size_t(i)], "stable parameter identities"); }
        if(external) {stage="external_state_restore";restoreExternalState(*life.component,*life.controller,*external);}
        need(life.component->getBusCount(kAudio,kInput) == (o.instrument?0:1)
            && life.component->getBusCount(kAudio,kOutput) == 2 && life.component->getBusCount(kEvent,kInput) == 1
            && life.component->getBusCount(kEvent,kOutput) == 1, "exact declared stereo/event buses");
        SpeakerArrangement in = SpeakerArr::kStereo, out[2]{SpeakerArr::kStereo,SpeakerArr::kStereo};
        ok(life.processor->setBusArrangements(o.instrument?nullptr:&in,o.instrument?0:1,out,2), "ordinary bus arrangement");
        for (int media = 0; media < 2; ++media) for (int direction = 0; direction < 2; ++direction) {
            for (int index = 0; index < life.component->getBusCount(media,direction); ++index) {
                BusInfo info{}; ok(life.component->getBusInfo(media,direction,index,info), "bus metadata");
                need(info.mediaType == media && info.direction == direction && info.channelCount == (media?16:2)
                    && info.busType == (!media && direction == kOutput && index == 1?kAux:kMain), "exact coherent bus shape");
                if (!media) { SpeakerArrangement arrangement = 0; ok(life.processor->getBusArrangement(direction,index,arrangement), "bus arrangement read");
                    need(arrangement == SpeakerArr::kStereo, "exact stereo arrangement"); }
                ok(life.component->activateBus(media,direction,index,true), "ordinary bus activation");
            }
        }
        need(life.processor->canProcessSampleSize(kSample32) == kResultTrue
            && life.processor->canProcessSampleSize(kSample64) != kResultTrue, "declared float32 processing");
        stage = "processing"; Exercise run(life,o); std::exception_ptr failure;const char* failureStage=nullptr;
        try {
            run.output.prepare(o.capturePrefix);
            if(o.scenario=="held-factory-refusal") {
                stage="post_update_processing_admission";
                run.configure(kOffline,o.block,o.rate);
                throw std::runtime_error("held factory unexpectedly admitted after bridge replacement");
            }
            if(o.scenario!="state-recall") {
                run.configure(kOffline,o.block,o.rate);
                // Opaque initial state is changed through SDK parameters only.
                run.buffers.prepare(0,kOffline,o.instrument,false); point(run.buffers.sent,0,.625); point(run.buffers.sent,1,.125);
                auto initialized = call([&]{return life.processor->process(run.buffers.data);});
                need(initialized.effects == 0, "initial flush callback effects"); ok(initialized.result, "initial recognizable state flush");
                life.stop();
            }
            if (o.scenario == "slow-offline") slow(run);
            else if (o.scenario == "abrupt-offline") abrupt(run);
            else if (o.scenario == "offline-failure" || o.scenario == "offline-timeout") {
                run.configure(kOffline,o.block,o.rate); run.zeroes(kOffline,o.rate);
                const bool timeout = o.scenario == "offline-timeout";
                const auto result = run.process(o.block,kOffline,o.rate,true,true,false,-1.,timeout?.75:.5,true);
                need(result.result != kResultOk, "failure/timeout must be explicit, never success silence");
                if (timeout) need(result.end-result.begin >= 59000000000ULL && result.end-result.begin <= 60004000000ULL,
                                  "original60s deadline plus bounded4ms cancellation observation");
                life.stop();
                std::cout << "{\"event\":\"expected_failure\",\"kind\":\"" << (timeout?"offline_deadline":"vendor_failure")
                          << "\",\"result\":" << result.result << ",\"callback_ns\":" << result.end-result.begin
                          << ",\"successful_audio_claim\":false}" << std::endl;
            } else {
                if(o.scenario=="state-record"||o.scenario=="state-recall")run.recalledStateAudio(o.block,o.rate);
                if (o.scenario == "matrix" || o.scenario == "offline"
                    || o.scenario == "state-record" || o.scenario == "state-recall") {
                    run.offlineMatrix(o.block,o.rate);
                    // Legal inactive rate/block reconfiguration, independently
                    // of republishing or manager preparation.
                    const int otherRate = o.rate == 96000?48000:96000;
                    run.offlineMatrix(std::min(257,o.block),otherRate);
                }
                if (o.scenario == "matrix" || o.scenario == "modes") run.modes(o.block,o.rate);
                if (o.scenario == "sustained") run.sustained(o.block,o.rate);
                stage="state_capture";
                if(external)captureExternalRecall(life,o,*external);
                else {
                    LVBState::Stream captured; ok(life.component->getState(&captured), "idle post-processing capture");
                    stateRoundtrip(life,o,captured);
                }
            }
        } catch (...) { failure = std::current_exception();failureStage=stage; }
        stage = "retirement"; const bool clean = life.finish();
        // All process calls have returned/joined. Even a refused stop retains
        // the SDK lifetime separately; these copied samples own no vendor memory.
        std::exception_ptr outputFailure;
        try { run.output.write(); } catch (...) { outputFailure = std::current_exception(); }
        run.report();
        if (failure) { stage = failureStage; std::rethrow_exception(failure); }
        need(clean, "positive completed SDK lifecycle retirement");
        if (outputFailure) { stage = "output_retention"; std::rethrow_exception(outputFailure); }
        need(run.oracle.mismatches == 0 && run.oracle.nonfinite == 0 && run.effects == 0, "exact completion audio and callback oracle");
        if (o.scenario != "offline-failure" && o.scenario != "offline-timeout") need(run.rejected == 0 && run.oracle.nonzero > 0, "actual successful nonzero audio");
        std::cout << "{\"event\":\"passed\",\"claim\":\"installed_sdk_completion_development_regression\",\"whole_callback_timing_qualified\":"
                  << (o.scenario!="sustained"&&run.overruns==0?"true":"false")
                  << ",\"timing_scope\":\"" << (o.scenario=="sustained"?"unpaced_sdk_host_local_n_over_fs":"existing_completion_scenario")
                  << "\",\"real_daw_or_soak_claim\":false}" << std::endl;
        return 0;
    } catch (const std::exception& error) {
        std::cout << "{\"event\":\"failed\",\"stage\":\"" << stage << "\",\"reason\":\"" << error.what() << "\"}" << std::endl;
        return 1;
    }
}
