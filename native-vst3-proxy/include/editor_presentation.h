#pragma once
#include <cstddef>
#include <cstdint>

// Native surface identity boundary, also used by the first-party fixture.
// No HWND or SDK pointer crosses; supervised editor status supplies the XID.
namespace LVBEditorPresentation {
enum Mode : uint32_t { Detached = 0, EmbeddedX11 = 1 };
enum Kind : uint32_t {
  WindowReady = 1, WindowResized = 2, WindowClosed = 3, FocusResult = 4,
  PopupResult = 5, InputObserved = 6, PopupVisible = 7,
  Resize = 101, Focus = 102, Popup = 103, Close = 104,
  CancelPopup = 105
};
struct Record {
  uint32_t version = 1, extent = 48, kind = 0, presentation = EmbeddedX11;
  uint64_t owner = 0;
  uint32_t epoch = 0, window = 0, width = 0, height = 0, result = 0, reserved = 0;
  bool valid() const {
    return version == 1 && extent == sizeof(Record) &&
      (presentation == Detached || presentation == EmbeddedX11) && owner && epoch && !reserved;
  }
};
static_assert(sizeof(Record) == 48);
static_assert(offsetof(Record, owner) == 16);
static_assert(offsetof(Record, epoch) == 24);
} // namespace LVBEditorPresentation
