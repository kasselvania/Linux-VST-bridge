// One actual stateful SDK producer: editor gesture, DSP, opaque state and reopen.
#define LVB_BETA_EDITOR 1
#include "stateful.cpp"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include "vst-state/stream.h"
#include <cstdlib>
#include <iostream>
#include <vector>

static void need(bool result, const char* why) {
    if (!result) { std::cerr << "FAIL: " << why << '\n'; std::exit(1); }
}
struct Host final : HostApplication, IComponentHandler {
    std::vector<char> events;
    std::vector<double> values;
    bool refuse = false;
    tresult PLUGIN_API queryInterface(const TUID id, void** result) override {
        if (!result) return kInvalidArgument;
        if (!FUnknownPrivate::iidEqual(id, IComponentHandler::iid))
            return HostApplication::queryInterface(id, result);
        *result = static_cast<IComponentHandler*>(this); addRef(); return kResultOk;
    }
    uint32 PLUGIN_API addRef() override { return HostApplication::addRef(); }
    uint32 PLUGIN_API release() override { return HostApplication::release(); }
    tresult PLUGIN_API beginEdit(ParamID id) override {
        need(id == 0, "gesture uses actual Level parameter");
        if (refuse) return kResultFalse;
        events.push_back('B'); return kResultOk;
    }
    tresult PLUGIN_API performEdit(ParamID id, ParamValue value) override {
        need(id == 0 && normalized(value), "normalized Level callback");
        events.push_back('V'); values.push_back(value); return kResultOk;
    }
    tresult PLUGIN_API endEdit(ParamID id) override {
        need(id == 0, "matching gesture end"); events.push_back('E'); return kResultOk;
    }
    tresult PLUGIN_API restartComponent(int32) override { return kNotImplemented; }
};
int main() {
    Host host;
    auto* application = static_cast<IHostApplication*>(&host);
    ReferenceProcessor processor;
    ReferenceController controller;
    need(processor.initialize(application) == kResultOk && controller.initialize(application) == kResultOk,
        "initialize real producer and controller");
    need(controller.setComponentHandler(&host) == kResultOk, "actual SDK handler");
    auto* view = controller.createView(ViewType::kEditor);
    need(view && !controller.createView("other"), "only SDK editor view");
    ViewRect bounds{};
    need(view->getSize(&bounds) == kResultOk && bounds.getWidth() == 320 && bounds.getHeight() == 140
        && view->canResize() == kResultFalse, "declared fixed editor");
    ViewRect bad{0, 0, 1, 1};
    need(view->onSize(&bad) != kResultOk && view->checkSizeConstraint(&bad) == kResultOk
        && bad.getWidth() == 320 && bad.getHeight() == 140, "fixed SDK constraints");
    need(view->onFocus(true) != kResultOk && view->attached(nullptr, kPlatformTypeHWND) != kResultOk,
        "unattached capability refusal");
    auto parent = CreateWindowW(L"STATIC", L"Native host controls", WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        10, 10, 400, 220, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
    need(parent != nullptr, "host-owned normal window");
    need(view->attached(parent, "unsupported") != kResultOk, "unsupported platform refusal");
    need(view->attached(parent, kPlatformTypeHWND) == kResultOk, "actual HWND attachment");
    auto child = GetWindow(parent, GW_CHILD);
    need(child && IsWindowVisible(child) && GetParent(child) == parent, "visible private editor child");
    need(view->attached(parent, kPlatformTypeHWND) != kResultOk, "duplicate attach refused");
    host.refuse = true;
    SendMessageW(child, WM_LBUTTONDOWN, MK_LBUTTON, MAKELPARAM(230, 70));
    need(controller.getParamNormalized(0) == .25 && host.events.empty(), "refused begin preserves value");
    host.refuse = false;
    SendMessageW(child, WM_LBUTTONDOWN, MK_LBUTTON, MAKELPARAM(230, 70));
    SendMessageW(child, WM_MOUSEMOVE, MK_LBUTTON, MAKELPARAM(160, 70));
    SendMessageW(child, WM_LBUTTONUP, 0, MAKELPARAM(160, 70));
    need(host.events == std::vector<char>({'B', 'V', 'V', 'E'})
        && host.values == std::vector<double>({.75, .5}) && controller.getParamNormalized(0) == .5,
        "real drag begin/value/end ordering");

    // The actual callback values drive the same processor, at the real queue
    // offsets. UI state alone is not proof of sound or persistent DSP state.
    ProcessSetup setup{kRealtime, kSample32, 64, 48000.};
    need(processor.setupProcessing(setup) == kResultOk && processor.setActive(true) == kResultOk,
        "activate real effect DSP");
    ParameterChanges parameters(1); int32 index = 0;
    auto* queue = parameters.addParameterData(0, index);
    queue->addPoint(0, host.values[0], index); queue->addPoint(32, host.values[1], index);
    std::array<float, 64> left{}, right{}, inLeft{}, inRight{};
    inLeft.fill(1.f); inRight.fill(-.5f);
    float* input[]{inLeft.data(), inRight.data()}; float* output[]{left.data(), right.data()};
    AudioBusBuffers in{}, out{}; in.numChannels = out.numChannels = 2;
    in.channelBuffers32 = input; out.channelBuffers32 = output;
    ProcessData data{}; data.processMode = kRealtime; data.symbolicSampleSize = kSample32;
    data.numSamples = 64; data.numInputs = data.numOutputs = 1;
    data.inputs = &in; data.outputs = &out; data.inputParameterChanges = &parameters;
    need(processor.process(data) == kResultOk, "gesture values reach actual DSP");
    for (int sample = 0; sample < 64; ++sample)
        need(left[sample] == (sample < 32 ? .75f : .5f) && right[sample] == left[sample] * -.5f,
            "stereo audio follows gesture at exact offset");
    LVBState::Stream saved, controllerSaved;
    need(processor.getState(&saved) == kResultOk && controller.getState(&controllerSaved) == kResultOk
        && saved.bytes == controllerSaved.bytes, "matching opaque processor/controller state");
    ReferenceProcessor reopened;
    saved.position = 0;
    need(reopened.initialize(application) == kResultOk && reopened.setState(&saved) == kResultOk,
        "fresh processor restores actual opaque state");
    LVBState::Stream recalled;
    need(reopened.getState(&recalled) == kResultOk && recalled.bytes == controllerSaved.bytes,
        "fresh processor state survives recall");

    need(view->onFocus(true) == kResultOk && GetFocus() == child, "SDK focus reaches editor");
    SendMessageW(child, WM_KEYDOWN, VK_RIGHT, 0);
    need(std::abs(controller.getParamNormalized(0) - .55) < 1e-12
        && host.events.size() == 7 && host.events[4] == 'B' && host.events[6] == 'E',
        "keyboard emits a complete real gesture");
    SendMessageW(child, WM_LBUTTONDOWN, MK_LBUTTON, MAKELPARAM(90, 70));
    need(view->removed() == kResultOk && !IsWindow(child) && IsWindowVisible(parent)
        && host.events.back() == 'E', "close finishes active gesture and preserves host");
    need(view->removed() == kResultOk && view->onFocus(true) != kResultOk, "idempotent retirement");
    saved.position = 0;
    need(controller.setComponentState(&saved) == kResultOk && controller.getParamNormalized(0) == .5,
        "opaque recalled state resynchronizes editor model");
    for (int epoch = 0; epoch < 2; ++epoch) {
        need(view->attached(parent, kPlatformTypeHWND) == kResultOk, "normal editor reopen");
        child = GetWindow(parent, GW_CHILD);
        need(child && IsWindowVisible(child) && view->onSize(&bounds) == kResultOk,
            "reopened editor has fixed visible child");
        SendMessageW(child, WM_LBUTTONDOWN, MK_LBUTTON, MAKELPARAM(160, 70));
        SendMessageW(child, WM_LBUTTONUP, 0, MAKELPARAM(160, 70));
        need(controller.getParamNormalized(0) == .5 && view->removed() == kResultOk
            && !IsWindow(child) && IsWindowVisible(parent), "reopened input and normal close");
    }
    view->release(); DestroyWindow(parent);
    processor.setActive(false); processor.terminate(); reopened.terminate(); controller.terminate();
    std::cout << "LVB_EDITOR_CONTRACT_V1 gesture=passed dsp=passed state=passed focus=passed close_reopen=passed\n";
}
