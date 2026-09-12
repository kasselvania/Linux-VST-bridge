#pragma once
// Development observer ABI 1. Separate from every audio/UI transport contract.
// One UI-thread writer, append-only slots, release commit last. Never overwrite
// a committed slot; a reader can retain complete records after helper failure.
#include <atomic>
#include <cstdint>
#include <cstring>
#include <type_traits>
namespace uio1 {
constexpr uint32_t capacity=16384, header_bytes=4096, record_bytes=112;
constexpr uint32_t mapping_bytes=header_bytes+capacity*record_bytes;
struct Record {
  uint64_t commit, qpc, action, hwnd, focus, active, capture;
  uint32_t message, source;
  int32_t x,y,screen_x,screen_y;
  uint32_t buttons,key_class;
  int64_t result;
  uint64_t message_time, cost_ticks;
};
static_assert(sizeof(Record)==record_bytes && std::is_trivial_v<Record>);
struct Header {
  char magic[4]; uint32_t version,bytes,record_size;
  uint32_t pid,tid; uint64_t start,root,nonce,frequency;
  uint64_t committed,dropped,action,stop,ready,closed;
  uint64_t ping,pong,pong_qpc,hook_calls,hook_ticks,max_hook_ticks;
  uint64_t filtered,heartbeat_errors,scope_errors,unhook_errors;
  uint64_t clock_request,clock_response,clock_qpc;
  uint64_t ping_qpc;
  uint64_t detached;
};
static_assert(sizeof(Header)<header_bytes);
inline std::atomic_ref<uint64_t> atom(uint64_t& x) {return std::atomic_ref<uint64_t>(x);}
static_assert(std::atomic_ref<uint64_t>::is_always_lock_free);
inline bool append(Header& h, Record* slots, Record r) noexcept {
  const auto n=atom(h.committed).load(std::memory_order_relaxed);
  if(n>=capacity){atom(h.dropped).fetch_add(1,std::memory_order_relaxed);return false;}
  r.commit=0; slots[n]=r;
  atom(slots[n].commit).store(n+1,std::memory_order_release);
  atom(h.committed).store(n+1,std::memory_order_release);return true;
}
inline bool selected(uint32_t m) noexcept {
  switch(m){case 0x200:case 0x201:case 0x202:case 0x204:case 0x205:case 0x20a:
    case 0x100:case 0x101:case 0x104:case 0x105:case 7:case 8:case 6:case 0x21:
    case 0x215:case 5:case 0xf:case 0x113:case 0x2e0:case 0x84:return true;
    default:return false;}
}
// No typed characters or arbitrary WPARAM/LPARAM contents leave the hook.
inline uint32_t key_class(uint64_t vk) noexcept {
  if(vk==0x10||vk==0x11||vk==0x12||vk==0x5b||vk==0x5c)return 1;
  if(vk==0x1b)return 2;
  if(vk>=0x21&&vk<=0x28)return 3;
  return 4;
}
}
