#pragma once
// Proxy-only (not part of the Windows host sources): the bridge's own
// "nothing recorded" state for an instance that never received the host's
// state, and the envelope reader that also accepts it.
#include "../../vst-state/stream.h"
#include <algorithm>
#include <cstdint>
#include <vector>
namespace LVBState {
// The bridge's own "nothing recorded" state: what an instance that never
// received the host's state saves, so a host that asks before restoring keeps
// asking. 16 bytes that cannot be an envelope (which starts "LVBSTATE") and so
// cannot be confused with any plug-in's state, empty or not.
inline constexpr uint8_t nothingRecorded[16] = {'L','V','B','N','O','N','E',0, 1,0,0,0, 0,0,0,0};
// As readEnvelope, but the exact nothingRecorded marker is also accepted
// (nothing=true, b empty). Anything else must be a complete envelope.
inline bool readEnvelopeOrNothing(Steinberg::IBStream *s, std::vector<uint8_t> &b,
                                  bool &nothing) {
  nothing = false;
  b.assign(sizeof(nothingRecorded), 0);
  if (!transfer(s, b.data(), b.size(), false))
    return false;
  uint8_t extra = 0;
  Steinberg::int32 count = 0;
  if (std::equal(b.begin(), b.end(), nothingRecorded)) {
    auto result = s->read(&extra, 1, &count);
    nothing = count == 0 && (result == Steinberg::kResultOk ||
                             result == Steinberg::kResultFalse);
    b.clear();
    return nothing;
  }
  b.resize(overhead);
  if (!transfer(s, b.data() + sizeof(nothingRecorded), overhead - sizeof(nothingRecorded), false))
    return false;
  uint32_t n = 0;
  for (size_t i = 0; i < 4; ++i)
    n |= uint32_t(b[64 + i]) << (8 * i);
  if (n > payloadLimit)
    return false;
  b.resize(overhead + n);
  if (!transfer(s, b.data() + overhead, n, false))
    return false;
  count = 0;
  auto result = s->read(&extra, 1, &count);
  return count == 0 && (result == Steinberg::kResultOk ||
                        result == Steinberg::kResultFalse);
}
} // namespace LVBState
