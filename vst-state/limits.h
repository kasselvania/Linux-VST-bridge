#pragma once
#include <cstddef>
namespace LVBState {
// Opaque state may contain recorded audio. Allocate only its actual extent.
constexpr size_t payloadLimit = 256 * 1024 * 1024, overhead = 104;
}
