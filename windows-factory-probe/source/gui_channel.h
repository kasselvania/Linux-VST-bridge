#pragma once
#include "../../native-vst3-proxy/include/ap11_gui.h"
#include "ap1_protocol.h"
#include <atomic>
#include <cstring>
#include <string>
#include <windows.h>
namespace linux_vst_bridge::wf0 {
// Companion UI protocol 5. The session's Linux owner creates this file before
// the existing authenticated handshake. Fixed layout; no SDK pointer.
// Activation uses explicitly typed X11 IDs.
class GuiChannel {
  static constexpr uint64_t capacity = 512;
  static constexpr size_t header = 320,
                          bytes = header +
                                  2 * capacity * sizeof(ap11_gui_message_t);
  HANDLE file_ = INVALID_HANDLE_VALUE, mapping_ = nullptr;
  uint8_t *view_ = nullptr;
  std::atomic_ref<uint64_t> word(size_t n) const {
    return std::atomic_ref<uint64_t>(*reinterpret_cast<uint64_t *>(view_ + n));
  }
  std::atomic_ref<uint32_t> flag(size_t n) const {
    return std::atomic_ref<uint32_t>(*reinterpret_cast<uint32_t *>(view_ + n));
  }

public:
  static_assert(std::atomic_ref<uint64_t>::is_always_lock_free &&
                std::atomic_ref<uint32_t>::is_always_lock_free);
  GuiChannel(const std::wstring &directory,
             const std::array<uint8_t, 16> &session) {
    using ap1::require;
    file_ = CreateFileW(
        (directory + L"\\ap11.ui").c_str(), GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_EXISTING, 0, nullptr);
    try {
      LARGE_INTEGER size{};
      require(file_ != INVALID_HANDLE_VALUE && GetFileSizeEx(file_, &size) &&
                  size.QuadPart == bytes,
              "GUI backing size");
      mapping_ =
          CreateFileMappingW(file_, nullptr, PAGE_READWRITE, 0, 0, nullptr);
      require(mapping_ != nullptr, "GUI mapping");
      view_ = static_cast<uint8_t *>(
          MapViewOfFile(mapping_, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, bytes));
      require(view_ != nullptr, "GUI view");
      require(!std::memcmp(view_, "LVBU", 4) && ap1::get(view_ + 4, 4) == 5 &&
                  ap1::get(view_ + 8, 4) == bytes &&
                  ap1::get(view_ + 12, 4) == sizeof(ap11_gui_message_t) &&
                  ap1::get(view_ + 32, 4) == capacity &&
                  std::equal(session.begin(), session.end(), view_ + 16),
              "GUI version/session/layout");
    } catch (...) {
      release();
      throw;
    }
  }
  ~GuiChannel() { release(); }
  GuiChannel(const GuiChannel &) = delete;
  GuiChannel &operator=(const GuiChannel &) = delete;
  void release() {
    if (view_) {
      UnmapViewOfFile(view_);
      view_ = nullptr;
    }
    if (mapping_) {
      CloseHandle(mapping_);
      mapping_ = nullptr;
    }
    if (file_ != INVALID_HANDLE_VALUE) {
      CloseHandle(file_);
      file_ = INVALID_HANDLE_VALUE;
    }
  }
  bool closed() const { return flag(104).load(std::memory_order_acquire) != 0; }
  uint32_t failure() const { return flag(108).load(std::memory_order_acquire); }
  void fail(uint32_t code) {
    uint32_t expected = 0;
    flag(108).compare_exchange_strong(expected, code,
                                      std::memory_order_acq_rel);
  }
  uint64_t revision() {
    auto r = word(96).fetch_add(1, std::memory_order_acq_rel);
    if (r == UINT64_MAX) {
      fail(AP11::Protocol);
      return 0;
    }
    return r;
  }
  uint32_t capabilities() const {
    return flag(136).load(std::memory_order_acquire);
  }
  void view_stage(uint32_t stage) {
    if (stage < 12 || stage == 100)
      flag(164).store(stage, std::memory_order_release);
    flag(160).store(stage, std::memory_order_release);
  }
  void view_fault(EXCEPTION_POINTERS *e) {
    auto *r = e->ExceptionRecord;
    auto *c = e->ContextRecord;
    word(176).store(c->Rip, std::memory_order_relaxed);
    word(184).store(r->NumberParameters > 1 ? r->ExceptionInformation[1] : 0,
                    std::memory_order_relaxed);
    word(192).store(c->Rcx, std::memory_order_relaxed);
    word(200).store(c->Rdx, std::memory_order_relaxed);
    word(208).store(c->R8, std::memory_order_relaxed);
    word(216).store(c->R9, std::memory_order_relaxed);
    word(224).store(c->Rax, std::memory_order_relaxed);
    word(232).store(c->Rbp, std::memory_order_relaxed);
    word(240).store(c->Rsp, std::memory_order_relaxed);
    flag(172).store(r->NumberParameters, std::memory_order_relaxed);
    flag(168).store(r->ExceptionCode, std::memory_order_release);
  }
  void ready() { flag(112).store(1, std::memory_order_release); }
  void heartbeat() { word(144).fetch_add(1, std::memory_order_release); }
  uint64_t close_requested() const {
    return word(120).load(std::memory_order_acquire);
  }
  struct CloseRequest {
    uint64_t sequence = 0, native_view = 0, cutoff = 0;
    uint32_t view_epoch = 0;
  };
  bool take_close(CloseRequest &request) const {
    const auto sequence = word(120).load(std::memory_order_seq_cst);
    if ((sequence & 1) || sequence == close_acknowledged())
      return false;
    request = {sequence, word(256).load(std::memory_order_seq_cst),
               word(152).load(std::memory_order_seq_cst),
               flag(264).load(std::memory_order_seq_cst)};
    // Never wait/spin for the producer. A changing snapshot is retried on the
    // next normal UI service turn; it cannot close the wrong generation.
    return sequence == word(120).load(std::memory_order_seq_cst);
  }
  uint64_t close_acknowledged() const {
    return word(128).load(std::memory_order_acquire);
  }
  uint64_t close_cutoff() const {
    return word(152).load(std::memory_order_acquire);
  }
  uint64_t commands_consumed() const {
    return word(72).load(std::memory_order_acquire);
  }
  void close_ack(uint64_t n) { word(128).store(n, std::memory_order_release); }
  void open_state(bool open) {
    flag(116).store(open ? 1u : 0u, std::memory_order_release);
  }
  size_t available() const {
    auto p = word(80).load(std::memory_order_relaxed),
         c = word(88).load(std::memory_order_acquire);
    return c <= p && p - c <= capacity ? size_t(capacity - (p - c)) : 0;
  }
  bool send(const ap11_gui_message_t &message) {
    if (!AP11::valid(message)) {
      fail(AP11::Protocol);
      return false;
    }
    if (closed() || failure())
      return false;
    auto p = word(80).load(std::memory_order_relaxed),
         c = word(88).load(std::memory_order_acquire);
    if (c > p || p - c > capacity || p == UINT64_MAX) {
      fail(AP11::Protocol);
      return false;
    }
    if (p - c == capacity) {
      fail(AP11::Backlog);
      return false;
    }
    std::memcpy(view_ + header + (capacity + p % capacity) * sizeof(message),
                &message, sizeof(message));
    word(80).store(p + 1, std::memory_order_release);
    return true;
  }
  bool take(ap11_gui_message_t &message) {
    auto c = word(72).load(std::memory_order_relaxed),
         p = word(64).load(std::memory_order_acquire);
    if (c > p || p - c > capacity || c == UINT64_MAX) {
      fail(AP11::Protocol);
      return false;
    }
    if (c == p)
      return false;
    std::memcpy(&message, view_ + header + c % capacity * sizeof(message),
                sizeof(message));
    word(72).store(c + 1, std::memory_order_release);
    if (!AP11::valid(message)) {
      fail(AP11::Protocol);
      return false;
    }
    return true;
  }
};
} // namespace linux_vst_bridge::wf0
