#pragma once

#include "pluginterfaces/base/funknown.h"
#include "pluginterfaces/vst/vsttypes.h"

namespace Kasselvania::LabHostProbe {

static DECLARE_UID (kProcessorClassId, 0x6F4E7A53, 0x92E54B54, 0xA98AD6F7, 0x14E0C201);
static DECLARE_UID (kControllerClassId, 0xB9C42F07, 0x36C34E21, 0x8E5A71D4, 0x0C8F1B62);

enum ParameterId : Steinberg::Vst::ParamID
{
    kGainParameterId = 0x4C485001,
    kBypassParameterId = 0x4C485002,
};

} // namespace Kasselvania::LabHostProbe
