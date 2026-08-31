#pragma once

#include "base/source/fstreamer.h"
#include "pluginterfaces/base/ibstream.h"

#include <cmath>

namespace Kasselvania::LabHostProbe {

constexpr Steinberg::int32 kProcessorStateMagic = 0x4C485030; // LHP0
constexpr Steinberg::int32 kControllerStateMagic = 0x4C484330; // LHC0
constexpr Steinberg::int32 kStateVersion = 1;
constexpr Steinberg::Vst::ParamValue kDefaultGain = 0.5;

struct State
{
    Steinberg::Vst::ParamValue gain {kDefaultGain};
    bool bypass {false};
};

inline Steinberg::Vst::ParamValue clampNormalized (Steinberg::Vst::ParamValue value) noexcept
{
    if (!std::isfinite (value) || value <= 0.0)
        return 0.0;
    if (value >= 1.0)
        return 1.0;
    return value;
}

inline Steinberg::tresult writeState (Steinberg::IBStream* stream, Steinberg::int32 magic,
                                      const State& state)
{
    if (!stream)
        return Steinberg::kInvalidArgument;

    Steinberg::IBStreamer writer (stream, kLittleEndian);
    if (!writer.writeInt32 (magic) || !writer.writeInt32 (kStateVersion) ||
        !writer.writeDouble (clampNormalized (state.gain)) ||
        !writer.writeInt32 (state.bypass ? 1 : 0))
        return Steinberg::kResultFalse;
    return Steinberg::kResultOk;
}

inline Steinberg::tresult readState (Steinberg::IBStream* stream, Steinberg::int32 expectedMagic,
                                     State& state)
{
    if (!stream)
        return Steinberg::kInvalidArgument;

    Steinberg::IBStreamer reader (stream, kLittleEndian);
    Steinberg::int32 magic = 0;
    Steinberg::int32 version = 0;
    double gain = 0.0;
    Steinberg::int32 bypass = 0;
    if (!reader.readInt32 (magic) || !reader.readInt32 (version) || !reader.readDouble (gain) ||
        !reader.readInt32 (bypass))
        return Steinberg::kResultFalse;
    if (magic != expectedMagic || version != kStateVersion || !std::isfinite (gain) || gain < 0.0 ||
        gain > 1.0 || (bypass != 0 && bypass != 1))
        return Steinberg::kResultFalse;

    state.gain = gain;
    state.bypass = bypass == 1;
    return Steinberg::kResultOk;
}

} // namespace Kasselvania::LabHostProbe
