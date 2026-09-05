#include "processor.h"
#include "public.sdk/source/main/pluginfactory_constexpr.h"
namespace AP2 {
// Exact retained Windows AGain processor. The private bundle has a deliberately
// limited label; it is never published alongside a DAW installation.
static constexpr Steinberg::TUID processorID =
    INLINE_UID(0x84E8DE5F, 0x92554F53, 0x96FAE413, 0x3C935A18);
} // namespace AP2
BEGIN_FACTORY_DEF("Kasselvania Research",
                  "https://github.com/kasselvania/Linux-VST-bridge", "", 1)
DEF_CLASS(AP2::processorID, 1, kVstAudioEffectClass, "AGain Offline Bridge", 0,
          "Fx|OnlyOfflineProcess", "0.1.0", kVstVersionString,
          AP2::Processor::create, nullptr)
END_FACTORY
