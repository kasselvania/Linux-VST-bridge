#include "processor.h"

#include "lab_host_probe/ids.h"

#include "pluginterfaces/vst/ivstparameterchanges.h"

#include <algorithm>
#include <cstring>

namespace Kasselvania::LabHostProbe {

using namespace Steinberg;
using namespace Steinberg::Vst;

Processor::Processor ()
{
    setControllerClass (kControllerClassId);
}

tresult PLUGIN_API Processor::initialize (FUnknown* context)
{
    const auto result = AudioEffect::initialize (context);
    if (result != kResultOk)
        return result;

    addAudioInput (STR16 ("Stereo Input"), SpeakerArr::kStereo);
    addAudioOutput (STR16 ("Stereo Output"), SpeakerArr::kStereo);
    return kResultOk;
}

tresult PLUGIN_API Processor::setBusArrangements (SpeakerArrangement* inputs, int32 numInputs,
                                                  SpeakerArrangement* outputs, int32 numOutputs)
{
    if (!inputs || !outputs || numInputs != 1 || numOutputs != 1 ||
        inputs[0] != SpeakerArr::kStereo || outputs[0] != SpeakerArr::kStereo)
        return kResultFalse;
    return AudioEffect::setBusArrangements (inputs, numInputs, outputs, numOutputs);
}

tresult PLUGIN_API Processor::canProcessSampleSize (int32 symbolicSampleSize)
{
    return symbolicSampleSize == kSample32 || symbolicSampleSize == kSample64 ? kResultTrue
                                                                              : kResultFalse;
}

void Processor::applyParameterChanges (IParameterChanges* changes) noexcept
{
    if (!changes)
        return;

    const int32 parameterCount = changes->getParameterCount ();
    for (int32 parameterIndex = 0; parameterIndex < parameterCount; ++parameterIndex)
    {
        auto* queue = changes->getParameterData (parameterIndex);
        if (!queue || queue->getPointCount () <= 0)
            continue;

        int32 sampleOffset = 0;
        ParamValue value = 0.0;
        if (queue->getPoint (queue->getPointCount () - 1, sampleOffset, value) != kResultTrue)
            continue;

        switch (queue->getParameterId ())
        {
            case kGainParameterId:
                state_.gain = clampNormalized (value);
                break;
            case kBypassParameterId:
                state_.bypass = value >= 0.5;
                break;
            default:
                break;
        }
    }
}

template <typename Sample>
tresult Processor::processAudio (ProcessData& data) noexcept
{
    if (data.numSamples <= 0)
        return kResultOk;
    if (data.numInputs < 1 || data.numOutputs < 1 || !data.inputs || !data.outputs)
        return kResultOk;

    auto& input = data.inputs[0];
    auto& output = data.outputs[0];
    const int32 outputChannels = std::max<int32> (0, output.numChannels);
    const int32 processChannels = std::min<int32> ({2, std::max<int32> (0, input.numChannels),
                                                    outputChannels});

    Sample** inputBuffers = nullptr;
    Sample** outputBuffers = nullptr;
    if constexpr (sizeof (Sample) == sizeof (Sample32))
    {
        inputBuffers = reinterpret_cast<Sample**> (input.channelBuffers32);
        outputBuffers = reinterpret_cast<Sample**> (output.channelBuffers32);
    }
    else
    {
        inputBuffers = reinterpret_cast<Sample**> (input.channelBuffers64);
        outputBuffers = reinterpret_cast<Sample**> (output.channelBuffers64);
    }

    if (!outputBuffers)
        return kResultOk;

    uint64 silenceFlags = 0;
    const auto sampleCount = static_cast<size_t> (data.numSamples);
    const Sample gain = static_cast<Sample> (state_.gain);

    for (int32 channel = 0; channel < outputChannels; ++channel)
    {
        auto* outputBuffer = outputBuffers[channel];
        const bool hasInput = channel < processChannels && inputBuffers && inputBuffers[channel];
        const bool inputSilent = channel < 64 && (input.silenceFlags & (uint64 {1} << channel)) != 0;

        if (!outputBuffer)
        {
            if (channel < 64)
                silenceFlags |= uint64 {1} << channel;
            continue;
        }

        if (!hasInput || inputSilent || (!state_.bypass && gain == Sample {0}))
        {
            std::fill_n (outputBuffer, sampleCount, Sample {0});
            if (channel < 64)
                silenceFlags |= uint64 {1} << channel;
            continue;
        }

        const auto* inputBuffer = inputBuffers[channel];
        if (state_.bypass)
        {
            if (inputBuffer != outputBuffer)
                std::memcpy (outputBuffer, inputBuffer, sampleCount * sizeof (Sample));
            continue;
        }

        for (size_t sample = 0; sample < sampleCount; ++sample)
            outputBuffer[sample] = inputBuffer[sample] * gain;
    }

    output.silenceFlags = silenceFlags;
    return kResultOk;
}

tresult PLUGIN_API Processor::process (ProcessData& data)
{
    applyParameterChanges (data.inputParameterChanges);
    if (data.symbolicSampleSize == kSample32)
        return processAudio<Sample32> (data);
    if (data.symbolicSampleSize == kSample64)
        return processAudio<Sample64> (data);
    return kResultFalse;
}

tresult PLUGIN_API Processor::setState (IBStream* stream)
{
    State restored;
    const auto result = readState (stream, kProcessorStateMagic, restored);
    if (result == kResultOk)
        state_ = restored;
    return result;
}

tresult PLUGIN_API Processor::getState (IBStream* stream)
{
    return writeState (stream, kProcessorStateMagic, state_);
}

template tresult Processor::processAudio<Sample32> (ProcessData&) noexcept;
template tresult Processor::processAudio<Sample64> (ProcessData&) noexcept;

} // namespace Kasselvania::LabHostProbe
