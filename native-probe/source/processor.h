#pragma once

#include "public.sdk/source/vst/vstaudioeffect.h"

#include "lab_host_probe/state.h"

namespace Kasselvania::LabHostProbe {

class Processor final : public Steinberg::Vst::AudioEffect
{
public:
    Processor ();

    static Steinberg::FUnknown* createInstance (void*)
    {
        return static_cast<Steinberg::Vst::IAudioProcessor*> (new Processor ());
    }

    Steinberg::tresult PLUGIN_API initialize (Steinberg::FUnknown* context) override;
    Steinberg::tresult PLUGIN_API setBusArrangements (
        Steinberg::Vst::SpeakerArrangement* inputs, Steinberg::int32 numInputs,
        Steinberg::Vst::SpeakerArrangement* outputs, Steinberg::int32 numOutputs) override;
    Steinberg::tresult PLUGIN_API canProcessSampleSize (Steinberg::int32 symbolicSampleSize) override;
    Steinberg::tresult PLUGIN_API process (Steinberg::Vst::ProcessData& data) override;
    Steinberg::tresult PLUGIN_API setState (Steinberg::IBStream* stream) override;
    Steinberg::tresult PLUGIN_API getState (Steinberg::IBStream* stream) override;

private:
    void applyParameterChanges (Steinberg::Vst::IParameterChanges* changes) noexcept;

    template <typename Sample>
    Steinberg::tresult processAudio (Steinberg::Vst::ProcessData& data) noexcept;

    State state_ {};
};

} // namespace Kasselvania::LabHostProbe
