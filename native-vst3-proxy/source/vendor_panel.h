#pragma once
#include "ap11_gui.h"
#include "editor_surface_api.h"
#include "ui_timer.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "public.sdk/source/common/pluginview.h"
#include <xcb/xcb.h>
#include <cstdlib>
#include <cstring>
#include <thread>
namespace AP11 {
struct PanelOwner {
  virtual void panelLoop(Steinberg::Linux::IRunLoop *) = 0;
  virtual bool panelOpen(uint64_t, ActivationContext = {}) = 0;
  virtual bool panelClose(uint64_t) = 0;
  virtual void panelFailure(uint64_t, uint32_t) = 0;
  virtual uint32_t panelState(uint64_t) const = 0;
  virtual bool panelSupportsEmbedding() const { return false; }
  virtual bool panelPresentation(uint64_t, ap11_gui_message_t &) const { return false; }
  virtual bool panelPresent(uint64_t, uint32_t) { return false; }
  virtual bool panelFocus(uint64_t, bool) { return false; }
  virtual bool panelSize(uint64_t, Steinberg::ViewRect &, bool) { return false; }
  virtual ~PanelOwner() = default;
};
// Embedded presentation owns a mapped Rust child. Detached fallback retains
// the unmapped lifecycle delegate. All operations run on the captured UI owner.
// The host retains its parent and IPlugView. WM_DELETE_WINDOW requests normal
// host retirement; this code never destroys the host-owned parent. Only a
// supplied parent advertising that protocol may be hidden/retired. Embedded
// host containers stay under host ownership and are never hidden or closed.
// Bitwig requires a valid-size child at attachment;
// the delegate never maps the child or selects input, background or controls.
// Actual Bitwig acceptance is required separately.
enum class ShellOperation { SendClose, Unmap, Flush };
enum class ShellResult { Accepted, Transient, ParentGone, ConnectionLost, Rejected };
struct ShellFaults {
  virtual ShellResult before(ShellOperation) { return ShellResult::Accepted; }
  virtual ~ShellFaults() = default;
};
class VendorPanel final : public Steinberg::CPluginView {
  Steinberg::IPtr<Steinberg::Vst::IEditController> controller_;
  PanelOwner &owner_;
  const uint64_t token_;
  Steinberg::IPtr<Steinberg::Linux::IRunLoop> loop_;
  std::thread::id thread_ = std::this_thread::get_id();
  xcb_connection_t *connection_ = nullptr;
  xcb_window_t parent_ = 0, root_ = 0;
  xcb_atom_t protocols_ = 0, delete_ = 0, active_ = 0, time_ = 0;
  AP15::UiTimer *timer_ = nullptr;
  ShellFaults *faults_;
  // Diagnostic failure is not authority for another XCB operation. In
  // particular, rejected Unmap must not suppress a still-deliverable Close.
  ShellResult shell_result_ = ShellResult::Accepted;
  ShellResult unmap_result_ = ShellResult::Accepted, close_result_ = ShellResult::Accepted;
  ShellResult flush_result_ = ShellResult::Accepted;
  bool opened_ = false, close_requested_ = false, removing_ = false, failed_ = false;
  bool needs_unmap_ = false, parent_gone_ = false, connection_lost_ = false;
  bool closeable_ = false;
  uint64_t surface_ = 0, focused_activation_ = 0;
  LVBEditorPresentation::Record binding_{};
  uint32_t presentation_ = Detached;
  bool fallback_pending_ = false, resizing_frame_ = false;
  unsigned fallback_attempts_ = 0;
  static LVBEditorPresentation::Record presentationRecord(const ap11_gui_message_t &m, uint32_t kind) {
    LVBEditorPresentation::Record record{};
    record.kind = kind; record.presentation = m.presentation;
    record.owner = m.native_view; record.epoch = m.view_epoch; record.window = m.target_x11;
    record.width = m.width; record.height = m.height;
    return record;
  }
  void detachSurface() {
    const auto handle = surface_; surface_ = 0; binding_ = {};
    if (handle) lvb_editor_surface_release(handle);
  }
  void fallback() {
    detachSurface(); // retire before asking Wine to restore decorations
    presentation_ = Detached; fallback_pending_ = true; fallback_attempts_ = 0;
  }
  void updatePresentation() {
    if (fallback_pending_) {
      if (owner_.panelPresent(token_, Detached)) { fallback_pending_ = false; needs_unmap_ = closeable_; }
      else if (++fallback_attempts_ == 3) { fallback_pending_ = false; failure(ShellResult::Rejected); }
      return;
    }
    ap11_gui_message_t m{};
    if (!surface_ || !owner_.panelPresentation(token_, m)) return;
    if (m.presentation != EmbeddedX11) { detachSurface(); presentation_ = Detached; needs_unmap_ = closeable_; return; }
    if (!m.target_x11 || !m.width || !m.height || m.width > 8192 || m.height > 8192) { fallback(); return; }
    auto record = presentationRecord(m, LVBEditorPresentation::WindowReady);
    if (!binding_.window) {
      if (lvb_editor_surface_bind(surface_, &record)) { fallback(); return; }
      binding_ = record;
    } else if (binding_.owner != record.owner || binding_.epoch != record.epoch || binding_.window != record.window) {
      fallback(); return;
    }
    const auto current = getRect();
    if (current.getWidth() != int32_t(m.width) || current.getHeight() != int32_t(m.height)) {
      record.kind = LVBEditorPresentation::WindowResized;
      if (lvb_editor_surface_resize(surface_, &record)) { fallback(); return; }
      Steinberg::ViewRect size{0, 0, int32_t(m.width), int32_t(m.height)};
      resizing_frame_ = true;
      const auto result = plugFrame ? plugFrame->resizeView(this, &size) : Steinberg::kResultFalse;
      resizing_frame_ = false;
      if (result != Steinberg::kResultOk) { fallback(); return; }
      setRect(size); binding_.width = m.width; binding_.height = m.height;
    }
    if (m.focus_result == FocusPending && m.activation != focused_activation_) {
      const bool accepted = lvb_editor_surface_focus(surface_) == 0;
      if (owner_.panelFocus(token_, accepted)) focused_activation_ = m.activation;
    }
  }
  unsigned close_attempts_ = 0, unmap_attempts_ = 0, close_wait_ = 0;
  ShellResult before(ShellOperation op) {
    if (xcb_connection_has_error(connection_)) return ShellResult::ConnectionLost;
    return faults_ ? faults_->before(op) : ShellResult::Accepted;
  }
  ShellResult flush() {
    const auto result = before(ShellOperation::Flush);
    flush_result_ = result != ShellResult::Accepted ? result :
      xcb_flush(connection_) > 0 ? ShellResult::Accepted : ShellResult::ConnectionLost;
    return flush_result_;
  }
  void failure(ShellResult result) {
    shell_result_ = result;
    parent_gone_ |= result == ShellResult::ParentGone;
    connection_lost_ |= result == ShellResult::ConnectionLost;
    if (!failed_) {
      failed_ = true;
      owner_.panelFailure(token_, parent_gone_ ? WindowLost : Host);
    }
  }
  xcb_atom_t atom(const char *name) {
    auto *reply = xcb_intern_atom_reply(connection_, xcb_intern_atom(connection_, false,
                                     uint16_t(std::strlen(name)), name), nullptr);
    auto result = reply ? reply->atom : 0;
    std::free(reply);
    return result;
  }
  ShellResult checkedResult(xcb_void_cookie_t cookie) {
    auto *error = xcb_request_check(connection_, cookie);
    auto ok = xcb_connection_has_error(connection_) ? ShellResult::ConnectionLost :
      !error ? ShellResult::Accepted : error->error_code == XCB_WINDOW ?
      ShellResult::ParentGone : error->error_code == XCB_ALLOC ? ShellResult::Transient : ShellResult::Rejected;
    std::free(error);
    return ok;
  }
  bool checked(xcb_void_cookie_t cookie) { return checkedResult(cookie) == ShellResult::Accepted; }
  uint32_t property(xcb_window_t window, xcb_atom_t name, xcb_atom_t type) {
    xcb_generic_error_t *error = nullptr;
    auto *reply = xcb_get_property_reply(connection_,
        xcb_get_property(connection_, false, window, name, type, 0, 1), &error);
    uint32_t result = 0;
    if (!error && reply && reply->format == 32 && reply->type == type &&
        reply->value_len == 1 && !reply->bytes_after)
      std::memcpy(&result, xcb_get_property_value(reply), 4);
    std::free(error); std::free(reply);
    return result;
  }
  ActivationContext activation() {
    auto active = property(root_, active_, XCB_ATOM_WINDOW);
    auto time = active ? property(active, time_, XCB_ATOM_CARDINAL) : 0;
    return {time, active};
  }
  void closeHost() {
    if (presentation_ == EmbeddedX11) return;
    if (!closeable_) { close_requested_ = true; return; }
    if (!parent_ || parent_gone_ || close_requested_ || close_attempts_ >= 3 ||
        connection_lost_ || close_result_ == ShellResult::Rejected) return;
    ++close_attempts_;
    auto result = before(ShellOperation::SendClose);
    if (result == ShellResult::Accepted) {
      xcb_client_message_event_t event{};
      event.response_type = XCB_CLIENT_MESSAGE; event.format = 32;
      event.window = parent_; event.type = protocols_;
      event.data.data32[0] = delete_; event.data.data32[1] = XCB_CURRENT_TIME;
      result = checkedResult(xcb_send_event_checked(connection_, false, parent_,
          XCB_EVENT_MASK_NO_EVENT, reinterpret_cast<const char *>(&event)));
    }
    close_result_ = result;
    if (result == ShellResult::Accepted) {
      // Server acceptance is final for delivery, even if the later flush is
      // ambiguous. Host retirement is separate and must never resend this.
      close_requested_ = true;
      if (flush() != ShellResult::Accepted) failure(flush_result_);
    } else if (result != ShellResult::Transient || close_attempts_ == 3)
      failure(result);
  }

public:
  VendorPanel(Steinberg::Vst::IEditController *controller, PanelOwner &owner, uint64_t token, ShellFaults *faults = nullptr)
      : controller_(controller), owner_(owner), token_(token), faults_(faults) {
    // Bitwig rejects 1x1 geometry before waiting for the attached child.
    // This is attachment geometry only: no mapped or interactive surface.
    setRect({0, 0, 64, 64});
  }
  ~VendorPanel() override {
    removed();
    detachSurface(); // native presentation retirement never depends on vendor/timer acceptance
    if (timer_) { timer_->detach(); timer_->release(); timer_ = nullptr; }
    if (connection_) xcb_disconnect(connection_);
  }
  ShellResult shellResult() const { return shell_result_; }
  Steinberg::tresult PLUGIN_API setFrame(Steinberg::IPlugFrame *frame) override {
    return thread_ == std::this_thread::get_id() ? CPluginView::setFrame(frame) : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API getSize(Steinberg::ViewRect *size) override {
    return thread_ == std::this_thread::get_id() ? CPluginView::getSize(size) : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API onSize(Steinberg::ViewRect *size) override {
    if (thread_ != std::this_thread::get_id() || !size) return Steinberg::kResultFalse;
    if (surface_) {
      auto requested = *size;
      if (!binding_.window) return requested.getWidth() == getRect().getWidth() &&
        requested.getHeight() == getRect().getHeight() ? CPluginView::onSize(size) : Steinberg::kResultFalse;
      if (!resizing_frame_ && !owner_.panelSize(token_, requested, true)) return Steinberg::kResultFalse;
      auto record = binding_; record.kind = LVBEditorPresentation::WindowResized;
      record.width = uint32_t(requested.getWidth()); record.height = uint32_t(requested.getHeight());
      if (binding_.window && lvb_editor_surface_resize(surface_, &record)) { fallback(); return Steinberg::kResultFalse; }
      *size = requested;
    }
    return CPluginView::onSize(size);
  }
  Steinberg::tresult PLUGIN_API checkSizeConstraint(Steinberg::ViewRect *size) override {
    if (thread_ != std::this_thread::get_id() || !size) return Steinberg::kResultFalse;
    if (surface_) return owner_.panelSize(token_, *size, false) ? Steinberg::kResultOk : Steinberg::kResultFalse;
    *size = {0, 0, 64, 64};
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API canResize() override {
    ap11_gui_message_t m{};
    return thread_ == std::this_thread::get_id() && surface_ && owner_.panelPresentation(token_, m) &&
      (m.presentation_flags & CanResize) ? Steinberg::kResultTrue : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API isPlatformTypeSupported(Steinberg::FIDString type) override {
    return type && !std::strcmp(type, Steinberg::kPlatformTypeX11EmbedWindowID)
      ? Steinberg::kResultOk : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API attached(void *parent, Steinberg::FIDString type) override {
    using namespace Steinberg;
    if (thread_ != std::this_thread::get_id() || !token_ || connection_ || !parent ||
        isPlatformTypeSupported(type) != kResultOk)
      return kResultFalse;
    loop_ = FUnknownPtr<Linux::IRunLoop>(plugFrame);
    if (!loop_) return kResultFalse;
    int screen = 0;
    connection_ = xcb_connect(nullptr, &screen);
    if (!connection_ || xcb_connection_has_error(connection_)) { removed(); return kResultFalse; }
    auto screens = xcb_setup_roots_iterator(xcb_get_setup(connection_));
    for (; screen > 0 && screens.rem; --screen) xcb_screen_next(&screens);
    if (!screens.rem) { removed(); return kResultFalse; }
    root_ = screens.data->root;
    parent_ = xcb_window_t(reinterpret_cast<uintptr_t>(parent));
    protocols_ = atom("WM_PROTOCOLS"); delete_ = atom("WM_DELETE_WINDOW");
    active_ = atom("_NET_ACTIVE_WINDOW"); time_ = atom("_NET_WM_USER_TIME");
    xcb_generic_error_t *error = nullptr;
    auto *protocols = xcb_get_property_reply(connection_,
        xcb_get_property(connection_, false, parent_, protocols_, XCB_ATOM_ATOM, 0, 16), &error);
    closeable_ = false;
    if (!error && protocols && protocols->type == XCB_ATOM_ATOM && protocols->format == 32 &&
        protocols->value_len <= 16 && !protocols->bytes_after) {
      auto *values = static_cast<xcb_atom_t *>(xcb_get_property_value(protocols));
      for (uint32_t i = 0; i < protocols->value_len; ++i) closeable_ |= values[i] == delete_;
    }
    std::free(protocols); std::free(error);
    const uint32_t mask = XCB_EVENT_MASK_STRUCTURE_NOTIFY;
    if (!checked(xcb_change_window_attributes_checked(connection_, parent_,
                  XCB_CW_EVENT_MASK, &mask)) ) {
      removed(); return kResultFalse;
    }
    // Bitwig checks for a child after attached(), even for a detached editor.
    // Own this XID through this private connection: disconnect retires it only
    // after the timer is stopped. Never map it, select input, or paint it.
    if (owner_.panelSupportsEmbedding() && !lvb_editor_surface_create(LVBEditor::SurfaceAbi,
          parent_, token_, 64, 64, &surface_)) presentation_ = EmbeddedX11;
    const auto child = surface_ ? 0 : xcb_generate_id(connection_);
    if (!surface_ && !checked(xcb_create_window_checked(connection_, XCB_COPY_FROM_PARENT, child,
        parent_, 0, 0, 64, 64, 0, XCB_WINDOW_CLASS_INPUT_OUTPUT,
        XCB_COPY_FROM_PARENT, 0, nullptr))) {
      removed(); return kResultFalse;
    }
    timer_ = new AP15::UiTimer(loop_, this, [](void *p) { static_cast<VendorPanel *>(p)->onTimer(); });
    if (!timer_->start()) { removed(); return kResultFalse; }
    removing_ = close_requested_ = failed_ = parent_gone_ = connection_lost_ = needs_unmap_ = false;
    close_attempts_ = unmap_attempts_ = close_wait_ = 0;
    shell_result_ = unmap_result_ = close_result_ = flush_result_ = ShellResult::Accepted;
    systemWindow = parent;
    owner_.panelLoop(loop_);
    auto context = activation();
    context.presentation = presentation_;
    opened_ = true;
    if (!owner_.panelOpen(token_, context)) { removed(); return kResultFalse; }
    return kResultOk;
  }
  Steinberg::tresult PLUGIN_API onFocus(Steinberg::TBool focused) override {
    if (thread_ != std::this_thread::get_id()) return Steinberg::kResultFalse;
    if (!focused || !opened_ || !connection_ || removing_ || close_requested_ || failed_ ||
        owner_.panelState(token_) >= ClosedByVendor) return Steinberg::kResultOk;
    auto context = activation(); context.presentation = presentation_;
    if (!owner_.panelOpen(token_, context)) return Steinberg::kResultFalse;
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API removed() override {
    if (thread_ != std::this_thread::get_id()) return Steinberg::kResultFalse;
    removing_ = true;
    const bool was_embedded = surface_ != 0;
    detachSurface(); // restore before a host may destroy its parent, even on refusal
    if (opened_) {
      if (!owner_.panelClose(token_)) {
        // The Windows view has not positively retired. Request its usable
        // detached presentation independently; keep vendor ownership pending.
        if (was_embedded) owner_.panelPresent(token_, Detached);
        return Steinberg::kResultFalse;
      }
      opened_ = false;
    }
    if (timer_) {
      if (!timer_->stop()) return Steinberg::kResultFalse;
      timer_->release(); timer_ = nullptr;
    }
    detachSurface();
    if (connection_) { xcb_disconnect(connection_); connection_ = nullptr; }
    parent_ = 0;
    closeable_ = false;
    loop_ = nullptr;
    systemWindow = nullptr;
    plugFrame = nullptr;
    return Steinberg::kResultOk;
  }
  void onTimer() {
    if (thread_ != std::this_thread::get_id()) return;
    if (removing_) { removed(); return; }
    if (!connection_) return;
    if (xcb_connection_has_error(connection_)) failure(ShellResult::ConnectionLost);
    if (!failed_) updatePresentation();
    for (unsigned i = 0; i < 32; ++i) {
      auto *event = xcb_poll_for_event(connection_);
      if (!event) break;
      const auto type = event->response_type & 127;
      if (presentation_ == Detached && closeable_ && type == XCB_MAP_NOTIFY && reinterpret_cast<xcb_map_notify_event_t *>(event)->window == parent_)
        needs_unmap_ = true;
      if (type == XCB_DESTROY_NOTIFY && reinterpret_cast<xcb_destroy_notify_event_t *>(event)->window == parent_)
        failure(ShellResult::ParentGone);
      std::free(event);
    }
    if (presentation_ == Detached && needs_unmap_ && !parent_gone_ && !connection_lost_ && unmap_attempts_ < 3 &&
        unmap_result_ != ShellResult::Rejected) {
      ++unmap_attempts_;
      auto result = before(ShellOperation::Unmap);
      if (result == ShellResult::Accepted) result = checkedResult(xcb_unmap_window_checked(connection_, parent_));
      unmap_result_ = result;
      if (result == ShellResult::Accepted) {
        needs_unmap_ = false; unmap_attempts_ = 0;
        if (flush() != ShellResult::Accepted) failure(flush_result_);
      }
      else if (result != ShellResult::Transient || unmap_attempts_ == 3) failure(result);
    }
    const auto state = owner_.panelState(token_);
    if (failed_ || state == ClosedByVendor || state == OpenRefused || state == EditorFailed) closeHost();
    // XCB cannot acknowledge host action. One accepted request is never
    // repeated; an unretired shell after 100 UI ticks is a truthful refusal.
    if (presentation_ == Detached && closeable_ && close_requested_ && close_wait_ < 100 && ++close_wait_ == 100) failure(ShellResult::Rejected);
    if (!connection_lost_ && flush() != ShellResult::Accepted) failure(flush_result_);
  }
};
} // namespace AP11
