#include "processor.h"
#include "public.sdk/source/main/pluginfactory_constexpr.h"
#ifdef AP3_PREVIEW
#include "public.sdk/source/vst/vsteditcontroller.h"
namespace AP2 {
class Controller final : public Steinberg::Vst::EditController {
public:
  static Steinberg::FUnknown *create(void *) {
    try {
      return static_cast<Steinberg::Vst::IEditController *>(new Controller);
    } catch (...) {
      return nullptr;
    }
  }
  Steinberg::tresult PLUGIN_API
  initialize(Steinberg::FUnknown *context) override {
    auto r = EditController::initialize(context);
    if (r != Steinberg::kResultOk)
      return r;
    // Retained AGain ParamID 0, normalized linear gain, default 1.0. The host
    // delivers changes through ProcessData; this controller never contacts DSP.
    parameters.addParameter(STR16("Gain"), STR16(""), 0, 1.,
                            Steinberg::Vst::ParameterInfo::kCanAutomate, 0);
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API
  setComponentState(Steinberg::IBStream *) override {
    return Steinberg::kNotImplemented;
  }
  Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream *) override {
    return Steinberg::kNotImplemented;
  }
  Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream *) override {
    return Steinberg::kNotImplemented;
  }
};
static constexpr Steinberg::TUID controlID =
    INLINE_UID(0xD1444DE3, 0x38814391, 0xA916DC9C, 0xFCC67008);
} // namespace AP2
#endif
namespace AP2 {
// Exact retained Windows AGain processor. The private bundle has a deliberately
// limited label; it is never published alongside a DAW installation.
static constexpr Steinberg::TUID processorID =
    INLINE_UID(0x84E8DE5F, 0x92554F53, 0x96FAE413, 0x3C935A18);
} // namespace AP2
#ifdef AP3_PREVIEW
BEGIN_FACTORY_DEF("Kasselvania Research",
                  "https://github.com/kasselvania/Linux-VST-bridge", "", 2)
DEF_CLASS(AP2::processorID, 1, kVstAudioEffectClass, "AGain Queued Preview", 0,
          "Fx", "0.2.0", kVstVersionString, AP2::Processor::create, nullptr)
DEF_CLASS(AP2::controlID, 1, kVstComponentControllerClass,
          "Bridge Reference Gain", 0, "", "0.2.0", kVstVersionString,
          AP2::Controller::create, nullptr)
#else
BEGIN_FACTORY_DEF("Kasselvania Research",
                  "https://github.com/kasselvania/Linux-VST-bridge", "", 1)
DEF_CLASS(AP2::processorID, 1, kVstAudioEffectClass, "AGain Offline Bridge", 0,
          "Fx|OnlyOfflineProcess", "0.1.0", kVstVersionString,
          AP2::Processor::create, nullptr)
#endif
END_FACTORY
