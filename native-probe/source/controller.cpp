#include "controller.h"

#include "lab_host_probe/ids.h"
#include "lab_host_probe/state.h"

namespace Kasselvania::LabHostProbe {

using namespace Steinberg;
using namespace Steinberg::Vst;

tresult PLUGIN_API Controller::initialize (FUnknown* context)
{
    const auto result = EditController::initialize (context);
    if (result != kResultOk)
        return result;

    parameters.addParameter (STR16 ("Gain"), nullptr, 0, kDefaultGain,
                             ParameterInfo::kCanAutomate, kGainParameterId);
    parameters.addParameter (STR16 ("Bypass"), nullptr, 1, 0.0,
                             ParameterInfo::kCanAutomate | ParameterInfo::kIsBypass,
                             kBypassParameterId);
    return kResultOk;
}

tresult PLUGIN_API Controller::setComponentState (IBStream* stream)
{
    State componentState;
    const auto result = readState (stream, kProcessorStateMagic, componentState);
    if (result != kResultOk)
        return result;

    if (setParamNormalized (kGainParameterId, componentState.gain) != kResultOk ||
        setParamNormalized (kBypassParameterId, componentState.bypass ? 1.0 : 0.0) != kResultOk)
        return kResultFalse;
    return kResultOk;
}

tresult PLUGIN_API Controller::setState (IBStream* stream)
{
    State controllerState;
    const auto result = readState (stream, kControllerStateMagic, controllerState);
    if (result != kResultOk)
        return result;
    if (setParamNormalized (kGainParameterId, controllerState.gain) != kResultOk ||
        setParamNormalized (kBypassParameterId, controllerState.bypass ? 1.0 : 0.0) != kResultOk)
        return kResultFalse;
    return kResultOk;
}

tresult PLUGIN_API Controller::getState (IBStream* stream)
{
    State controllerState;
    controllerState.gain = getParamNormalized (kGainParameterId);
    controllerState.bypass = getParamNormalized (kBypassParameterId) >= 0.5;
    return writeState (stream, kControllerStateMagic, controllerState);
}

} // namespace Kasselvania::LabHostProbe
