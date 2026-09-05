#pragma once
// Ordinary bounded SDK stream, shared only by the two C++ SDK edges/tests.
// It deliberately does not implement IStreamAttributes or expose project paths.
#include "pluginterfaces/base/ibstream.h"
#include <algorithm>
#include <atomic>
#include <cstdint>
#include <cstring>
#include <limits>
#include <vector>
namespace LVBState {
constexpr size_t payloadLimit = 1024 * 1024, overhead = 104;
class Stream final : public Steinberg::IBStream {
public:
  std::vector<uint8_t> bytes;
  size_t position = 0;
  bool failed = false;
  explicit Stream(std::vector<uint8_t> input = {}, size_t limit = payloadLimit)
      : bytes(std::move(input)), limit_(limit) {
    if (bytes.size() > limit_)
      failed = true;
  }
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id,
                                               void **out) override {
    if (!out)
      return Steinberg::kInvalidArgument;
    *out = nullptr;
    if (Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::IBStream::iid) ||
        Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::FUnknown::iid)) {
      *out = static_cast<Steinberg::IBStream *>(this);
      addRef();
      return Steinberg::kResultOk;
    }
    return Steinberg::kNoInterface;
  }
  Steinberg::uint32 PLUGIN_API addRef() override { return ++references_; }
  // Streams have lexical ownership; no plug-in may retain one beyond the call.
  Steinberg::uint32 PLUGIN_API release() override { return --references_; }
  bool quiescent() const { return references_ == 1; }
  Steinberg::tresult PLUGIN_API read(void *out, Steinberg::int32 n,
                                     Steinberg::int32 *count) override {
    if (count)
      *count = 0;
    if (failed || n < 0 || (n && !out) || position > bytes.size())
      return Steinberg::kResultFalse;
    size_t got = std::min(static_cast<size_t>(n), bytes.size() - position);
    if (got)
      std::memcpy(out, bytes.data() + position, got);
    position += got;
    if (count)
      *count = static_cast<Steinberg::int32>(got);
    return got == static_cast<size_t>(n) ? Steinberg::kResultOk
                                         : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API write(void *input, Steinberg::int32 n,
                                      Steinberg::int32 *count) override {
    if (count)
      *count = 0;
    if (failed || n < 0 || (n && !input) || position > limit_ ||
        static_cast<size_t>(n) > limit_ - position) {
      failed = true;
      return Steinberg::kResultFalse;
    }
    try {
      size_t end = position + static_cast<size_t>(n);
      if (end > bytes.size())
        bytes.resize(end);
      if (n)
        std::memcpy(bytes.data() + position, input, static_cast<size_t>(n));
      position = end;
      if (count)
        *count = n;
      return Steinberg::kResultOk;
    } catch (...) {
      failed = true;
      return Steinberg::kResultFalse;
    }
  }
  Steinberg::tresult PLUGIN_API seek(Steinberg::int64 offset,
                                     Steinberg::int32 mode,
                                     Steinberg::int64 *result) override {
    int64_t base = mode == kIBSeekSet   ? 0
                   : mode == kIBSeekCur ? static_cast<int64_t>(position)
                   : mode == kIBSeekEnd ? static_cast<int64_t>(bytes.size())
                                        : -1;
    if (failed || base < 0 || offset < -base ||
        offset > static_cast<int64_t>(limit_) - base)
      return Steinberg::kResultFalse;
    position = static_cast<size_t>(base + offset);
    if (result)
      *result = static_cast<int64_t>(position);
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API tell(Steinberg::int64 *out) override {
    if (!out)
      return Steinberg::kInvalidArgument;
    *out = static_cast<int64_t>(position);
    return Steinberg::kResultOk;
  }

private:
  size_t limit_;
  std::atomic<Steinberg::uint32> references_{1};
};
// A short successful operation is legal. Zero progress or an inconsistent count
// is a failure. Counts, not an optimistic return code, establish completion.
inline bool transfer(Steinberg::IBStream *s, uint8_t *bytes, size_t n,
                     bool writing) {
  if (!s || n > payloadLimit + overhead)
    return false;
  while (n) {
    Steinberg::int32 count = 0, requested = static_cast<Steinberg::int32>(n);
    auto r = writing ? s->write(bytes, requested, &count)
                     : s->read(bytes, requested, &count);
    if (count <= 0 || count > requested ||
        (r != Steinberg::kResultOk && r != Steinberg::kResultFalse))
      return false;
    bytes += count;
    n -= static_cast<size_t>(count);
    if (r != Steinberg::kResultOk && n)
      return false;
  }
  return true;
}
inline bool readEnvelope(Steinberg::IBStream *s, std::vector<uint8_t> &b) {
  b.resize(overhead);
  if (!transfer(s, b.data(), b.size(), false))
    return false;
  uint32_t n = 0;
  for (size_t i = 0; i < 4; ++i)
    n |= uint32_t(b[64 + i]) << (8 * i);
  if (n > payloadLimit)
    return false;
  b.resize(overhead + n);
  if (!transfer(s, b.data() + overhead, n, false))
    return false;
  // Host supplies the component's bounded stream; reject unexplained trailing
  // bytes.
  uint8_t extra = 0;
  Steinberg::int32 count = 0;
  s->read(&extra, 1, &count);
  return count == 0;
}
} // namespace LVBState
