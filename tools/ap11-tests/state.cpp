// Real SDK state interfaces through production commercial_state(). Reading a
// bundle must not restore it into the editor and restart the host again.
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "windows-factory-probe/source/ap8_state.h"
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <limits>
using namespace Steinberg;
using namespace Steinberg::Vst;
void check(bool ok, const char *why) {
  if (!ok) {
    std::cerr << "FAIL: " << why << '\n';
    std::exit(1);
  }
}
struct FixtureComponent final : AudioEffect {
  double value = .375;
  unsigned restores = 0;
  tresult PLUGIN_API getState(IBStream *s) override {
    int32 n = 0;
    return s->write(&value, 8, &n) == kResultOk && n == 8 ? kResultOk
                                                          : kResultFalse;
  }
  tresult PLUGIN_API setState(IBStream *s) override {
    int32 n = 0;
    ++restores;
    return s->read(&value, 8, &n) == kResultOk && n == 8 ? kResultOk
                                                         : kResultFalse;
  }
};
struct Controller final : EditController {
  unsigned synchronizations = 0, invalidations = 0;
  bool invalid_readback = false;
  double readback_value = 0.;
  ParamValue PLUGIN_API getParamNormalized(ParamID id) override {
    return invalid_readback && id == 42 ? readback_value
                                      : EditController::getParamNormalized(id);
  }
  tresult PLUGIN_API initialize(FUnknown *h) override {
    auto r = EditController::initialize(h);
    parameters.addParameter(u"Gain", u"", 0, .375, ParameterInfo::kCanAutomate,
                            42);
    return r;
  }
  tresult PLUGIN_API setComponentState(IBStream *s) override {
    ++synchronizations;
    double value = 0;
    int32 n = 0;
    if (s->read(&value, 8, &n) != kResultOk || n != 8)
      return kResultFalse;
    ++invalidations;
    return EditController::setParamNormalized(42, value);
  }
};
int main() {
  HostApplication host;
  FixtureComponent component;
  Controller controller;
  check(component.initialize(&host) == kResultOk &&
            controller.initialize(&host) == kResultOk,
        "SDK initialize");
  using linux_vst_bridge::wf0::commercial_state;
  auto original = commercial_state(component, controller, true, nullptr);
  check(controller.synchronizations == 0 && controller.invalidations == 0,
        "state capture must not reapply component state or invalidate "
        "parameters");
  for (int i = 0; i < 32; ++i)
    check(commercial_state(component, controller, true, nullptr) == original,
          "repeated capture remains pure");
  component.value = .75;
  controller.setParamNormalized(42, .75);
  auto recalled = commercial_state(component, controller, true, &original);
  check(component.restores == 1 && controller.synchronizations == 1 &&
            controller.invalidations == 1,
        "restore synchronizes controller exactly once");
  check(component.value == .375 && controller.getParamNormalized(42) == .375 &&
            recalled == original,
        "opaque restore and controller readback retained");
  check(commercial_state(component, controller, true, nullptr) == original &&
            controller.synchronizations == 1,
        "post-restore capture does not trigger another refresh");
  controller.invalid_readback = true;
  for (auto value : {-.25, 1.25, std::numeric_limits<double>::quiet_NaN()}) {
    controller.readback_value = value;
    bool refused = false;
    try { commercial_state(component, controller, true, nullptr); }
    catch (const std::runtime_error& e) {
      refused = std::string(e.what()).find("state parameter readback: id=42 value=") == 0;
    }
    check(refused, "invalid readback identifies parameter and value; never clamps or fabricates state");
  }
  controller.terminate();
  component.terminate();
  std::cout << "AP11 pure state capture and exactly-once restore "
               "synchronization PASS\n";
}
