#pragma once
#include "editor_presentation.h"
// UI-only Rust boundary. Handles belong to the creating thread; they are never
// SDK pointers and cannot be used by process(). Release retires the handle even
// if X11 restoration fails; the private connection's SaveSet rescues Wine.
namespace LVBEditor {
constexpr uint32_t SurfaceAbi = 1;
enum SurfaceResult : uint32_t {
  Ok, Identity, Busy, WrongThread, Connection, ReplyTimeout, X11, Unsupported,
  Internal
};
}
extern "C" {
uint32_t lvb_editor_surface_create(uint32_t abi, uint32_t parent, uint64_t owner,
                                   uint32_t width, uint32_t height, uint64_t *out);
uint32_t lvb_editor_surface_bind(uint64_t, const LVBEditorPresentation::Record *);
uint32_t lvb_editor_surface_resize(uint64_t, const LVBEditorPresentation::Record *);
uint32_t lvb_editor_surface_focus(uint64_t);
uint32_t lvb_editor_surface_release(uint64_t);
}
