#pragma once
#include "ap11_gui.h"
#include <cstdint>
namespace AP15 {
inline uint64_t advance(uint64_t &counter) {
  return counter == UINT64_MAX ? 0 : ++counter;
}
// Native UI owner only. Object addresses, DSP generations, editor epochs and
// activation serials are deliberately not interchangeable identities.
class EditorLifecycle {
  uint64_t allocated_ = 0, retired_ = 0;
  uint64_t retired_activation_ = 0;
  uint32_t retired_epoch_ = 0;
  uint32_t last_epoch_ = 0;
  ap11_gui_message_t current_{};
public:
  uint64_t allocate() {
    return advance(allocated_);
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
    retired_activation_ = command.activation;
    retired_epoch_ = command.view_epoch;
    current_ = {};
    return true;
  }
  bool accepts_result(const ap11_gui_message_t &m) const {
    if (m.kind == AP11::Close)
      return !current_.native_view && m.native_view == retired_ &&
             m.activation == retired_activation_ && m.view_epoch == retired_epoch_;
    if (m.kind == AP11::Open || m.kind == AP11::Focus)
      return m.native_view == current_.native_view && m.activation == current_.activation;
    return true;
  }
  bool status(const ap11_gui_message_t &event) {
    if (!AP11::valid(event) || event.kind != AP11::EditorStatus || !event.native_view ||
        event.native_view != current_.native_view ||
        event.activation != current_.activation ||
        event.count > 1 ||
        event.lifecycle > AP11::EditorFailed ||
        (current_.view_epoch && current_.view_epoch != event.view_epoch) ||
        (event.count && (!event.view_epoch || (!current_.view_epoch && event.view_epoch <= last_epoch_))))
      return false;
    const bool open = event.lifecycle == AP11::Opened || event.lifecycle == AP11::AwaitingFocus ||
                      event.lifecycle == AP11::Focused || event.lifecycle == AP11::FocusRefused;
    const bool closed = event.lifecycle == AP11::ClosedByVendor || event.lifecycle == AP11::ClosedByDaw ||
                        event.lifecycle == AP11::OpenRefused;
    if ((!open && !closed && event.lifecycle != AP11::EditorFailed) ||
        (open && event.count != 1) || (closed && event.count != 0))
      return false;
    // A closed editor cannot be resurrected by a delayed focus/status reply.
    if (current_.lifecycle >= AP11::ClosedByVendor && event.count)
      return false;
    if (event.view_epoch > last_epoch_) last_epoch_ = event.view_epoch;
    current_ = event;
    return true;
  }
  void failed() { current_.lifecycle = AP11::EditorFailed; }
  void sessionRetired() {
    retired_ = allocated_;
    last_epoch_ = 0;
    retired_activation_ = 0;
    retired_epoch_ = 0;
    current_ = {};
  }
};
} // namespace AP15
