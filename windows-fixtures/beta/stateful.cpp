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
#ifndef LVB_BETA_COMPLETION
#define LVB_BETA_COMPLETION 0
#endif
#if LVB_BETA_COMPLETION
#include <chrono>
#include <thread>
#endif

using namespace Steinberg;
using namespace Steinberg::Vst;
#ifndef LVB_BETA_INSTRUMENT
#define LVB_BETA_INSTRUMENT 0
#endif
#ifndef LVB_BETA_STATE_VERSION
#define LVB_BETA_STATE_VERSION 1
#endif
#ifndef LVB_BETA_RESTORE_POLICY
#define LVB_BETA_RESTORE_POLICY 0
#endif
static constexpr bool evolved = LVB_BETA_STATE_VERSION == 2;
static constexpr int fixtureAudioOutputCount = LVB_BETA_COMPLETION ? 2 : 1;
static const FUID processorID(0x4C564242, LVB_BETA_COMPLETION ? 0x434D5031 : 0x45544131,
    LVB_BETA_INSTRUMENT ? 0x494E5354 : 0x45464658, 0x00000001);
static const FUID controllerID(0x4C564242, LVB_BETA_COMPLETION ? 0x434D5031 : 0x45544131,
    LVB_BETA_INSTRUMENT ? 0x494E5354 : 0x45464658, 0x00000002);
static constexpr double tau = 6.2831853071795864769;
struct Settings { double gain = .25, colour = .5, trim = .25; };
static bool normalized(double value) {
    return std::isfinite(value) && value >= 0. && value <= 1.;
}
static tresult state(IBStream* stream, Settings& settings, bool write) {
    if (!stream) return kInvalidArgument;
    // Revision 2 deliberately changes the layout and adds a parameter. Only
    // the vendor fixture interprets/migrates the earlier opaque schema.
    std::array<uint8, 32> bytes{ 'L', 'V', 'B', 'B', LVB_BETA_STATE_VERSION, LVB_BETA_INSTRUMENT, 0, 0 };
    int32 count = 0;
    if (write) {
        std::memcpy(bytes.data() + 8, evolved ? &settings.trim : &settings.gain, 8);
        std::memcpy(bytes.data() + 16, &settings.colour, 8);
        if(evolved) std::memcpy(bytes.data() + 24, &settings.gain, 8);
        const int32 size=evolved?32:24;
        return stream->write(bytes.data(), size, &count) == kResultOk
            && count == size ? kResultOk : kResultFalse;
    }
    const auto header = bytes;
    if (stream->read(bytes.data(), 8, &count) != kResultOk || count != 8
        || !std::equal(bytes.begin(), bytes.begin() + 4, header.begin())
        || bytes[5]!=LVB_BETA_INSTRUMENT || bytes[6] || bytes[7]
        || (bytes[4]!=1 && !(evolved&&bytes[4]==2))) return kResultFalse;
    const int32 remaining=bytes[4]==2?24:16;
    if(stream->read(bytes.data()+8,remaining,&count)!=kResultOk||count!=remaining) return kResultFalse;
    Settings candidate;
    std::memcpy(&candidate.gain, bytes.data() + (bytes[4]==2?24:8), 8);
    std::memcpy(&candidate.colour, bytes.data() + 16, 8);
    if(bytes[4]==2) std::memcpy(&candidate.trim,bytes.data()+8,8);
    else if(evolved) candidate.trim=candidate.gain+candidate.colour;
    if (!normalized(candidate.gain) || !normalized(candidate.colour) || !normalized(candidate.trim)) return kResultFalse;
    settings = candidate;
    return kResultOk;
}
class ReferenceProcessor final : public AudioEffect {
    Settings settings;
#if LVB_BETA_COMPLETION
    std::array<std::array<float, 13>, 2> latency{};
    size_t latencyCursor = 0;
    bool delayCapture = false;
#endif
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
        if (LVB_BETA_COMPLETION) addAudioOutput(STR16("Auxiliary stereo output"), SpeakerArr::kStereo,
                                               kAux, BusInfo::kDefaultActive);
        if (LVB_BETA_INSTRUMENT || LVB_BETA_COMPLETION) addEventInput(STR16("Notes"), 16);
        if (LVB_BETA_COMPLETION) addEventOutput(STR16("Fixture results"), 16);
        return kResultOk;
    }
    tresult PLUGIN_API setBusArrangements(SpeakerArrangement* in, int32 ni,
        SpeakerArrangement* out, int32 no) override {
        if (ni != (LVB_BETA_INSTRUMENT ? 0 : 1) || no != fixtureAudioOutputCount || !out
            || out[0] != SpeakerArr::kStereo
            || (LVB_BETA_COMPLETION && out[1] != SpeakerArr::kStereo)
            || (!LVB_BETA_INSTRUMENT && (!in || in[0] != SpeakerArr::kStereo))) return kResultFalse;
        return AudioEffect::setBusArrangements(in, ni, out, no);
    }
    tresult PLUGIN_API canProcessSampleSize(int32 size) override {
        return size == kSample32 ? kResultTrue : kResultFalse;
    }
    tresult PLUGIN_API setupProcessing(ProcessSetup& setup) override {
        if (!std::isfinite(setup.sampleRate) || setup.sampleRate < 8000.
            || setup.sampleRate > 384000. || setup.maxSamplesPerBlock < 0) return kResultFalse;
        if (LVB_BETA_COMPLETION && (setup.maxSamplesPerBlock > 1024
            || setup.symbolicSampleSize != kSample32 || setup.processMode < kRealtime
            || setup.processMode > kOffline)) return kResultFalse;
        return AudioEffect::setupProcessing(setup);
    }
    tresult PLUGIN_API setActive(TBool active) override {
        if (!active) voices.fill(Voice{});
#if LVB_BETA_COMPLETION
        if (!active) { latency = {}; latencyCursor = 0; }
#endif
        return AudioEffect::setActive(active);
    }
