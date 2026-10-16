#pragma once
#include "../../native-vst3-proxy/include/ap11_midi_mapping.h"
#include "pluginterfaces/base/funknown.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmidicontrollers.h"
#include <algorithm>
#include <vector>
namespace linux_vst_bridge::wf0 {
// The vendor's complete IMidiMapping table, read on its controller owner
// thread (SDK: UI thread, connected). A controller without IMidiMapping has
// an empty table, so hosts keep today's behavior for it. Parameter IDs are
// reported as the vendor answered; the native side admits only IDs its
// descriptor exposes. No windows.h dependency: also built natively for tests.
struct MidiTable {
  bool supported = false;
  uint32_t buses = 0;
  std::vector<AP11::MidiAssignment> assignments;
};
inline MidiTable read_midi_table(Steinberg::Vst::IEditController &controller, int32_t event_inputs) {
  using namespace Steinberg;
  MidiTable table;
  FUnknownPtr<Vst::IMidiMapping> mapping(&controller);
  if (!mapping)
    return table;
  table.supported = true;
  table.buses = uint32_t(std::clamp<int32_t>(event_inputs, 0, int32_t(AP11::MidiBuses)));
  for (uint32_t bus = 0; bus < table.buses; ++bus)
    for (uint32_t channel = 0; channel < AP11::MidiChannels; ++channel)
      for (uint32_t number = 0; number < AP11::MidiControllers; ++number) {
        Vst::ParamID id = 0;
        if (mapping->getMidiControllerAssignment(int32(bus), int16(channel), Vst::CtrlNumber(number),
                                                 id) == kResultOk)
          table.assignments.push_back({bus, channel, number, id});
      }
  return table;
}
} // namespace linux_vst_bridge::wf0
