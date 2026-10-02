// SDK tests for the actual distributable producer, before installed DAW tests.
#include "stateful.cpp"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include "vst-state/stream.h"
#include <cstdlib>
#include <iostream>
#include <limits>
static void need(bool result, const char* why) {
    if (!result) { std::cerr << "FAIL: " << why << '\n'; std::exit(1); }
}
int main() {
    HostApplication host;
    ReferenceProcessor processor;
    ReferenceController controller;
    need(processor.initialize(&host) == kResultOk && controller.initialize(&host) == kResultOk, "initialize");
    need(processor.getBusCount(kAudio, kInput) == (LVB_BETA_INSTRUMENT ? 0 : 1), "exact input role");
    ProcessSetup setup{kRealtime, kSample32, 64, 48000.};
    need(processor.setupProcessing(setup) == kResultOk && processor.setActive(true) == kResultOk, "activate");
    Settings selected{.75, .25};
    LVBState::Stream selectedState;
    need(state(&selectedState, selected, true) == kResultOk, "selected state");
    selectedState.position = 0;
    need(processor.setState(&selectedState) == kResultOk, "restore selected state");
    LVBState::Stream captured;
    need(processor.getState(&captured) == kResultOk && captured.bytes == selectedState.bytes, "recognizable capture");
    captured.position = 0;
    need(controller.setComponentState(&captured) == kResultOk && controller.getParamNormalized(0) == .75
        && controller.getParamNormalized(1) == .25, "controller synchronization");
    LVBState::Stream controllerState;
    need(controller.getState(&controllerState) == kResultOk && controllerState.bytes == captured.bytes, "controller capture");
    for (int fault = 0; fault < 3; ++fault) {
        auto bytes = captured.bytes;
        if (fault == 0) bytes.resize(23);
        if (fault == 1) bytes[5] ^= 1; // A different role is never accepted.
        if (fault == 2) { double nan = std::numeric_limits<double>::quiet_NaN(); std::memcpy(bytes.data() + 8, &nan, 8); }
        LVBState::Stream invalid(bytes);
        need(processor.setState(&invalid) != kResultOk, "invalid state refused");
        LVBState::Stream unchanged;
        need(processor.getState(&unchanged) == kResultOk && unchanged.bytes == captured.bytes, "failed restore preserves state");
    }
    std::array<float, 64> inputLeft{}, inputRight{}, left{}, right{};
    inputLeft.fill(1.f); inputRight.fill(-.5f);
    float* input[] = {inputLeft.data(), inputRight.data()};
    float* output[] = {left.data(), right.data()};
    AudioBusBuffers in{}, out{}; in.numChannels = out.numChannels = 2;
    in.channelBuffers32 = input; out.channelBuffers32 = output;
    ProcessData data{}; data.processMode = kRealtime; data.symbolicSampleSize = kSample32;
    data.numSamples = 64; data.numInputs = LVB_BETA_INSTRUMENT ? 0 : 1; data.numOutputs = 1;
    data.inputs = LVB_BETA_INSTRUMENT ? nullptr : &in; data.outputs = &out;
    EventList notes(2);
    Event on{}; on.type = Event::kNoteOnEvent; on.sampleOffset = 7;
    on.noteOn = {2, 69, 0.f, .8f, 0, 42};
    Event off{}; off.type = Event::kNoteOffEvent; off.sampleOffset = 43;
    off.noteOff = {2, 69, 0.f, 42, 0.f};
    notes.addEvent(on); notes.addEvent(off);
    if (LVB_BETA_INSTRUMENT) data.inputEvents = &notes;
    ParameterChanges parameters(1);
    int32 index = 0;
    auto queue = parameters.addParameterData(0, index);
    queue->addPoint(32, .25, index);
    data.inputParameterChanges = &parameters;
    need(processor.process(data) == kResultOk, "process notes and sample-offset automation");
    for (int i = 0; i < 64; ++i) {
        need(std::isfinite(left[i]) && std::isfinite(right[i]), "finite output");
        if (LVB_BETA_INSTRUMENT) {
            if (i <= 7 || i >= 43) need(left[i] == 0.f && right[i] == 0.f, "exact note onset/release");
        } else {
            const float expected = float((i < 32 ? .75 : .25) * .75);
            need(left[i] == expected && right[i] == expected * -.5f, "stereo and exact automation offset");
        }
    }
    if (LVB_BETA_INSTRUMENT) need(left[20] > 0.f, "real synthesized signal");
    LVBState::Stream automated;
    need(processor.getState(&automated) == kResultOk, "capture automated state");
    automated.position = 0; Settings recalled;
    need(state(&automated, recalled, false) == kResultOk && recalled.gain == .25 && recalled.colour == .25, "automation persists");
    processor.setActive(false); processor.terminate(); controller.terminate();
    std::cout << "LVB_REFERENCE_CONTRACT_V1 role=" << (LVB_BETA_INSTRUMENT ? "instrument" : "effect")
        << " state=passed automation=passed audio=passed note_release=passed\n";
}
