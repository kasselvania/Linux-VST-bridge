#pragma once
#include "ap11_gui.h"
#include <cstdint>
namespace AP15 {
// Native UI owner only. Object addresses, DSP generations, editor epochs and
// activation serials are deliberately not interchangeable identities.
class EditorLifecycle {
  uint64_t allocated_ = 0, retired_ = 0;
  ap11_gui_message_t current_{};
public:
  uint64_t allocate() {
    return allocated_ == UINT64_MAX ? 0 : ++allocated_;
  }
  uint64_t owner() const { return current_.native_view; }
  uint32_t state(uint64_t owner) const {
    return owner && owner == current_.native_view ? current_.lifecycle : AP11::Absent;
  }
  bool begin(uint64_t owner, uint64_t activation, AP11::ActivationContext context,
             ap11_gui_message_t &command) {
    if (!owner || owner > allocated_ || owner <= retired_ || !activation ||
        (current_.native_view && current_.native_view != owner) ||
        activation <= current_.activation || current_.lifecycle >= AP11::ClosedByVendor)
      return false;
    command = {};
    command.kind = AP11::Open;
    command.native_view = owner;
    command.view_epoch = current_.view_epoch;
    command.activation = activation;
    command.user_time = context.user_time;
    command.requestor_x11 = context.requestor_x11;
    command.lifecycle = current_.view_epoch ? AP11::AwaitingFocus : AP11::Opening;
    current_ = command;
    return true;
  }
  bool close(uint64_t owner, ap11_gui_message_t &command) {
    if (!owner || current_.native_view != owner)
      return false;
    command = current_;
    command.kind = AP11::Close;
    command.lifecycle = AP11::Closing;
    retired_ = owner;
    current_ = {};
    return true;
  }
  bool status(const ap11_gui_message_t &event) {
    if (!AP11::valid(event) || !event.native_view ||
        event.native_view != current_.native_view ||
        event.activation != current_.activation ||
        event.lifecycle > AP11::EditorFailed ||
        (current_.view_epoch && current_.view_epoch != event.view_epoch) ||
        (event.count && !event.view_epoch))
      return false;
    // A closed editor cannot be resurrected by a delayed focus/status reply.
    if (current_.lifecycle >= AP11::ClosedByVendor && event.count)
      return false;
    current_ = event;
    return true;
  }
  void failed() { current_.lifecycle = AP11::EditorFailed; }
  void sessionRetired() {
    retired_ = allocated_;
    current_ = {};
  }
};
} // namespace AP15
