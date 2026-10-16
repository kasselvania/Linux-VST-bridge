#pragma once
// Why the DAW could not start this plug-in's audio (#403). The DAW shows only
// "error code 1"; the backend keeps one bounded record per plug-in class so
// the manager's card can say why. Owner thread, inactive instance only.
#include <cstdint>
#include <cstring>
extern "C" {
uint32_t lvb_activation_refused(const uint8_t* identity, const uint8_t* engine,
                                uint32_t stage, uint32_t reason, uint64_t requested);
uint32_t lvb_activation_cleared(const uint8_t* identity);
}
namespace LVBActivation {
enum Stage : uint32_t { Setup = 1, Activate = 2 };
// Codes match the backend's activation_record::REASONS (index + 1).
enum Reason : uint32_t {
  None = 0,
  BlockSize = 1,         // maxSamplesPerBlock outside 1..LVBHostBlock::maximum
  SampleFormat = 2,      // 64-bit (or unknown) sample size
  ProcessMode = 3,       // neither realtime, prefetch nor offline
  SampleRate = 4,        // a rate the bridge does not carry
  WindowsSetup = 5,      // the Windows side or the plug-in refused the setup
  WindowsActivation = 6, // the Windows side or the plug-in refused to activate
  Memory = 7,            // the bridge could not prepare its buffers
};
// The backend's retained setup detail (ap2_error) as one reason.
inline Reason setupDetail(const char* detail) {
  if (!detail) return WindowsSetup;
  if (std::strstr(detail, "DAW block size above")) return BlockSize;
  if (std::strstr(detail, "unsupported setup")) return SampleRate;
  return WindowsSetup;
}
} // namespace LVBActivation