#if LVB_BETA_COMPLETION
    // All render storage is prepared during construction/setup. The explicit
    // processing transition is lightweight; the SDK base returns not implemented.
    tresult PLUGIN_API setProcessing(TBool) override { return kResultOk; }
#endif
    uint32 PLUGIN_API getLatencySamples() override { return LVB_BETA_COMPLETION ? 13 : 0; }
    uint32 PLUGIN_API getTailSamples() override { return LVB_BETA_COMPLETION ? 13 : 0; }
    tresult PLUGIN_API getState(IBStream* stream) override {
#if LVB_BETA_COMPLETION
        if (delayCapture) {
            delayCapture = false;
            std::this_thread::sleep_for(std::chrono::milliseconds(200));
        }
#endif
        return state(stream, settings, true);
    }
    tresult PLUGIN_API setState(IBStream* stream) override {
        if constexpr(LVB_BETA_RESTORE_POLICY==1) {
            int64 position=0;uint8 header[8]{};int32 read=0;
            if(!stream||stream->tell(&position)!=kResultOk||stream->read(header,8,&read)!=kResultOk
                ||read!=8||stream->seek(position,IBStream::kIBSeekSet,nullptr)!=kResultOk) return kResultFalse;
            if(header[4]==1) return kResultFalse; // explicit vendor migration refusal
        }
        return state(stream, settings, false);
    }
    tresult PLUGIN_API process(ProcessData& data) override {
        if (data.symbolicSampleSize != kSample32 || data.numSamples < 0
            || data.numSamples > processSetup.maxSamplesPerBlock) return kInvalidArgument;
        if (LVB_BETA_COMPLETION && (data.processMode < kRealtime || data.processMode > kOffline
            || ((data.processMode == kOffline) != (processSetup.processMode == kOffline)))) return kResultFalse;
        // Apply points at their declared sample. Bounded SDK queues, no heap.
        const auto parameters = data.inputParameterChanges;
        const auto events = data.inputEvents;
        const auto eventCount = events ? events->getEventCount() : 0;
        const int32 queueCount = parameters ? parameters->getParameterCount() : 0;
        if (eventCount < 0 || eventCount > 256 || queueCount < 0
            || queueCount > (evolved?3:2) + (LVB_BETA_COMPLETION?2:0)) return kInvalidArgument;
        struct Point { IParamValueQueue* queue = nullptr; int32 index = 0, count = 0, offset = 0; double value = 0.; };
        std::array<Point, LVB_BETA_COMPLETION ? 5 : 3> points{};
        for (int32 q = 0; q < queueCount; ++q) {
            auto& point = points[q];
            point.queue = parameters->getParameterData(q);
            if (!point.queue) return kInvalidArgument;
            point.count = point.queue->getPointCount();
            if (point.count < 0 || point.count > 256) return kInvalidArgument;
            if (point.count && (point.queue->getPoint(0, point.offset, point.value) != kResultOk
                || point.offset < 0 || !normalized(point.value))) return kInvalidArgument;
        }
#if LVB_BETA_COMPLETION
        // A declared transient fixture operation, not an environment/runtime
        // override. Slow work is legal only under actual offline processing.
        for (int32 q = 0; q < queueCount; ++q) if (points[q].queue->getParameterId() == 31) {
            const auto& point = points[q];
            if (point.count != 1 || point.offset != 0) return kInvalidArgument;
            const int operation = int(std::lround(point.value * 4.));
            if (point.value != double(operation) / 4.) return kInvalidArgument;
            if (operation == 4) delayCapture = true;
            if (operation == 2) return kResultFalse;
            if (operation == 1 || operation == 3) {
                if (data.processMode != kOffline) return kResultFalse;
                std::this_thread::sleep_for(std::chrono::seconds(operation == 1 ? 12 : 65));
            }
        }
#endif
        for (int32 sample = 0; sample < std::max(1, data.numSamples); ++sample) {
            for (int32 q = 0; q < queueCount; ++q) {
                auto& point = points[q];
                while (point.index < point.count && point.offset == sample) {
                    if (point.queue->getParameterId() == 0) settings.gain = point.value;
                    if (point.queue->getParameterId() == 1) settings.colour = point.value;
                    if (evolved && point.queue->getParameterId() == 17) settings.trim = point.value;
                    if (++point.index < point.count) {
                        if (point.queue->getPoint(point.index, point.offset, point.value) != kResultOk
                            || point.offset < sample || !normalized(point.value)) return kInvalidArgument;
                    }
                }
            }
            if (!data.numSamples) break; // State/parameter flush, no samples.
            if (data.numOutputs != fixtureAudioOutputCount || !data.outputs || data.outputs[0].numChannels != 2
                || !data.outputs[0].channelBuffers32 || !data.outputs[0].channelBuffers32[0]
                || !data.outputs[0].channelBuffers32[1]) return kInvalidArgument;
            if (LVB_BETA_COMPLETION && (data.outputs[1].numChannels != 2
                || !data.outputs[1].channelBuffers32 || !data.outputs[1].channelBuffers32[0]
                || !data.outputs[1].channelBuffers32[1])) return kInvalidArgument;
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
                float value = float(settings.gain * (evolved ? .75 + settings.trim : 1.)
                    * (LVB_BETA_INSTRUMENT ? input / 16. : input * (.5 + settings.colour)));
#if LVB_BETA_COMPLETION
                const float delayed = latency[ch][latencyCursor];
                latency[ch][latencyCursor] = value;
                value = delayed;
                data.outputs[1].channelBuffers32[ch][sample] = value * (ch ? -.5f : .5f);
#endif
                data.outputs[0].channelBuffers32[ch][sample] = value;
            }
#if LVB_BETA_COMPLETION
            latencyCursor = (latencyCursor + 1) % 13;
#endif
        }
        if (data.numSamples) for (int bus = 0; bus < fixtureAudioOutputCount; ++bus) data.outputs[bus].silenceFlags = 0;
