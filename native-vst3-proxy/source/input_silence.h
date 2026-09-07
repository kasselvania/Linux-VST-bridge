#pragma once
#include <bit>
#include <cmath>
#include <cstdint>

namespace AP2 {
struct InputSilence {
  uint64_t flags, nonzero = 0;
  uint32_t first_bits = 0;
  double finite_peak = 0.;
};

// SDK silence flags are optional hints. Preserve the samples, including a
// host's near-zero noise floor, and remove only a contradicted hint. Bit tests
// also handle signed zero/subnormals independently of the host's FP mode.
inline InputSilence inputSilence(const float* left, const float* right,
                                 int frames, uint64_t flags) {
  InputSilence result{flags};
  const float* inputs[] = {left, right};
  for (unsigned ch = 0; ch < 2; ++ch) {
    if (!(flags & (uint64_t(1) << ch))) continue;
    for (int i = 0; i < frames; ++i) {
      const auto bits = std::bit_cast<uint32_t>(inputs[ch][i]);
      if (!(bits & 0x7fffffff)) continue;
      result.flags &= ~(uint64_t(1) << ch);
      if (!result.nonzero++) result.first_bits = bits;
      const double value = std::abs(double(inputs[ch][i]));
      if (std::isfinite(value) && value > result.finite_peak)
        result.finite_peak = value;
    }
  }
  return result;
}
} // namespace AP2
