#pragma once
#include "ap11_gui.h"
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
  virtual void panelClose(uint64_t) = 0;
  virtual uint32_t panelState(uint64_t) const = 0;
  virtual ~PanelOwner() = default;
};
// Qualification candidate: an unmapped, noninteractive X11 lifecycle delegate.
// The host retains its parent and IPlugView. WM_DELETE_WINDOW requests normal
// host retirement; this code never destroys the host-owned parent. Only a
// supplied parent advertising that protocol is admitted by this bounded
// detached-shell adapter. Actual Bitwig acceptance is required separately.
class VendorPanel final : public Steinberg::CPluginView,
                          public Steinberg::Linux::ITimerHandler {
  Steinberg::IPtr<Steinberg::Vst::IEditController> controller_;
  PanelOwner &owner_;
  const uint64_t token_;
  Steinberg::IPtr<Steinberg::Linux::IRunLoop> loop_;
  std::thread::id thread_ = std::this_thread::get_id();
  xcb_connection_t *connection_ = nullptr;
  xcb_window_t parent_ = 0, root_ = 0;
  xcb_atom_t protocols_ = 0, delete_ = 0, active_ = 0, time_ = 0;
  bool registered_ = false, opened_ = false, close_requested_ = false;
  xcb_atom_t atom(const char *name) {
    auto *reply = xcb_intern_atom_reply(connection_, xcb_intern_atom(connection_, false,
                                     uint16_t(std::strlen(name)), name), nullptr);
    auto result = reply ? reply->atom : 0;
    std::free(reply);
    return result;
  }
  bool checked(xcb_void_cookie_t cookie) {
    auto *error = xcb_request_check(connection_, cookie);
    bool ok = !error && !xcb_connection_has_error(connection_);
    std::free(error);
    return ok;
  }
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
    if (!parent_ || close_requested_) return;
    close_requested_ = true;
    xcb_client_message_event_t event{};
    event.response_type = XCB_CLIENT_MESSAGE;
    event.format = 32;
    event.window = parent_;
    event.type = protocols_;
    event.data.data32[0] = delete_;
    event.data.data32[1] = XCB_CURRENT_TIME;
    checked(xcb_send_event_checked(connection_, false, parent_, XCB_EVENT_MASK_NO_EVENT,
                                  reinterpret_cast<const char *>(&event)));
    xcb_flush(connection_);
  }
public:
  VendorPanel(Steinberg::Vst::IEditController *controller, PanelOwner &owner, uint64_t token)
      : controller_(controller), owner_(owner), token_(token) {
    // Nonzero SDK geometry is retained. No child, painting, button, input
    // selection or bridge-control surface is created by the delegate.
    setRect({0, 0, 1, 1});
  }
  ~VendorPanel() override { removed(); }
  Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID id, void **out) override {
    if (!out) return Steinberg::kInvalidArgument;
    if (Steinberg::FUnknownPrivate::iidEqual(id, Steinberg::Linux::ITimerHandler::iid)) {
      *out = static_cast<Steinberg::Linux::ITimerHandler *>(this);
      addRef(); return Steinberg::kResultOk;
    }
    return CPluginView::queryInterface(id, out);
  }
  Steinberg::uint32 PLUGIN_API addRef() override { return CPluginView::addRef(); }
  Steinberg::uint32 PLUGIN_API release() override { return CPluginView::release(); }
  Steinberg::tresult PLUGIN_API setFrame(Steinberg::IPlugFrame *frame) override {
    return thread_ == std::this_thread::get_id() ? CPluginView::setFrame(frame) : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API getSize(Steinberg::ViewRect *size) override {
    return thread_ == std::this_thread::get_id() ? CPluginView::getSize(size) : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API onSize(Steinberg::ViewRect *size) override {
    return thread_ == std::this_thread::get_id() ? CPluginView::onSize(size) : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API checkSizeConstraint(Steinberg::ViewRect *size) override {
    if (thread_ != std::this_thread::get_id() || !size) return Steinberg::kResultFalse;
    *size = {0, 0, 1, 1};
    return Steinberg::kResultOk;
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
    bool accepted = false;
    if (!error && protocols && protocols->type == XCB_ATOM_ATOM && protocols->format == 32 &&
        protocols->value_len <= 16 && !protocols->bytes_after) {
      auto *values = static_cast<xcb_atom_t *>(xcb_get_property_value(protocols));
      for (uint32_t i = 0; i < protocols->value_len; ++i) accepted |= values[i] == delete_;
    }
    std::free(protocols); std::free(error);
    const uint32_t mask = XCB_EVENT_MASK_STRUCTURE_NOTIFY;
    if (!accepted || !checked(xcb_change_window_attributes_checked(connection_, parent_,
                  XCB_CW_EVENT_MASK, &mask)) || loop_->registerTimer(this, 10) != kResultOk) {
      removed(); return kResultFalse;
    }
    registered_ = true;
    systemWindow = parent;
    owner_.panelLoop(loop_);
    const auto context = activation();
    opened_ = true;
    if (!owner_.panelOpen(token_, context)) { removed(); return kResultFalse; }
    return kResultOk;
  }
  Steinberg::tresult PLUGIN_API onFocus(Steinberg::TBool focused) override {
    if (thread_ != std::this_thread::get_id()) return Steinberg::kResultFalse;
    if (focused && opened_ && !close_requested_ && !owner_.panelOpen(token_, activation()))
      return Steinberg::kResultFalse;
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API removed() override {
    if (thread_ != std::this_thread::get_id()) return Steinberg::kResultFalse;
    if (registered_) { registered_ = false; loop_->unregisterTimer(this); }
    if (opened_) { opened_ = false; owner_.panelClose(token_); }
    if (connection_) { xcb_disconnect(connection_); connection_ = nullptr; }
    parent_ = 0;
    loop_ = nullptr;
    systemWindow = nullptr;
    plugFrame = nullptr;
    return Steinberg::kResultOk;
  }
  void PLUGIN_API onTimer() override {
    if (!connection_ || thread_ != std::this_thread::get_id()) return;
    for (unsigned i = 0; i < 32; ++i) {
      auto *event = xcb_poll_for_event(connection_);
      if (!event) break;
      if ((event->response_type & 127) == XCB_MAP_NOTIFY &&
          reinterpret_cast<xcb_map_notify_event_t *>(event)->window == parent_)
        checked(xcb_unmap_window_checked(connection_, parent_));
      std::free(event);
    }
    const auto state = owner_.panelState(token_);
    if (state == ClosedByVendor || state == OpenRefused || state == EditorFailed)
      closeHost();
    xcb_flush(connection_);
  }
};
} // namespace AP11