#if LVB_BETA_COMPLETION
        if (data.outputParameterChanges) {
            int32 index = 0;
            auto* mode = data.outputParameterChanges->addParameterData(32, index);
            if (!mode || mode->addPoint(0, double(data.processMode) / 2., index) != kResultOk) return kResultFalse;
            auto* gain = data.outputParameterChanges->addParameterData(0, index);
            if (!gain || gain->addPoint(std::max(0, data.numSamples - 1), settings.gain, index) != kResultOk) return kResultFalse;
        }
        if (data.outputEvents) {
            // Offset zero is valid for this declared non-note result on a
            // zero-frame operation; zero-frame input notes remain unsupported.
            Event result{};
            result.type = Event::kLegacyMIDICCOutEvent;
            result.midiCCOut = {74, 2, int8(data.processMode), 0};
            bool published = false;
            for (int32 i = 0; i < eventCount; ++i) {
                Event event{};
                if (events->getEvent(i, event) != kResultOk) return kResultFalse;
                // Keep returned events in sample order, preserving the host's
                // order for input notes at the same position.
                if (!published && event.sampleOffset > 0) {
                    if (data.outputEvents->addEvent(result) != kResultOk) return kResultFalse;
                    published = true;
                }
                if (data.outputEvents->addEvent(event) != kResultOk) return kResultFalse;
            }
            if (!published && data.outputEvents->addEvent(result) != kResultOk) return kResultFalse;
        }
