#pragma once
#include <cstdint>
// UI ABI 1 / independent shared-memory protocol 1. No host/SDK pointer crosses.
// Calls are native owner/UI-thread only, except the private atomic revision
// assigned by the Rust audio admission path. No GUI queue is used by audio.
struct ap11_gui_message_t {
  uint32_t kind = 0, id = 0;
  uint64_t revision = 0;
  double value = 0;
  int32_t flags = 0, steps = 0;
  uint32_t count = 0, result = 0;
  char16_t title[128]{}, units[128]{};
};
static_assert(sizeof(ap11_gui_message_t) == 552);
namespace AP11 {
enum Kind : uint32_t {
  Open = 1,
  Close = 2,
  Set = 3,
  Refresh = 4,
  Begin = 101,
  Value = 102,
  End = 103,
  Dirty = 104,
  GroupBegin = 105,
  GroupEnd = 106,
  RequestOpen = 107,
  EditorStatus = 108,
  RefreshBegin = 109,
  Parameter = 110,
  RefreshEnd = 111,
  Restart = 112
};
enum Error : uint32_t {
  NoError = 0,
  Backlog = 1,
  Protocol = 2,
  WrongThread = 3,
  NoView = 4,
  Platform = 5,
  Attach = 6,
  Size = 7,
  Removal = 8,
  Controller = 9,
  Host = 10,
  Closed = 11
};
} // namespace AP11
extern "C" {
uint32_t ap11_gui_generation(uint64_t, uint64_t *);
uint32_t ap11_gui_command(uint64_t, uint64_t, ap11_gui_message_t *);
uint32_t ap11_gui_take(uint64_t, uint64_t, ap11_gui_message_t *);
uint32_t ap11_gui_capabilities(uint64_t, uint64_t, uint32_t);
uint32_t ap11_gui_failure(uint64_t, uint64_t, uint32_t);
}
