// Pinned SDK objects driving production handler, editor lifecycle and GUI
// mapping. This is deterministic instrumentation, not commercial evidence.
#include "controller_updates.h"
#include "editor_session.h"
#include "public.sdk/source/common/pluginview.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "vendor_handler.h"
#include "ui_apartment.h"
#include <filesystem>
#include <iostream>
#include <memory>
using namespace Steinberg;
using namespace Steinberg::Vst;
using namespace linux_vst_bridge::wf0;
namespace {
void check(bool v, const char *why) {
  if (!v) {
    std::cerr << "FAIL: " << why << '\n';
    std::exit(1);
  }
}
std::thread::id owner = std::this_thread::get_id();
struct Stats {
  uint32_t created = 0, attached = 0, removed = 0, destroyed = 0, focus = 0,
           keys = 0, sizes = 0;
  bool refuse = false;
};
struct View final : CPluginView, IPlugViewContentScaleSupport {
  Stats &stats;
  explicit View(Stats &s) : stats(s) {
    ++stats.created;
    setRect({0, 0, 400, 240});
  }
  ~View() override {
    check(!systemWindow, "view removed before release");
    check(std::this_thread::get_id() == owner, "view destruction owner");
    ++stats.destroyed;
  }
  tresult PLUGIN_API queryInterface(const TUID id, void **out) override {
    if (!out)
      return kInvalidArgument;
    if (FUnknownPrivate::iidEqual(id, IPlugViewContentScaleSupport::iid)) {
      *out = static_cast<IPlugViewContentScaleSupport *>(this);
      addRef();
      return kResultOk;
    }
    return CPluginView::queryInterface(id, out);
  }
  uint32 PLUGIN_API addRef() override { return CPluginView::addRef(); }
  uint32 PLUGIN_API release() override { return CPluginView::release(); }
  tresult PLUGIN_API isPlatformTypeSupported(FIDString type) override {
    return type && !std::strcmp(type, kPlatformTypeHWND) ? kResultOk
                                                         : kResultFalse;
  }
  tresult PLUGIN_API setFrame(IPlugFrame *frame) override {
    auto r = CPluginView::setFrame(frame);
    if (frame) {
      ViewRect same{};
      check(getSize(&same) == kResultOk &&
                frame->resizeView(this, &same) == kResultOk,
            "same-size request during frame installation");
    }
    return r;
  }
  tresult PLUGIN_API attached(void *parent, FIDString type) override {
    check(std::this_thread::get_id() == owner && IsWindow(HWND(parent)) &&
              plugFrame,
          "attached local parent and frame on owner");
    check(isPlatformTypeSupported(type) == kResultOk, "HWND negotiation");
    systemWindow = parent;
    ++stats.attached;
    return kResultOk;
  }
  tresult PLUGIN_API removed() override {
    check(std::this_thread::get_id() == owner && IsWindow(HWND(systemWindow)),
          "remove before destroying parent");
    if (stats.refuse)
      return kResultFalse;
    systemWindow = nullptr;
    ++stats.removed;
    return kResultOk;
  }
  tresult PLUGIN_API onSize(ViewRect *r) override {
    check(systemWindow, "no platform sizing before attached");
    ++stats.sizes;
    return CPluginView::onSize(r);
  }
  tresult PLUGIN_API onFocus(TBool) override {
    ++stats.focus;
    return kResultOk;
  }
  tresult PLUGIN_API onKeyDown(char16, int16, int16) override {
    ++stats.keys;
    return kResultOk;
  }
  tresult PLUGIN_API onKeyUp(char16, int16, int16) override {
    ++stats.keys;
    return kResultOk;
  }
  tresult PLUGIN_API canResize() override { return kResultFalse; }
  tresult requestSize(ViewRect &r) { return plugFrame->resizeView(this, &r); }
  tresult PLUGIN_API setContentScaleFactor(float scale) override {
    check(!plugFrame, "initial scale before frame installation");
    ViewRect r{0, 0, int32(400 * scale), int32(240 * scale)};
    setRect(r);
    return plugFrame ? plugFrame->resizeView(this, &r) : kResultOk;
  }
};
struct Controller final : EditController {
  Stats stats;
  bool echo = true, no_view = false;
  View *last_view = nullptr;
  tresult PLUGIN_API initialize(FUnknown *h) override {
    auto r = EditController::initialize(h);
    parameters.addParameter(u"Gain", u"", 0, .5, ParameterInfo::kCanAutomate,
                            42);
    return r;
  }
  IPlugView *PLUGIN_API createView(FIDString name) override {
    check(std::this_thread::get_id() == owner &&
              !std::strcmp(name, ViewType::kEditor),
          "same SDK controller creates editor");
    return no_view ? nullptr : (last_view = new View(stats));
  }
  tresult PLUGIN_API setParamNormalized(ParamID id, ParamValue value) override {
    check(std::this_thread::get_id() == owner, "controller update owner");
    auto r = EditController::setParamNormalized(id, value);
    if (echo && componentHandler) {
      check(componentHandler->beginEdit(id) == kResultOk,
            "host-update echo begin suppressed");
      check(componentHandler->performEdit(id, value) == kResultOk,
            "host-update echo value suppressed");
      check(componentHandler->endEdit(id) == kResultOk,
            "host-update echo end suppressed");
    }
    return r;
  }
};
struct Mapping {
  static constexpr size_t bytes = 256 + 2 * 512 * sizeof(ap11_gui_message_t);
  std::filesystem::path dir;
  HANDLE file = INVALID_HANDLE_VALUE, map = nullptr;
  uint8_t *data = nullptr;
  std::array<uint8_t, 16> id{};
  Mapping() {
    dir = std::filesystem::temp_directory_path() /
          (L"ap11-sdk-" + std::to_wstring(GetCurrentProcessId()) + L"-" +
           std::to_wstring(GetTickCount64()));
    check(std::filesystem::create_directory(dir), "private fixture directory");
    id[0] = 19;
    file = CreateFileW((dir / L"ap11.ui").c_str(), GENERIC_READ | GENERIC_WRITE,
                       FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, CREATE_NEW,
                       0, nullptr);
    check(file != INVALID_HANDLE_VALUE, "backing create");
    LARGE_INTEGER length{};
    length.QuadPart = bytes;
    check(SetFilePointerEx(file, length, nullptr, FILE_BEGIN) &&
              SetEndOfFile(file),
          "backing size");
    map = CreateFileMappingW(file, nullptr, PAGE_READWRITE, 0, 0, nullptr);
    data = static_cast<uint8_t *>(
        MapViewOfFile(map, FILE_MAP_ALL_ACCESS, 0, 0, bytes));
    check(data, "native mapping");
    std::memset(data, 0, bytes);
    std::memcpy(data, "LVBU", 4);
    put(4, 1, 4);
    put(8, bytes, 4);
    put(12, 552, 4);
    std::copy(id.begin(), id.end(), data + 16);
    put(32, 512, 4);
    put(96, 1, 8);
    put(136, 7, 4);
  }
  ~Mapping() {
    UnmapViewOfFile(data);
    CloseHandle(map);
    CloseHandle(file);
    std::filesystem::remove(dir / L"ap11.ui");
    std::filesystem::remove(dir);
  }
  void put(size_t off, uint64_t value, size_t n) {
    linux_vst_bridge::ap1::put(data + off, value, n);
  }
  auto word(size_t off) {
    return std::atomic_ref<uint64_t>(*reinterpret_cast<uint64_t *>(data + off));
  }
  void command(uint32_t kind, double value = 0, uint64_t revision = 0) {
    auto p = word(64).load(), c = word(72).load();
    check(p - c < 512, "command bound");
    ap11_gui_message_t m{};
    m.kind = kind;
    m.id = 42;
    m.value = value;
    m.revision = revision;
    std::memcpy(data + 256 + p % 512 * sizeof(m), &m, sizeof(m));
    word(64).store(p + 1, std::memory_order_release);
  }
  std::vector<ap11_gui_message_t> drain() {
    std::vector<ap11_gui_message_t> result;
    auto p = word(80).load(std::memory_order_acquire), c = word(88).load();
    while (c < p) {
      ap11_gui_message_t m{};
      std::memcpy(&m, data + 256 + (512 + c % 512) * sizeof(m), sizeof(m));
      result.push_back(m);
      ++c;
    }
    word(88).store(c, std::memory_order_release);
    return result;
  }
  void close() {
    word(152).store(word(64).load(), std::memory_order_release);
    word(120).fetch_add(1, std::memory_order_acq_rel);
  }
};
struct External final : ExternalProcessing {
  EditorSession *session = nullptr;
  tresult editor_edit(uint32_t kind, uint32_t id, double value) override {
    return session ? session->edit(kind, id, value) : kNotImplemented;
  }
  tresult request_restart(int32_t flags) override {
    return session ? session->restart(flags) : kNotImplemented;
  }
  void ready() override {}
  bool next(ExternalBlock &, float *, float *) override { return false; }
  void done(const float *, const float *, uint64_t, uint64_t,
            const ap10_results_t *) override {}
};
} // namespace
int main() {
  UiApartment apartment;
  APTTYPE type{};
  APTTYPEQUALIFIER qualifier{};
  check(CoGetApartmentType(&type, &qualifier) == S_OK &&
            (type == APTTYPE_STA || type == APTTYPE_MAINSTA),
        "controller and view own a Windows STA apartment");
  HostApplication host;
  auto *c = new Controller;
  check(c->initialize(&host) == kResultOk, "controller initialize");
  Mapping native;
  GuiChannel channel(native.dir.wstring(), native.id);
  EditorSession session(channel, *c);
  External external;
  external.session = &session;
  VendorHandler handler;
  handler.external = &external;
  check(c->setComponentHandler(&handler) == kResultOk,
        "production SDK handler");
  check(c->stats.created == 0, "no editor on scan/restore");
  c->no_view = true;
  native.command(AP11::Open);
  session.service(true);
  auto denied = native.drain();
  check(!session.is_open() && channel.failure() == 0 &&
            std::any_of(denied.begin(), denied.end(), [](const auto &m) {
              return m.kind == AP11::EditorStatus && m.result == AP11::NoView;
            }),
        "ordinary open failure explicit without poisoning session");
  c->no_view = false;
  native.command(AP11::Open);
  session.service(true);
  check(session.is_open() && c->stats.created == 1 && c->stats.attached == 1,
        "actual SDK attach");
  check(c->stats.sizes == 0, "unchanged size does not call onSize");
  ViewRect changed{0, 0, 440, 260};
  check(c->last_view->requestSize(changed) == kResultOk && c->stats.sizes == 1,
        "changed size still reaches attached view synchronously");
  check(c->last_view->requestSize(changed) == kResultOk && c->stats.sizes == 1,
        "repeated unchanged request does not duplicate onSize");
  native.drain();
  native.command(AP11::Open);
  session.service(true);
  check(c->stats.created == 1 && session.view().focuses == 1,
        "open focuses existing view");
  SendMessageW(session.view().window(), WM_KEYDOWN, VK_LEFT, 0);
  SendMessageW(session.view().window(), WM_KEYUP, VK_LEFT, 0);
  SendMessageW(session.view().window(), WM_CHAR, L'a', 0);
  SendMessageW(session.view().window(), WM_CHAR, L'z', 0);
  check(c->stats.keys == 4 && c->stats.focus > 0,
        "focus and parent keyboard forwarding");
  native.drain();
  FUnknownPtr<IComponentHandler2> grouped(
      static_cast<IComponentHandler *>(&handler));
  check(grouped && grouped->startGroupEdit() == kResultOk, "SDK group begin");
  check(handler.beginEdit(42) == kResultOk &&
            handler.performEdit(42, .75) == kResultOk &&
            handler.endEdit(42) == kResultOk,
        "real handler gesture");
  check(grouped->finishGroupEdit() == kResultOk &&
            grouped->setDirty(true) == kResultOk,
        "group end and dirty");
  auto edits = native.drain();
  std::vector<uint32_t> kinds;
  for (auto &m : edits)
    kinds.push_back(m.kind);
  check(kinds == std::vector<uint32_t>({105, 101, 102, 103, 106, 104}),
        "bounded notification order");
  auto accepted = edits[2].revision;
  check(session.host_value(42, .2, accepted - 1) && session.stale_updates == 1,
        "older audio display update cannot overwrite edit");
  auto fresh = channel.revision();
  native.command(AP11::Set, .4, fresh);
  session.service(true);
  check(c->getParamNormalized(42) == .4 && session.suppressed_echoes == 3 &&
            native.drain().empty(),
        "reverse controller update without gesture echo");
  check(handler.restartComponent(kParamValuesChanged | kParamTitlesChanged) ==
            kResultOk,
        "preset invalidation accepted");
  session.service(true);
  auto refresh = native.drain();
  check(refresh.size() == 3 && refresh[0].kind == AP11::RefreshBegin &&
            refresh[1].kind == AP11::Parameter && refresh[1].id == 42 &&
            refresh[1].value == .4 && refresh[2].kind == AP11::RefreshEnd,
        "SDK metadata and value refresh");
  check(handler.beginEdit(42) == kResultOk, "gesture active on close");
  native.close();
  session.service(true);
  check(!session.is_open() && c->stats.removed == 1 && c->stats.destroyed == 1,
        "remove release before parent destruction");
  auto closed = native.drain();
  check(std::any_of(closed.begin(), closed.end(),
                    [](const auto &m) { return m.kind == AP11::End; }),
        "close finishes accepted gesture");
  native.command(AP11::Open);
  native.close();
  session.service(true);
  check(c->stats.created == 1, "queued open before close cutoff cancelled");
  native.drain();
  native.command(AP11::Open);
  session.service(true);
  check(c->stats.created == 2 && c->getParamNormalized(42) == .4,
        "reopen same controller sound");
  native.drain();
  // Real Win32 close delivery must only enqueue removal. A host callback may
  // not release a vendor view while its native event dispatch is on the stack.
  SendMessageW(session.view().window(), WM_CLOSE, 0, 0);
  SendMessageW(session.view().window(), WM_CLOSE, 0, 0);
  check(session.is_open() && c->stats.removed == 1,
        "WM_CLOSE defers SDK removal until owner service");
  session.service(true);
  check(!session.is_open() && c->stats.removed == 2 && c->stats.destroyed == 2,
        "queued repeated window close removes exactly once");
  native.drain();
  native.command(AP11::Open);
  session.service(true);
  check(c->stats.created == 3 && c->getParamNormalized(42) == .4,
        "window close and reopen retains controller sound");
  native.drain();
  c->stats.refuse = true;
  check(!session.close() && session.is_open() &&
            IsWindow(session.view().window()),
        "refused removal preserves live parent/view");
  c->stats.refuse = false;
  check(session.close(), "positive removal retry");
  native.drain();
  // A consumer stall has a fixed capacity and a visible failure; it does not
  // acquire an audio lock, submit DSP edits, or erase a required close request.
  check(handler.beginEdit(42) == kResultOk, "backlog begin");
  for (unsigned i = 0; i < 511; ++i)
    check(handler.performEdit(42, .3) == kResultOk, "exact backlog capacity");
  check(handler.performEdit(42, .3) != kResultOk &&
            channel.failure() == AP11::Backlog,
        "explicit queue exhaustion");
  native.close();
  session.service(true);
  check(channel.close_requested() > 0, "close remains independent of backlog");
  native.drain();
  check(session.host_value(42, .45, channel.revision()) &&
            c->getParamNormalized(42) == .45,
        "GUI overload does not poison host controller updates");
  c->setComponentHandler(nullptr);
  grouped = nullptr;
  c->terminate();
  c->release();
  // Actual latest-value publication never pairs a value with another revision.
  auto updates = std::make_unique<ControllerUpdates>();
  std::array<uint32_t, 1> ids{42};
  check(updates->configure(ids), "update identity");
  std::atomic<bool> done = false;
  std::thread producer([&] {
    for (uint64_t i = 1; i <= 100000; ++i)
      check(updates->publish(42, double(i % 1024) / 1024., i),
            "audio publication");
    done = true;
  });
  auto apply = [](uint32_t id, double value, uint64_t revision) {
    check(id == 42 && value == double(revision % 1024) / 1024.,
          "consistent value/revision under concurrent publication");
    return true;
  };
  while (!done.load())
    check(updates->drain(apply), "bounded UI drain");
  producer.join();
  updates->drain(apply);
  std::cout << "AP11 SDK view lifecycle, gestures, revision ordering, preset "
               "refresh, backlog and cleanup PASS\n";
}