#endif
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
        if(evolved) parameters.addParameter(STR16("Trim"), nullptr, 0, .25, ParameterInfo::kCanAutomate, 17);
        if (LVB_BETA_COMPLETION) {
            parameters.addParameter(STR16("Fixture operation"), nullptr, 4, 0., ParameterInfo::kCanAutomate, 31);
            parameters.addParameter(STR16("Observed process mode"), nullptr, 2, 0., ParameterInfo::kIsReadOnly, 32);
        }
        return kResultOk;
    }
    tresult PLUGIN_API setComponentState(IBStream* stream) override {
        Settings settings;
        auto result = state(stream, settings, false);
        if (result != kResultOk) return result;
        if (setParamNormalized(0, settings.gain) != kResultOk) return kResultFalse;
        if(setParamNormalized(1, settings.colour)!=kResultOk) return kResultFalse;
        return evolved?setParamNormalized(17,settings.trim):kResultOk;
    }
    tresult PLUGIN_API getState(IBStream* stream) override {
        Settings settings{getParamNormalized(0), getParamNormalized(1),evolved?getParamNormalized(17):.25};
        return state(stream, settings, true);
    }
    tresult PLUGIN_API setState(IBStream* stream) override {
        if constexpr(LVB_BETA_RESTORE_POLICY==2) {
            int64 position=0;uint8 header[8]{};int32 read=0;
            if(!stream||stream->tell(&position)!=kResultOk||stream->read(header,8,&read)!=kResultOk
                ||read!=8||stream->seek(position,IBStream::kIBSeekSet,nullptr)!=kResultOk) return kResultFalse;
            if(header[4]==1) return kResultFalse; // component already migrated: partial restore
        }
        return setComponentState(stream);
    }
};
BEGIN_FACTORY_DEF("Linux VST Bridge", "https://github.com/kasselvania/Linux-VST-bridge", "")
DEF_CLASS2(INLINE_UID_FROM_FUID(processorID), PClassInfo::kManyInstances, kVstAudioEffectClass,
    LVB_BETA_COMPLETION ? (LVB_BETA_INSTRUMENT ? "LVB Completion Instrument" : "LVB Completion Effect")
                        : (LVB_BETA_INSTRUMENT ? "LVB Reference Instrument" : "LVB Reference Effect"), Vst::kDistributable,
    LVB_BETA_INSTRUMENT ? "Instrument|Synth" : "Fx", "1.0.0", kVstVersionString, ReferenceProcessor::create)
DEF_CLASS2(INLINE_UID_FROM_FUID(controllerID), PClassInfo::kManyInstances, kVstComponentControllerClass,
    LVB_BETA_COMPLETION ? "LVB Completion Controller" : "LVB Reference Controller", 0, "", "1.0.0", kVstVersionString, ReferenceController::create)
END_FACTORY
