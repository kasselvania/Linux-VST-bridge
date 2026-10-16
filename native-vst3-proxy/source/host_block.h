#pragma once
#include <cstdint>
// DAW block sizes. The Windows plug-in is set up for one transport block; a
// longer DAW block is played as consecutive transport blocks inside the same
// DAW callback by the backend (daw_blocks.rs), so latency is unchanged.
namespace LVBHostBlock {
// Largest maxSamplesPerBlock accepted; the backend's HOST_BLOCK_MAX.
inline constexpr int32_t maximum = 32768;
// One transport block: what the Windows plug-in is set up for.
inline constexpr int32_t transport = 1024;
static_assert(maximum % transport == 0);
} // namespace LVBHostBlock
