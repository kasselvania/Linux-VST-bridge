#pragma once
#include "ap11_gui.h"
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <xcb/xcb.h>
#include <xcb/xcbext.h>

namespace AP11 {
// Native UI-thread only. Normal EWMH application activation, never a pager
// impersonation, XSetInputFocus, synthetic input or an always-on-top request.
// Checked asynchronous XCB replies avoid process-global Xlib error handlers
// and keep destroyed/stale target windows from terminating the DAW.
class DesktopActivation {
  using Clock = std::chrono::steady_clock;
  enum class Stage { Idle, Target, Atoms, Source, Active, Focus, Parent, Done };
  xcb_connection_t *connection_ = nullptr;
  xcb_window_t root_ = 0;
  xcb_atom_t active_ = 0;
  Stage stage_ = Stage::Idle;
  unsigned sequence_ = 0, depth_ = 0;
  Clock::time_point deadline_{};
  ap11_gui_message_t request_{};
  bool delivered_ = false;

  void discard() {
    if (sequence_ && connection_)
      xcb_discard_reply(connection_, sequence_);
    sequence_ = 0;
  }
  void finish(uint32_t outcome) {
    discard();
    request_.focus_result = outcome;
    stage_ = Stage::Done;
  }
  void active(Stage stage) {
    stage_ = stage;
    sequence_ = xcb_get_property(connection_, false, root_, active_,
                                 XCB_ATOM_WINDOW, 0, 1)
                    .sequence;
    xcb_flush(connection_);
  }
  bool connect() {
    if (connection_)
      return !xcb_connection_has_error(connection_);
    int screen = 0;
    connection_ = xcb_connect(nullptr, &screen);
    if (!connection_ || xcb_connection_has_error(connection_))
      return false;
    auto i = xcb_setup_roots_iterator(xcb_get_setup(connection_));
    for (; screen > 0 && i.rem; --screen)
      xcb_screen_next(&i);
    if (!i.rem)
      return false;
    root_ = i.data->root;
    return true;
  }

public:
  ~DesktopActivation() {
    discard();
    if (connection_)
      xcb_disconnect(connection_);
  }
  DesktopActivation() = default;
  DesktopActivation(const DesktopActivation &) = delete;
  DesktopActivation &operator=(const DesktopActivation &) = delete;
  void cancel() {
    discard();
    stage_ = Stage::Idle;
    request_ = {};
    delivered_ = false;
  }
  void begin(const ap11_gui_message_t &request) {
    cancel();
    request_ = request;
    request_.focus_result = FocusPending;
    stage_ = Stage::Target;
    deadline_ = Clock::now() + std::chrono::seconds(2);
  }
  void target(const ap11_gui_message_t &reply) {
    if (!AP11::valid(reply) || reply.native_view != request_.native_view ||
        reply.activation != request_.activation ||
        (request_.view_epoch && reply.view_epoch != request_.view_epoch))
      return;
    if (stage_ == Stage::Target && Clock::now() >= deadline_)
      finish(FocusDenied);
    // A slow vendor attachment can outlive the target-response deadline. The
    // earlier timeout has no target/epoch for Windows to accept. Bind its
    // negative result to the late owned target, without attempting activation
    // with expired user intent, so neither end remains "requesting focus".
    if (stage_ == Stage::Done && request_.focus_result == FocusDenied &&
        !request_.target_x11 && reply.target_x11 && reply.view_epoch) {
      request_.target_x11 = reply.target_x11;
      request_.view_epoch = reply.view_epoch;
      delivered_ = false;
      return;
    }
    if (stage_ != Stage::Target)
      return;
    request_.target_x11 = reply.target_x11;
    request_.view_epoch = reply.view_epoch;
    if (!request_.user_time || !request_.requestor_x11 || !reply.target_x11 ||
        !reply.view_epoch || !connect()) {
      finish(FocusUnsupported);
      return;
    }
    // Nothing is remapped or raised here. The real click's timestamp and
    // requestor are transferred to the WM with the session-owned target XID.
    deadline_ = Clock::now() + std::chrono::milliseconds(750);
    if (active_)
      active(Stage::Source);
    else {
      const char name[] = "_NET_ACTIVE_WINDOW";
      sequence_ =
          xcb_intern_atom(connection_, true, sizeof(name) - 1, name).sequence;
      stage_ = Stage::Atoms;
      xcb_flush(connection_);
    }
  }
  // At most four ready replies per host UI tick. No wait for the X server or
  // WM.
  bool poll(ap11_gui_message_t &out) {
    if (stage_ == Stage::Idle || delivered_)
      return false;
    if (stage_ != Stage::Done && Clock::now() >= deadline_)
      finish(FocusDenied);
    for (unsigned step = 0; step < 4 && sequence_; ++step) {
      if (xcb_connection_has_error(connection_)) {
        finish(FocusUnsupported);
        break;
      }
      void *raw = nullptr;
      xcb_generic_error_t *error = nullptr;
      if (!xcb_poll_for_reply(connection_, sequence_, &raw, &error))
        break;
      sequence_ = 0;
      if (error || !raw) {
        std::free(error);
        std::free(raw);
        finish(FocusCancelled);
        break;
      }
      auto stage = stage_;
      if (stage == Stage::Atoms) {
        active_ = static_cast<xcb_intern_atom_reply_t *>(raw)->atom;
        if (!active_)
          finish(FocusUnsupported);
        else
          active(Stage::Source);
      } else if (stage == Stage::Source || stage == Stage::Active) {
        auto *reply = static_cast<xcb_get_property_reply_t *>(raw);
        uint32_t window = 0;
        if (reply->type == XCB_ATOM_WINDOW && reply->format == 32 &&
            reply->value_len == 1 && !reply->bytes_after)
          std::memcpy(&window, xcb_get_property_value(reply), sizeof(window));
        if (window == request_.target_x11) {
          sequence_ = xcb_get_input_focus(connection_).sequence;
          stage_ = Stage::Focus;
        } else if (window != request_.requestor_x11) {
          // The user has moved elsewhere since clicking. Never steal it back.
          finish(FocusDenied);
        } else if (stage == Stage::Source) {
          xcb_client_message_event_t event{};
          event.response_type = XCB_CLIENT_MESSAGE;
          event.format = 32;
          event.window = request_.target_x11;
          event.type = active_;
          event.data.data32[0] = 1; // application, not pager
          event.data.data32[1] = request_.user_time;
          event.data.data32[2] = request_.requestor_x11;
          // The target may disappear after its reply. SendEvent addresses the
          // stable root, and subsequent checked reads detect disappearance.
          xcb_send_event(connection_, false, root_,
                         XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT |
                             XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY,
                         reinterpret_cast<const char *>(&event));
          active(Stage::Active);
        } else
          active(Stage::Active);
      } else if (stage == Stage::Focus) {
        auto focus = static_cast<xcb_get_input_focus_reply_t *>(raw)->focus;
        depth_ = 0;
        if (focus == request_.target_x11)
          finish(FocusConfirmed);
        else if (focus <= 1 || focus == root_)
          active(Stage::Active);
        else {
          sequence_ = xcb_query_tree(connection_, focus).sequence;
          stage_ = Stage::Parent;
        }
      } else if (stage == Stage::Parent) {
        auto parent = static_cast<xcb_query_tree_reply_t *>(raw)->parent;
        if (parent == request_.target_x11)
          finish(FocusConfirmed);
        else if (!parent || parent == root_ || ++depth_ == 32)
          active(Stage::Active);
        else
          sequence_ = xcb_query_tree(connection_, parent).sequence;
      }
      std::free(raw);
      xcb_flush(connection_);
    }
    if (stage_ != Stage::Done || delivered_)
      return false;
    delivered_ = true;
    out = request_;
    return true;
  }
};
} // namespace AP11
