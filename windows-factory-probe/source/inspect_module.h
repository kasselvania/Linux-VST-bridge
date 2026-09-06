#pragma once
#include "factory_census.h"
namespace linux_vst_bridge::wf0 {
// Owner-thread SDK inspection of one selected module; never a DAW callback.
int inspect_module(Steinberg::IPluginFactory*, EventWriter&);
}
