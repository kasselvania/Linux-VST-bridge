#pragma once

#include "public.sdk/source/vst/vsteditcontroller.h"

namespace Kasselvania::LabHostProbe {

class Controller final : public Steinberg::Vst::EditController
{
public:
    static Steinberg::FUnknown* createInstance (void*)
    {
        return static_cast<Steinberg::Vst::IEditController*> (new Controller ());
    }

    Steinberg::tresult PLUGIN_API initialize (Steinberg::FUnknown* context) override;
    Steinberg::tresult PLUGIN_API setComponentState (Steinberg::IBStream* stream) override;
    Steinberg::tresult PLUGIN_API setState (Steinberg::IBStream* stream) override;
    Steinberg::tresult PLUGIN_API getState (Steinberg::IBStream* stream) override;
};

} // namespace Kasselvania::LabHostProbe
