#pragma once
#include "factory_census.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
namespace linux_vst_bridge::wf0 {
// Explicit inactive inspection only. Never called by the audio/editor service loop.
void assess_graphics_editor(Steinberg::Vst::IEditController&, EventWriter&);
void assess_graphics_runtime(EventWriter&);
}
