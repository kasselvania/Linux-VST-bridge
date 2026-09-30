// First-party SDK instrumentation for delivered installation and project recall.
// Compile separately as an instrument and effect. No vendor state or DSP.
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "public.sdk/source/main/pluginfactory.h"
#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <algorithm>
#include <array>
#include <cmath>
#include <cstring>

using namespace Steinberg;
using namespace Steinberg::Vst;
#ifndef LVB_BETA_INSTRUMENT
#define LVB_BETA_INSTRUMENT 0
#endif
static const FUID processorID(0x4C564242, 0x45544131,
    LVB_BETA_INSTRUMENT ? 0x494E5354 : 0x45464658, 0x00000001);
static const FUID controllerID(0x4C564242, 0x45544131,
    LVB_BETA_INSTRUMENT ? 0x494E5354 : 0x45464658, 0x00000002);
static constexpr double tau = 6.2831853071795864769;
struct Settings { double gain = .25, colour = .5; };
static bool normalized(double value) {
    return std::isfinite(value) && value >= 0. && value <= 1.;
}
static tresult state(IBStream* stream, Settings& settings, bool write) {
    if (!stream) return kInvalidArgument;
    // Fixed little-endian 24-byte state, including exact fixture role/version.
    std::array<uint8, 24> bytes{ 'L', 'V', 'B', 'B', 1, LVB_BETA_INSTRUMENT, 0, 0 };
    int32 count = 0;
    if (write) {
        std::memcpy(bytes.data() + 8, &settings.gain, 8);
        std::memcpy(bytes.data() + 16, &settings.colour, 8);
        return stream->write(bytes.data(), int32(bytes.size()), &count) == kResultOk
            && count == int32(bytes.size()) ? kResultOk : kResultFalse;
    }
    const auto header = bytes;
    if (stream->read(bytes.data(), int32(bytes.size()), &count) != kResultOk
        || count != int32(bytes.size())
        || !std::equal(bytes.begin(), bytes.begin() + 8, header.begin())) return kResultFalse;
    Settings candidate;
    std::memcpy(&candidate.gain, bytes.data() + 8, 8);
    std::memcpy(&candidate.colour, bytes.data() + 16, 8);
    if (!normalized(candidate.gain) || !normalized(candidate.colour)) return kResultFalse;
    settings = candidate;
    return kResultOk;
}
class ReferenceProcessor final : public AudioEffect {
    Settings settings;
    struct Voice { bool active = false; int32 id = -1; int16 pitch = 0, channel = 0;
        double phase = 0., increment = 0., velocity = 0.; };
    std::array<Voice, 16> voices{};
    void note(const Event& event) {
        if (event.type == Event::kNoteOnEvent) {
            auto found = std::find_if(voices.begin(), voices.end(), [](const Voice& v) { return !v.active; });
            if (found == voices.end()) return; // Explicit bounded 16-voice fixture.
            const auto& n = event.noteOn;
            if (n.pitch < 0 || n.pitch > 127 || !std::isfinite(n.velocity)) return;
            *found = Voice{true, n.noteId, n.pitch, n.channel, 0.,
                tau * 440. * std::exp2((n.pitch - 69.) / 12.) / processSetup.sampleRate,
                std::clamp(double(n.velocity), 0., 1.)};
        } else if (event.type == Event::kNoteOffEvent) {
            const auto& n = event.noteOff;
            for (auto& voice : voices) if (voice.active && voice.channel == n.channel
                && (n.noteId >= 0 ? voice.id == n.noteId : voice.pitch == n.pitch)) voice.active = false;
        }
    }
public:
    ReferenceProcessor() { setControllerClass(controllerID); }
    static FUnknown* create(void*) { return static_cast<IAudioProcessor*>(new ReferenceProcessor); }
    tresult PLUGIN_API initialize(FUnknown* host) override {
        auto result = AudioEffect::initialize(host);
        if (result != kResultOk) return result;
        if (!LVB_BETA_INSTRUMENT) addAudioInput(STR16("Stereo input"), SpeakerArr::kStereo);
        addAudioOutput(STR16("Stereo output"), SpeakerArr::kStereo);
        if (LVB_BETA_INSTRUMENT) addEventInput(STR16("Notes"), 16);
        return kResultOk;
    }
    tresult PLUGIN_API setBusArrangements(SpeakerArrangement* in, int32 ni,
        SpeakerArrangement* out, int32 no) override {
        if (ni != (LVB_BETA_INSTRUMENT ? 0 : 1) || no != 1 || !out
            || out[0] != SpeakerArr::kStereo
            || (!LVB_BETA_INSTRUMENT && (!in || in[0] != SpeakerArr::kStereo))) return kResultFalse;
        return AudioEffect::setBusArrangements(in, ni, out, no);
    }
    tresult PLUGIN_API canProcessSampleSize(int32 size) override {
        return size == kSample32 ? kResultTrue : kResultFalse;
    }
    tresult PLUGIN_API setupProcessing(ProcessSetup& setup) override {
        if (!std::isfinite(setup.sampleRate) || setup.sampleRate < 8000.
            || setup.sampleRate > 384000. || setup.maxSamplesPerBlock < 0) return kResultFalse;
        return AudioEffect::setupProcessing(setup);
    }
    tresult PLUGIN_API setActive(TBool active) override {
        if (!active) voices.fill(Voice{});
        return AudioEffect::setActive(active);
    }
    tresult PLUGIN_API getState(IBStream* stream) override { return state(stream, settings, true); }
    tresult PLUGIN_API setState(IBStream* stream) override { return state(stream, settings, false); }
    tresult PLUGIN_API process(ProcessData& data) override {
        if (data.symbolicSampleSize != kSample32 || data.numSamples < 0
            || data.numSamples > processSetup.maxSamplesPerBlock) return kInvalidArgument;
        // Apply points at their declared sample. Bounded SDK queues, no heap.
        const auto parameters = data.inputParameterChanges;
        const auto events = data.inputEvents;
        const auto eventCount = events ? events->getEventCount() : 0;
        const int32 queueCount = parameters ? parameters->getParameterCount() : 0;
        if (eventCount < 0 || eventCount > 256 || queueCount < 0 || queueCount > 2) return kInvalidArgument;
        struct Point { IParamValueQueue* queue = nullptr; int32 index = 0, count = 0, offset = 0; double value = 0.; };
        std::array<Point, 2> points{};
        for (int32 q = 0; q < queueCount; ++q) {
            auto& point = points[q];
            point.queue = parameters->getParameterData(q);
            if (!point.queue) return kInvalidArgument;
            point.count = point.queue->getPointCount();
            if (point.count < 0 || point.count > 256) return kInvalidArgument;
            if (point.count && (point.queue->getPoint(0, point.offset, point.value) != kResultOk
                || point.offset < 0 || !normalized(point.value))) return kInvalidArgument;
        }
        for (int32 sample = 0; sample < std::max(1, data.numSamples); ++sample) {
            for (int32 q = 0; q < queueCount; ++q) {
                auto& point = points[q];
                while (point.index < point.count && point.offset == sample) {
                    if (point.queue->getParameterId() == 0) settings.gain = point.value;
                    if (point.queue->getParameterId() == 1) settings.colour = point.value;
                    if (++point.index < point.count) {
                        if (point.queue->getPoint(point.index, point.offset, point.value) != kResultOk
                            || point.offset < sample || !normalized(point.value)) return kInvalidArgument;
                    }
                }
            }
            if (!data.numSamples) break; // State/parameter flush, no samples.
            if (data.numOutputs != 1 || !data.outputs || data.outputs[0].numChannels != 2
                || !data.outputs[0].channelBuffers32 || !data.outputs[0].channelBuffers32[0]
                || !data.outputs[0].channelBuffers32[1]) return kInvalidArgument;
            double signal = 0.;
            if (LVB_BETA_INSTRUMENT) {
                for (int32 i = 0; i < eventCount; ++i) {
                    Event event{};
                    if (events->getEvent(i, event) != kResultOk) return kInvalidArgument;
                    if (event.sampleOffset == sample && event.busIndex == 0) note(event);
                }
                for (auto& voice : voices) if (voice.active) {
                    signal += voice.velocity * (std::sin(voice.phase)
                        + settings.colour * .5 * std::sin(voice.phase * 2.));
                    voice.phase = std::fmod(voice.phase + voice.increment, tau);
                }
            } else if (data.numInputs != 1 || !data.inputs || data.inputs[0].numChannels != 2
                || !data.inputs[0].channelBuffers32
                || (!data.inputs[0].channelBuffers32[0] && !(data.inputs[0].silenceFlags & 1))
                || (!data.inputs[0].channelBuffers32[1] && !(data.inputs[0].silenceFlags & 2))) return kInvalidArgument;
            for (int ch = 0; ch < 2; ++ch) {
                const auto input = LVB_BETA_INSTRUMENT ? signal
                    : (data.inputs[0].silenceFlags & (1ull << ch)) ? 0.
                    : data.inputs[0].channelBuffers32[ch][sample];
                data.outputs[0].channelBuffers32[ch][sample] = float(settings.gain
                    * (LVB_BETA_INSTRUMENT ? input / 16. : input * (.5 + settings.colour)));
            }
        }
        if (data.numSamples) data.outputs[0].silenceFlags = 0;
        return kResultOk;
    }
};
class ReferenceController final : public EditControllerEx1 {
public:
    static FUnknown* create(void*) { return static_cast<IEditController*>(new ReferenceController); }
    tresult PLUGIN_API initialize(FUnknown* host) override {
        auto result = EditControllerEx1::initialize(host);
        if (result != kResultOk) return result;
        parameters.addParameter(STR16("Level"), nullptr, 0, .25, ParameterInfo::kCanAutomate, 0);
        parameters.addParameter(STR16("Colour"), nullptr, 0, .5, ParameterInfo::kCanAutomate, 1);
        return kResultOk;
    }
    tresult PLUGIN_API setComponentState(IBStream* stream) override {
        Settings settings;
        auto result = state(stream, settings, false);
        if (result != kResultOk) return result;
        if (setParamNormalized(0, settings.gain) != kResultOk) return kResultFalse;
        return setParamNormalized(1, settings.colour);
    }
    tresult PLUGIN_API getState(IBStream* stream) override {
        Settings settings{getParamNormalized(0), getParamNormalized(1)};
        return state(stream, settings, true);
    }
    tresult PLUGIN_API setState(IBStream* stream) override { return setComponentState(stream); }
};
BEGIN_FACTORY_DEF("Linux VST Bridge", "https://github.com/kasselvania/Linux-VST-bridge", "")
DEF_CLASS2(INLINE_UID_FROM_FUID(processorID), PClassInfo::kManyInstances, kVstAudioEffectClass,
    LVB_BETA_INSTRUMENT ? "LVB Reference Instrument" : "LVB Reference Effect", Vst::kDistributable,
    LVB_BETA_INSTRUMENT ? "Instrument|Synth" : "Fx", "1.0.0", kVstVersionString, ReferenceProcessor::create)
DEF_CLASS2(INLINE_UID_FROM_FUID(controllerID), PClassInfo::kManyInstances, kVstComponentControllerClass,
    "LVB Reference Controller", 0, "", "1.0.0", kVstVersionString, ReferenceController::create)
END_FACTORY
