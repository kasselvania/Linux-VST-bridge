// Test the actual first-party completion producer. This is not an installed
// bridge or transport test, and does not substitute for the independent host.
#define LVB_BETA_COMPLETION 1
#include "stateful.cpp"
#include "tools/beta/sdk_buffers.h"
#include "windows-factory-probe/source/bus_layout.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "vst-state/stream.h"
#include <cstdlib>
#include <iostream>
#include <string_view>
using namespace CompletionSDK;
static void need(bool result, const char* why) {
    if (!result) { std::cerr << "FAIL: " << why << '\n'; std::exit(1); }
}
static void point(Parameters& parameters, ParamID id, int32 offset, double value) {
    int32 index = 0; auto* queue = parameters.addParameterData(id, index);
    need(queue && queue->addPoint(offset, value, index) == kResultOk, "bounded parameter point");
}
struct Block {
    std::array<std::array<float, 1024>, 2> input{};
    std::array<std::array<float, 1024>, 4> output{};
    float* in[2]{input[0].data(), input[1].data()};
    float* out[4]{output[0].data(), output[1].data(), output[2].data(), output[3].data()};
    AudioBusBuffers inputs{}, outputs[2]{};
    Parameters sent, returned;
    Events notes, results;
    ProcessData data{};
    Block() {
        inputs.numChannels = 2; inputs.channelBuffers32 = in;
        for (int i = 0; i < 2; ++i) { outputs[i].numChannels = 2; outputs[i].channelBuffers32 = out + 2*i; }
        data.symbolicSampleSize = kSample32; data.numInputs = LVB_BETA_INSTRUMENT ? 0 : 1;
        data.numOutputs = 2; data.inputs = LVB_BETA_INSTRUMENT ? nullptr : &inputs; data.outputs = outputs;
        data.inputParameterChanges = &sent; data.outputParameterChanges = &returned;
        data.inputEvents = &notes; data.outputEvents = &results;
    }
    void prepare(int n, int mode) {
        sent.clear(); returned.clear(); notes.clear(); results.clear();
        data.numSamples = n; data.processMode = mode;
        data.numInputs = n && !LVB_BETA_INSTRUMENT ? 1 : 0; data.numOutputs = n ? 2 : 0;
        data.inputs = data.numInputs ? &inputs : nullptr; data.outputs = n ? outputs : nullptr;
        inputs.silenceFlags = 0;
        for (auto& lane : input) lane.fill(0);
        for (auto& lane : output) lane.fill(-999.f);
    }
    void checkResults(int n, int mode, double gain, int echoed = 0) {
        need(returned.getParameterCount() == 2, "two exact returned parameter queues");
        for (int q = 0; q < 2; ++q) {
            auto* queue = returned.getParameterData(q); int32 offset = -1; double value = -1;
            need(queue && queue->getPointCount() == 1 && queue->getPoint(0, offset, value) == kResultOk,
                 "one point in each returned queue");
            need(queue->getParameterId() == (q ? 0u : 32u)
                && offset == (q ? std::max(0, n-1) : 0) && value == (q ? gain : double(mode)/2.),
                 "actual mode/final gain and offsets");
        }
        need(results.getEventCount() == echoed+1, "exact returned events");
        Event cc{}; int found = 0; int32 previous = 0;
        for (int i = 0; i < results.getEventCount(); ++i) {
            Event event{}; need(results.getEvent(i,event) == kResultOk
                && event.sampleOffset >= previous, "ordered returned events");
            previous = event.sampleOffset;
            if (event.type == Event::kLegacyMIDICCOutEvent) { cc = event; ++found; }
        }
        need(found == 1 && cc.busIndex == 0 && cc.sampleOffset == 0
            && cc.midiCCOut.controlNumber == 74 && cc.midiCCOut.channel == 2 && cc.midiCCOut.value == mode,
            "inline non-note result, including zero frames");
        need(sent.quiescent() && returned.quiescent() && notes.quiescent() && results.quiescent(),
             "no retained SDK sink ownership");
    }
};
int main(int argc, char** argv) {
    need(argc == 1 || (argc == 2 && std::string_view(argv[1]) == "--slow"), "optional exact slow fixture test");
    HostApplication host; ReferenceProcessor processor; ReferenceController controller;
    need(processor.initialize(&host) == kResultOk && controller.initialize(&host) == kResultOk, "initialize");
    linux_vst_bridge::wf0::BusLayout layout; layout.read(processor, processor, false);
    need(layout.counts == std::array<int,4>{LVB_BETA_INSTRUMENT?0:1,2,1,1}, "production stereo bus census accepts fixture");
    layout.negotiate(processor); layout.activate(processor, true);
    need(processor.getLatencySamples() == 13 && processor.getTailSamples() == 13, "truthful vendor latency and tail");
    ProcessSetup setup{kRealtime, kSample32, 1024, 48000.};
    need(processor.setupProcessing(setup) == kResultOk && processor.setActive(true) == kResultOk
        && processor.setProcessing(true) == kResultOk, "activate processing");
    Block block;
    for (int n = 0; n <= 1024; ++n) {
        const int mode = n%2 ? kPrefetch : kRealtime;
        block.prepare(n, mode);
        if (!n) point(block.sent, 0, 0, .625);
        need(processor.process(block.data) == kResultOk, "every actual N0..M and RT/prefetch switch");
        block.checkResults(n, mode, .625);
        for (int i = 0; i < n; ++i) for (const auto& lane : block.output) need(lane[size_t(i)] == 0.f, "exact initial silence");
    }
    // Consecutive N0 calls have distinct results without invented input notes.
    for (double value : {.25, .5, .75}) {
        block.prepare(0, kRealtime); point(block.sent, 0, 0, value);
        need(processor.process(block.data) == kResultOk, "zero-frame parameter flush");
        block.checkResults(0, kRealtime, value);
    }
    need(processor.setProcessing(false) == kResultOk && processor.setActive(false) == kResultOk, "inactive reconfiguration");
    setup = {kOffline, kSample32, 257, 96000.};
    need(processor.setupProcessing(setup) == kResultOk && processor.setActive(true) == kResultOk
        && processor.setProcessing(true) == kResultOk, "new offline/rate/block setup");
    block.prepare(257, kOffline); point(block.sent, 0, 0, .5); point(block.sent, 1, 0, .25);
    Event on{}; on.type = Event::kNoteOnEvent; on.sampleOffset = 0; on.noteOn = {2,69,0.f,.8f,0,42};
    Event off{}; off.type = Event::kNoteOffEvent; off.sampleOffset = 256; off.noteOff = {2,69,0.f,42,0.f};
    need(block.notes.addEvent(on) == kResultOk && block.notes.addEvent(off) == kResultOk, "first/final events");
    block.input[0].fill(.25f); block.input[1].fill(-.125f);
    if (!LVB_BETA_INSTRUMENT) { block.out[0] = block.in[0]; block.out[1] = block.in[1]; }
    need(processor.process(block.data) == kResultOk, "offline in-place/multi-output process");
    block.checkResults(257, kOffline, .5, 2);
    Event echoed{}; need(block.results.getEvent(0, echoed) == kResultOk && echoed.sampleOffset == 0
        && echoed.noteOn.noteId == 42 && block.results.getEvent(2, echoed) == kResultOk
        && echoed.sampleOffset == 256 && echoed.noteOff.noteId == 42, "first/final event fidelity");
    double phase = 0.; const double increment = tau*440./96000.;
    std::array<float,257> expected{};
    for (int i = 0; i < 256; ++i) {
        expected[size_t(i)] = LVB_BETA_INSTRUMENT ? float(.5*double(.8f)*(std::sin(phase)+.125*std::sin(2*phase))/16.) : .09375f;
        phase = std::fmod(phase+increment,tau);
    }
    if (!LVB_BETA_INSTRUMENT) expected[256] = .09375f;
    for (int i = 0; i < 257; ++i) {
        const float left = i < 13 ? 0.f : expected[size_t(i-13)];
        const float right = LVB_BETA_INSTRUMENT ? left : left*-.5f;
        need(std::abs(block.out[0][i]-left) < 1e-7 && std::abs(block.out[1][i]-right) < 1e-7
            && block.out[2][i] == block.out[0][i]*.5f && block.out[3][i] == block.out[1][i]*-.5f,
            "vendor13 latency, exact stereo polarity and auxiliary outputs");
    }
    // The host, rather than the bridge, supplies the finite vendor tail.
    block.out[0] = block.output[0].data(); block.out[1] = block.output[1].data();
    block.prepare(13, kOffline); need(processor.process(block.data) == kResultOk, "explicit final tail call");
    for (int i = 0; i < 13; ++i) need(std::abs(block.output[0][size_t(i)]-expected[size_t(244+i)]) < 1e-7,
                                  "final original audio frames survive");
    block.prepare(1, kRealtime); need(processor.process(block.data) != kResultOk, "offline boundary requires reconfiguration");
    block.prepare(0, kOffline); point(block.sent, 31, 0, .5);
    need(processor.process(block.data) == kResultFalse, "explicit vendor failure, never success silence");
    LVBState::Stream original; need(processor.getState(&original) == kResultOk, "opaque original state");
    original.position = 0; need(controller.setComponentState(&original) == kResultOk
        && controller.getParamNormalized(0) == .5 && controller.getParamNormalized(1) == .25, "controller state synchronization");
    block.prepare(0, kOffline); point(block.sent, 31, 0, 1.);
    need(processor.process(block.data) == kResultOk, "arm short capture delay through declared parameter");
    const auto before = std::chrono::steady_clock::now(); LVBState::Stream delayed;
    need(processor.getState(&delayed) == kResultOk && delayed.bytes == original.bytes, "delayed capture preserves opaque state");
    need(std::chrono::steady_clock::now()-before >= std::chrono::milliseconds(190), "capture fixture actually delayed reply");
    if (argc == 2) {
        block.prepare(257, kOffline); point(block.sent, 31, 0, .25);
        const auto start = std::chrono::steady_clock::now();
        need(processor.process(block.data) == kResultOk && std::chrono::steady_clock::now()-start >= std::chrono::seconds(12),
             "valid slow offline exceeds old five-second worker timeout");
        block.checkResults(257, kOffline, .5);
    }
    need(processor.setProcessing(false) == kResultOk && processor.setActive(false) == kResultOk
        && processor.terminate() == kResultOk && controller.terminate() == kResultOk, "completed SDK teardown");
    std::cout << "LVB_COMPLETION_FIXTURE_V1 role=" << (LVB_BETA_INSTRUMENT?"instrument":"effect")
              << " N0_through_1024=passed modes=passed in_place=passed multiout=passed first_final=passed state=passed slow="
              << (argc == 2?"passed":"not_run") << '\n';
}
