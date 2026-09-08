#pragma once
#include <algorithm>
#include <array>
#include <atomic>
#include <bit>
#include <cmath>
#include <cstdint>
#include <span>
#include <type_traits>

namespace linux_vst_bridge::wf0 {
// Audio automation remains ordered in IParameterChanges. The controller needs
// only the latest display value, delivered on its owner thread. A state barrier
// drains these values while the audio worker is parked, before asking the SDK
// for opaque state. No controller call or allocation occurs on the audio
// thread.
class ControllerUpdates {
public:
  static constexpr size_t capacity = 8192;
  static_assert(std::atomic<uint64_t>::is_always_lock_free);
  bool configure(std::span<const uint32_t> ids) {
    if (ids.size() > capacity)
      return false;
    size_ = ids.size();
    std::copy(ids.begin(), ids.end(), ids_.begin());
    std::sort(ids_.begin(), ids_.begin() + size_);
    return std::adjacent_find(ids_.begin(), ids_.begin() + size_) ==
           ids_.begin() + size_;
  }
  bool publish(uint32_t id, double value, uint64_t revision = 0) {
    if (!std::isfinite(value) || value < 0 || value > 1)
      return false;
    auto end = ids_.begin() + size_;
    auto p = std::lower_bound(ids_.begin(), end, id);
    if (p == end || *p != id)
      return false;
    size_t i = size_t(p - ids_.begin());
    stamps_[i].fetch_add(1, std::memory_order_acq_rel);
    values_[i].store(std::bit_cast<uint64_t>(value), std::memory_order_relaxed);
    revisions_[i].store(revision, std::memory_order_relaxed);
    stamps_[i].fetch_add(1, std::memory_order_release);
    dirty_[i / 64].fetch_or(uint64_t(1) << (i % 64), std::memory_order_release);
    return true;
  }
  template <class Apply> bool drain(Apply apply) {
    for (size_t word = 0; word < (size_ + 63) / 64; ++word) {
      if (!dirty_[word].load(std::memory_order_acquire))
        continue;
      auto bits = dirty_[word].exchange(0, std::memory_order_acq_rel);
      while (bits) {
        size_t i = word * 64 + std::countr_zero(bits);
        bits &= bits - 1;
        auto before = stamps_[i].load(std::memory_order_acquire);
        auto value = values_[i].load(std::memory_order_relaxed);
        auto revision = revisions_[i].load(std::memory_order_relaxed);
        std::atomic_thread_fence(std::memory_order_acquire);
        if ((before & 1) ||
            before != stamps_[i].load(std::memory_order_acquire)) {
          dirty_[word].fetch_or(uint64_t(1) << (i % 64),
                                std::memory_order_release);
          continue; // Never wait for an audio-side publication.
        }
        if constexpr (std::is_invocable_r_v<bool, Apply, uint32_t, double,
                                            uint64_t>) {
          if (!apply(ids_[i], std::bit_cast<double>(value), revision))
            return false;
        } else if (!apply(ids_[i], std::bit_cast<double>(value)))
          return false;
      }
    }
    return true;
  }

private:
  size_t size_ = 0;
  std::array<uint32_t, capacity> ids_{};
  std::array<std::atomic<uint64_t>, capacity> values_{};
  std::array<std::atomic<uint64_t>, capacity> revisions_{}, stamps_{};
  std::array<std::atomic<uint64_t>, capacity / 64> dirty_{};
};
} // namespace linux_vst_bridge::wf0
