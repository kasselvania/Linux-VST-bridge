#include "controller.h"
#include "processor.h"
#include "version.h"

#include "lab_host_probe/ids.h"

#include "public.sdk/source/main/pluginfactory_constexpr.h"

BEGIN_FACTORY_DEF (HP0_VENDOR_NAME, HP0_VENDOR_URL, HP0_VENDOR_EMAIL, 2)

DEF_CLASS (Kasselvania::LabHostProbe::kProcessorClassId, Steinberg::PClassInfo::kManyInstances,
           kVstAudioEffectClass, HP0_PLUGIN_NAME, Steinberg::Vst::kDistributable, "Fx",
           HP0_PLUGIN_VERSION, kVstVersionString,
           Kasselvania::LabHostProbe::Processor::createInstance, nullptr)

DEF_CLASS (Kasselvania::LabHostProbe::kControllerClassId, Steinberg::PClassInfo::kManyInstances,
           kVstComponentControllerClass, HP0_PLUGIN_NAME " Controller", 0, "",
           HP0_PLUGIN_VERSION, kVstVersionString,
           Kasselvania::LabHostProbe::Controller::createInstance, nullptr)

END_FACTORY
