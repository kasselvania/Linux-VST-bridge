// Pinned SDK objects driving production handler, editor lifecycle and GUI
// mapping. This is deterministic instrumentation, not commercial evidence.
#include "controller_updates.h"
#include "process_retirement.h"
#include "input_observation.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "editor_session.h"
#include "public.sdk/source/common/pluginview.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "ui_apartment.h"
#include "vendor_handler.h"
#include <filesystem>
#include <iostream>
#include <memory>
#include <stdexcept>
namespace linux_vst_bridge::wf0 {
struct TerminalStatusTestAccess {
 template<class AfterCopy> static bool fail(TerminalStatus& status,AfterCopy after_copy) {
  return status.editor_fatal_after_copy(AP11::Removal,2,after_copy);
 }
};
}
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
  bool retained_test = false, processing = false, forbid_release = false, refuse_frame = false;
  std::vector<int> retirement;
  bool refuse = false, refuse_attach = false, refuse_size = false, lost_parent = false,
       close_during_attach = false, size_after_attach = false, refuse_scale = false;
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
    if (stats.retained_test) { check(!stats.processing, "no release during processing"); stats.retirement.push_back(3); }
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
  uint32 PLUGIN_API release() override { if(stats.forbid_release)ExitProcess(87);return CPluginView::release(); }
  tresult PLUGIN_API isPlatformTypeSupported(FIDString type) override {
    return type && !std::strcmp(type, kPlatformTypeHWND) ? kResultOk
                                                         : kResultFalse;
  }
  tresult PLUGIN_API setFrame(IPlugFrame *frame) override {
    if(!frame&&stats.refuse_frame)return kResultFalse;
    if (!frame && stats.retained_test) { check(!stats.processing, "no frame detach during processing"); stats.retirement.push_back(1); }
    auto r = CPluginView::setFrame(frame);
    if (frame && !stats.refuse_size && !stats.size_after_attach) {
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
    if (stats.refuse_attach) return kResultFalse;
    systemWindow = parent;
    ++stats.attached;
    if (stats.close_during_attach) SendMessageW(HWND(parent),WM_CLOSE,0,0);
    return kResultOk;
  }
  tresult PLUGIN_API removed() override {
    check(!plugFrame, "host frame detached before vendor removal");
    if (stats.retained_test) { check(!stats.processing, "no removal during processing"); stats.retirement.push_back(2); }
    check(std::this_thread::get_id() == owner && (stats.lost_parent || IsWindow(HWND(systemWindow))),
          "remove before destroying parent");
    if (stats.refuse)
      return kResultFalse;
    const auto focusBefore = stats.focus, sizesBefore = stats.sizes;
    SendMessageW(HWND(systemWindow), WM_SETFOCUS, 0, 0);
    SendMessageW(HWND(systemWindow), WM_KILLFOCUS, 0, 0);
    SendMessageW(HWND(systemWindow), WM_SIZE, SIZE_RESTORED,
                 MAKELPARAM(400, 240));
    check(stats.focus == focusBefore && stats.sizes == sizesBefore,
          "removal does not reenter dismantling view through window callbacks");
    systemWindow = nullptr;
    ++stats.removed;
    return kResultOk;
  }
  tresult PLUGIN_API onSize(ViewRect *r) override {
    check(systemWindow, "no platform sizing before attached");
    ++stats.sizes;
    return CPluginView::onSize(r);
  }
  tresult PLUGIN_API getSize(ViewRect *r) override {
    return stats.refuse_size || (stats.size_after_attach && !systemWindow) ? kResultFalse : CPluginView::getSize(r);
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
    if (stats.refuse_scale) return kResultFalse;
    ViewRect r{0, 0, int32(400 * scale), int32(240 * scale)};
    setRect(r);
    return plugFrame ? plugFrame->resizeView(this, &r) : kResultOk;
  }
};
struct Controller final : EditController {
  IComponentHandler *retainedHandler() { componentHandler->addRef(); return componentHandler; }
  Stats stats;
  bool echo = true, no_view = false, invalid_readback=false;
  bool dirty_string_tail=false;
  tresult PLUGIN_API getParameterInfo(int32 index,ParameterInfo& info) override {
    auto result=EditController::getParameterInfo(index,info);
    if(result==kResultOk&&dirty_string_tail) {
      // Legal short strings with unspecified trailing bytes, as observed in
      // Kontakt's first controller parameter. Neither tail may be forwarded.
      std::fill(std::begin(info.title)+5,std::end(info.title),char16(0x1234));
      std::fill(std::begin(info.units)+1,std::end(info.units),char16(0x5678));
    }
    return result;
  }
  ParamValue PLUGIN_API getParamNormalized(ParamID id) override {
    return invalid_readback?-1:EditController::getParamNormalized(id);
  }
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
  static constexpr size_t bytes = 320 + 2 * 512 * sizeof(ap11_gui_message_t);
  std::filesystem::path dir;
  HANDLE file = INVALID_HANDLE_VALUE, map = nullptr;
  uint8_t *data = nullptr;
  std::array<uint8_t, 16> id{};
  explicit Mapping(std::filesystem::path base = {}) {
    dir = (base.empty() ? std::filesystem::temp_directory_path() : base) /
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
    put(4, AP11::GuiProtocolVersion, 4);
    put(8, bytes, 4);
    put(12, sizeof(ap11_gui_message_t), 4);
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
  uint64_t native_view = 1, activation = 0;
  void command(uint32_t kind, double value = 0, uint64_t revision = 0) {
    ap11_gui_message_t m{};
    m.kind = kind;
    if (kind == AP11::Open) { m.native_view = native_view; m.activation = ++activation; }
    m.id = 42;
    m.value = value;
    m.revision = revision;
    command(m);
  }
  void command(ap11_gui_message_t m) {
    if (m.kind == AP11::Open && !m.native_view) { m.native_view = native_view; if (!m.activation) m.activation = ++activation; }
    auto p = word(64).load(), c = word(72).load();
    check(p - c < 512, "command bound");
    std::memcpy(data + 320 + p % 512 * sizeof(m), &m, sizeof(m));
    word(64).store(p + 1, std::memory_order_release);
  }
  std::vector<ap11_gui_message_t> drain() {
    std::vector<ap11_gui_message_t> result;
    auto p = word(80).load(std::memory_order_acquire), c = word(88).load();
    while (c < p) {
      ap11_gui_message_t m{};
      std::memcpy(&m, data + 320 + (512 + c % 512) * sizeof(m), sizeof(m));
      result.push_back(m);
      ++c;
    }
    word(88).store(c, std::memory_order_release);
    return result;
  }
  void close() {
    word(120).fetch_add(1);
    word(256).store(native_view);
    put(264,0,4);
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
// #410: a vendor controller with IMidiMapping, as Kontakt exposes one.
struct MidiController final : EditController, IMidiMapping {
  std::vector<AP11::MidiAssignment> table;
  uint32_t queries = 0;
  bool fails = false;
  bool invalidate_during_query = false;
  uint32_t query_depth = 0, maximum_query_depth = 0;
  int32 program_steps = 127;
  UnitID program_unit = 0;
  tresult PLUGIN_API initialize(FUnknown *host) override {
    auto result = EditController::initialize(host);
    if (result == kResultOk)
      parameters.addParameter(STR16("Program"), nullptr, 127, 0, ParameterInfo::kIsProgramChange,
                              10);
    return result;
  }
  tresult PLUGIN_API getParameterInfo(int32 index, ParameterInfo &info) override {
    auto result = EditController::getParameterInfo(index, info);
    if (result == kResultOk && info.id == 10) {
      info.stepCount = program_steps;
      info.unitId = program_unit;
    }
    return result;
  }
  tresult PLUGIN_API queryInterface(const TUID iid, void **obj) override {
    QUERY_INTERFACE(iid, obj, IMidiMapping::iid, IMidiMapping)
    return EditController::queryInterface(iid, obj);
  }
  REFCOUNT_METHODS(EditController)
  tresult PLUGIN_API getMidiControllerAssignment(int32 bus, int16 channel, CtrlNumber number,
                                                 ParamID &id) override {
    check(std::this_thread::get_id() == owner, "vendor mapping asked on its owner thread");
    ++queries;
    if (fails) throw std::runtime_error("vendor mapping failure");
    ++query_depth;
    maximum_query_depth = std::max(maximum_query_depth, query_depth);
    if (invalidate_during_query) {
      invalidate_during_query = false;
      check(componentHandler && componentHandler->restartComponent(kMidiCCAssignmentChanged) == kResultOk,
            "vendor can invalidate its mapping during a mapping query");
    }
    --query_depth;
    for (const auto &a : table)
      if (a.bus == uint32_t(bus) && a.channel == uint32_t(channel) && a.controller == uint32_t(number)) {
        id = a.id;
        return kResultOk;
      }
    return kResultFalse;
  }
};
// Exactly one complete published table: Begin, Entries..., End.
std::vector<AP11::MidiAssignment> midi_table(const std::vector<ap11_gui_message_t> &all,
                                             uint64_t &serial) {
  std::vector<ap11_gui_message_t> messages;
  std::copy_if(all.begin(), all.end(), std::back_inserter(messages), [](const auto &m) {
    return m.kind >= AP11::MidiMapBegin && m.kind <= AP11::MidiMapEnd;
  });
  check(messages.size() >= 2 && messages.front().kind == AP11::MidiMapBegin &&
            messages.back().kind == AP11::MidiMapEnd,
        "one complete MIDI assignment stream");
  serial = messages.front().revision;
  check(serial && messages.back().revision == serial &&
            messages.back().count == messages.front().count,
        "stream identity and total");
  std::vector<AP11::MidiAssignment> out;
  for (size_t i = 1; i + 1 < messages.size(); ++i) {
    AP11::MidiAssignment records[AP11::MidiAssignmentsPerMessage];
    check(messages[i].revision == serial && AP11::unpackMidi(messages[i], records),
          "well-formed assignment records");
    out.insert(out.end(), records, records + messages[i].count);
  }
  check(out.size() == messages.front().count, "record total");
  return out;
}
std::vector<std::vector<ap11_gui_message_t>> midi_streams(const std::vector<ap11_gui_message_t> &messages) {
  std::vector<std::vector<ap11_gui_message_t>> streams;
  for (const auto &m : messages) {
    if (m.kind < AP11::MidiMapBegin || m.kind > AP11::MidiMapEnd) continue;
    if (m.kind == AP11::MidiMapBegin) streams.emplace_back();
    check(!streams.empty(), "assignments follow a stream start");
    streams.back().push_back(m);
  }
  return streams;
}
void midi_mapping_stream() {
  HostApplication host;
  {
    // A vendor without IMidiMapping publishes an empty table: hosts keep
    // today's behavior for it.
    auto *plain = new Controller;
    plain->echo = false;
    check(plain->initialize(&host) == kResultOk, "plain controller");
    Mapping native;
    GuiChannel channel(native.dir.wstring(), native.id);
    {
      EditorSession session(channel, *plain);
      session.midi_inputs(1);
      uint64_t serial = 0;
      check(midi_table(native.drain(), serial).empty(), "no IMidiMapping, empty table");
    }
    plain->terminate();
    plain->release();
  }
  auto *c = new MidiController;
  check(c->initialize(&host) == kResultOk, "MIDI controller");
  c->table = {{0, 0, 1, 7}, {0, 0, kPitchBend, 8}, {0, 3, kAfterTouch, 9}, {1, 0, 74, 10}};
  {
    Mapping native;
    GuiChannel channel(native.dir.wstring(), native.id);
    EditorSession session(channel, *c);
    session.midi_inputs(2);
    uint64_t first = 0;
    auto table = midi_table(native.drain(), first);
    check(c->queries == 2 * AP11::MidiChannels * AP11::MidiControllers && table.size() == 4 &&
              table[0].controller == 1 && table[0].id == 7 && table[1].controller == kPitchBend &&
              table[2].channel == 3 && table[3].bus == 1 && table[3].id == 10,
          "the vendor's complete table for both event inputs is queued at load");
    c->table[0].id = 9;
    check(session.restart(kMidiCCAssignmentChanged) == kResultOk, "vendor MIDI assignment change accepted");
    session.service(true);
    uint64_t second = 0;
    table = midi_table(native.drain(), second);
    check(second > first && table.size() == 4 && table[0].id == 9, "changed assignments republished");
    session.service(true);
    native.drain(); // finish the metadata refresh queued after the mapping
    // The routing IDs can stay identical while a loaded bank changes the
    // program count/unit. An ordinary refresh may already be in progress.
    session.request_refresh(kParamTitlesChanged);
    c->program_steps = 31;
    c->program_unit = 42;
    check(session.restart(kMidiCCAssignmentChanged) == kResultOk, "program-only MIDI invalidation");
    std::vector<ap11_gui_message_t> messages;
    for (int i = 0; i < 3; ++i) {
      session.service(true);
      auto more = native.drain();
      messages.insert(messages.end(), more.begin(), more.end());
    }
    const auto map_end = std::find_if(messages.begin(), messages.end(), [](const auto &m) {
      return m.kind == AP11::MidiMapEnd && m.flags == kMidiCCAssignmentChanged;
    });
    const auto refreshed_program = std::find_if(map_end, messages.end(), [](const auto &m) {
      return m.kind == AP11::Parameter && m.id == 10 && m.steps == 31 && m.count == 42;
    });
    const auto invalidation = std::find_if(refreshed_program, messages.end(), [](const auto &m) {
      return m.kind == AP11::RefreshEnd && m.flags == kMidiCCAssignmentChanged;
    });
    check(map_end != messages.end() && refreshed_program != messages.end() && invalidation != messages.end(),
          "unchanged routes precede updated program metadata and its original MIDI invalidation");
    native.command(AP11::Refresh);
    session.service(true);
    messages = native.drain();
    check(std::any_of(messages.begin(), messages.end(), [](const auto &m) { return m.kind == AP11::RefreshEnd; }),
          "native bootstrap refresh still refreshes parameters");
    auto streams = midi_streams(messages);
    uint64_t third = 0;
    check(streams.size() == 1 && midi_table(streams[0], third).size() == 4 && third > second,
          "native bootstrap refresh republishes the MIDI table");
    session.midi_restored();
    session.service(true);
    uint64_t restored = 0;
    check(midi_table(native.drain(), restored).size() == 4 && restored > third,
          "a restored project state republishes the MIDI table");
    External external;
    external.session = &session;
    VendorHandler handler;
    handler.external = &external;
    check(c->setComponentHandler(&handler) == kResultOk, "reentrant mapping handler");
    c->invalidate_during_query = true;
    session.midi_restored();
    messages.clear();
    for (int i = 0; i < 3; ++i) {
      session.service(true);
      auto more = native.drain();
      messages.insert(messages.end(), more.begin(), more.end());
    }
    check(c->maximum_query_depth == 1 && midi_streams(messages).size() == 2 && channel.failure() == 0,
          "a reentrant mapping invalidation is coalesced without recursive vendor queries");
    check(c->setComponentHandler(nullptr) == kResultOk, "reentrant handler detached");
    // Every channel and controller assigned: 2096 records over 33 messages,
    // two back-to-back changes publish two complete tables in order.
    c->table.clear();
    for (uint32_t ch = 0; ch < AP11::MidiChannels; ++ch)
      for (uint32_t n = 0; n < AP11::MidiControllers; ++n) c->table.push_back({0, ch, n, 1000 + ch * 1000 + n});
    check(session.restart(kMidiCCAssignmentChanged | kParamValuesChanged) == kResultOk &&
              session.restart(kMidiCCAssignmentChanged) == kResultOk,
          "combined and repeated vendor restarts accepted");
    messages.clear();
    for (int i = 0; i < 4; ++i) {
      session.service(true);
      auto more = native.drain();
      messages.insert(messages.end(), more.begin(), more.end());
    }
    streams = midi_streams(messages);
    check(streams.size() == 2 && streams[0].size() == 35 && streams[1].size() == 35 && channel.failure() == 0,
          "large tables complete in order without queue overrun");
    uint64_t a = 0, b = 0;
    const auto full = midi_table(streams[0], a);
    check(midi_table(streams[1], b).size() == 2096 && b > a && full.size() == 2096 &&
              full.back().channel == 15 && full.back().controller == 130 && full.back().id == 1000 + 15 * 1000 + 130,
          "every channel, aftertouch, pitch bend and program change carried");
    // A vendor exception costs MIDI routing only: an empty table, live session.
    c->fails = true;
    check(session.restart(kMidiCCAssignmentChanged) == kResultOk, "failing vendor mapping restart");
    session.service(true);
    uint64_t failed = 0;
    check(midi_table(native.drain(), failed).empty() && channel.failure() == 0,
          "vendor mapping failure publishes an empty table");
    c->fails = false;
  }
  c->terminate();
  c->release();
  // The host's mapped controller changes reach the vendor processor as ordinary
  // parameter changes at their sample offsets, beside the notes.
  std::array<InputEvent, event_capacity> in{};
  in[0] = {0, 0, 1, 0, 60, .8, 0};
  in[1] = {5, 2, 7, 0, 0, .25, 0};
  in[2] = {20, 2, 8, 0, 0, .5, 0};
  in[3] = {40, 2, 7, 0, 0, .75, 0};
  Changes changes;
  Notes notes;
  changes.load(in.data(), 4, notes);
  int32 offset = -1;
  ParamValue value = -1;
  auto *wheel = changes.getParameterData(0), *bend = changes.getParameterData(1);
  check(notes.getEventCount() == 1 && changes.getParameterCount() == 2 && wheel && bend &&
            wheel->getParameterId() == 7 && wheel->getPointCount() == 2 &&
            wheel->getPoint(0, offset, value) == kResultOk && offset == 5 && value == .25 &&
            wheel->getPoint(1, offset, value) == kResultOk && offset == 40 && value == .75 &&
            bend->getParameterId() == 8 && bend->getPointCount() == 1 &&
            bend->getPoint(0, offset, value) == kResultOk && offset == 20 && value == .5,
        "modulation and pitch bend reach the plug-in as parameter changes at their offsets");
  std::cout << "AP11 Windows MIDI controller mapping PASS\n";
}
unsigned destroyRefusals=0;
HWND lastDestroyed=nullptr;
BOOL WINAPI controlledDestroy(HWND window) {
  if(destroyRefusals) { --destroyRefusals; SetLastError(ERROR_ACCESS_DENIED); return FALSE; }
  lastDestroyed=window;
  return DestroyWindow(window);
}
Stats* retiring_stats = nullptr;
BOOL WINAPI ordered_destroy(HWND w) {
  check(retiring_stats && !retiring_stats->processing, "parent destruction after processing");
  retiring_stats->retirement.push_back(4);
  return DestroyWindow(w);
}
void retained_lifecycle() {
  HostApplication host;
  auto* c = new Controller;
  check(c->initialize(&host) == kResultOk, "retained controller");
  c->stats.retained_test = true;
  {
    Mapping native; GuiChannel channel(native.dir.wstring(),native.id);
    EditorSession session(channel,*c,nullptr,true);
    native.command(AP11::Open); session.service(true); native.drain();
    const auto window = session.view().window();
    const auto epoch = session.view().opens;
    c->stats.processing = true;
    check(session.edit(AP11::Begin,42,0)==kResultOk, "retained gesture begins");
    native.close(); session.service(true); native.drain();
    check(!session.is_open() && IsWindow(window) && !IsWindowVisible(window) &&
          session.view().window()==window && c->stats.created==1 && c->stats.attached==1 &&
          c->stats.retirement.empty() && session.ends==1,
          "active close hides and ends gesture without SDK retirement");
    check(session.host_value(42,.6,channel.revision()), "processing/controller remains usable hidden");
    ++native.native_view; native.command(AP11::Open); session.service(true); native.drain();
    check(session.is_open() && session.view().window()==window && IsWindowVisible(window) &&
          session.view().opens==epoch+1 && c->stats.created==1 && c->stats.attached==1 &&
          c->stats.retirement.empty(), "reopen reuses exact view/parent with fresh logical epoch");
    SendMessageW(window,WM_CLOSE,0,0); session.service(true); native.drain();
    check(!session.is_open() && c->stats.retirement.empty(), "vendor close also retains during processing");
    c->stats.processing = false; // production owner reaches this only after stop/join/deactivate
    retiring_stats=&c->stats;
    const_cast<VendorView&>(session.view()).destruction(ordered_destroy);
    check(session.retire(), "final retained retirement");
    check(c->stats.retirement==std::vector<int>({1,2,3,4}), "frame removed release parent exact ordering");
    check(session.retire() && c->stats.retirement.size()==4 && !IsWindow(window), "retirement exactly once");
  }
  {
    Mapping native; GuiChannel channel(native.dir.wstring(),native.id);
    EditorSession session(channel,*c,nullptr,true);
    native.command(AP11::Open); session.service(true); native.drain();
    native.close(); session.service(true); native.drain();
    c->stats.lost_parent=true; DestroyWindow(session.view().window());
    ++native.native_view; native.command(AP11::Open); session.service(true); native.drain();
    check(!session.is_open() && channel.failure() && c->stats.created==2,
          "destroyed retained parent refuses reopen without replacement");
    check(session.retire(), "lost retained view cleaned only at retirement");
  }
  check(c->stats.created==c->stats.destroyed && c->stats.attached==c->stats.removed,
        "retained references balanced");
  c->terminate(); c->release(); retiring_stats=nullptr;
}

struct RetirementPlugin:AudioEffect {
 std::atomic<unsigned> blocks{0};bool inactive=false,stopped=false;
 tresult PLUGIN_API initialize(FUnknown* h)override{auto r=AudioEffect::initialize(h);addAudioInput(u"Aux",SpeakerArr::kStereo,kAux);addAudioOutput(u"Out",SpeakerArr::kStereo);return r;}
 tresult PLUGIN_API process(ProcessData& d)override{for(int c=0;c<2;++c)for(int i=0;i<d.numSamples;++i)d.outputs[0].channelBuffers32[c][i]=0.f;++blocks;return kResultOk;}
 tresult PLUGIN_API setProcessing(TBool on)override{stopped=!on;return kResultOk;}
 tresult PLUGIN_API setActive(TBool on)override{inactive=!on;return AudioEffect::setActive(on);}
};
struct RetirementExternal:ExternalProcessing {
 RetirementPlugin& p;Mapping& native;EditorSession& editor;Controller& c;std::atomic<unsigned> phase{0};HWND parent;uint32_t epoch;InputObservation input;uint64_t sequence=0;
 RetirementExternal(RetirementPlugin& p,Mapping& n,EditorSession& e,Controller& c):p(p),native(n),editor(e),c(c),parent(e.view().window()),epoch(e.view().opens){}
 bool commercial()const override{return true;}bool stateful()const override{return true;}
 void ready()override{}
 bool next(ExternalBlock& b,float* left,float* right)override{Sleep(1);if(phase.load()==2)return false;b.frames=16;b.silence=3;b.gain_present=false;std::fill_n(left,16,0.f);std::fill_n(right,16,0.f);++sequence;input.observe(1,sequence,(sequence-1)*16,16,3,left,right);return true;}
 void done(const float*,const float*,uint64_t,uint64_t,const ap10_results_t*)override{}
 void service_owner()override{
  if(p.blocks.load()<2)return;
  if(phase.load()==0){native.close();editor.service(true);native.drain();check(!editor.is_open()&&editor.view().window()==parent&&!IsWindowVisible(parent)&&c.stats.retirement.empty(),"processing close hides without release");phase=1;}
  else if(phase.load()==1&&p.blocks.load()>3){++native.native_view;native.command(AP11::Open);editor.service(true);native.drain();check(editor.is_open()&&editor.view().window()==parent&&editor.view().opens==epoch+1&&c.stats.retirement.empty(),"processing reopen reuses view and advances epoch");phase=2;}
 }
};
void retirement_child(const char* path,int mode){
 HostApplication host;Controller* c=new Controller;check(c->initialize(&host)==kResultOk,"retirement controller");c->stats.retained_test=true;
 Mapping native{std::filesystem::path(path)};GuiChannel channel(native.dir.wstring(),native.id);auto* editor=new EditorSession(channel,*c,nullptr,true);
 native.command(AP11::Open);editor->service(true);native.drain();c->stats.processing=true;
 RetirementPlugin plugin;check(plugin.initialize(&host)==kResultOk,"retirement processor");RetirementExternal external(plugin,native,*editor,*c);
 linux_vst_bridge::wf0::EventWriter events(1048576);HostCallbackSink calls(&events,GetCurrentThreadId());
 auto result=run_offline_processing(plugin,plugin,calls,events,&external);
 check(result.success&&result.quiescent&&result.retirement_ready&&plugin.stopped&&plugin.inactive&&external.phase==2,"production processing stopped joined and deactivated");
 c->stats.processing=false;c->stats.forbid_release=true;retiring_stats=&c->stats;
 const_cast<VendorView&>(editor->view()).destruction(mode==5?controlledDestroy:ordered_destroy);if(mode==5)destroyRefusals=1;
 if(mode==3)c->stats.refuse_frame=true;if(mode==4)c->stats.refuse=true;
 RetirementStatus status(std::filesystem::path(path).wstring(),native.id);bool endpoint=false;
 try{complete_process_retirement(mode!=1&&result.retirement_ready,mode!=2,editor,status,1,20,256,1,[&]{
  check(plugin.stopped&&plugin.inactive&&result.quiescent,"input report only after worker joined");
  check(external.input.count==1&&external.input.rows[0].silence==3,"worker retained exact silent input witness");
  auto before=events.sequence();external.input.dump(events);
  check(events.sequence()==before+2,"bounded input summary and row flushed before retirement commit");
  endpoint=true;
 });}
 catch(...){check(mode!=0,"ordinary process retirement succeeds");check(!endpoint,"failed detach cannot close endpoint or commit");ExitProcess(0);}
 check(mode==0&&endpoint&&!IsWindow(external.parent)&&c->stats.retirement==std::vector<int>({1,2,4})&&c->stats.destroyed==0,"exact detach parent destruction without vendor release");
 Sleep(5000);ExitProcess(89); // parent must reclaim the owned child first
}
void process_retirement_regression(){
 wchar_t exe[32768]{};check(GetModuleFileNameW(nullptr,exe,32768)!=0,"fixture path");
 for(int mode=0;mode<6;++mode){
  auto dir=std::filesystem::temp_directory_path()/(L"ap18-retire-"+std::to_wstring(GetCurrentProcessId())+L"-"+std::to_wstring(mode));std::filesystem::create_directory(dir);
  HANDLE f=CreateFileW((dir/L"ap18.retirement").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);check(f!=INVALID_HANDLE_VALUE,"retirement file");
  HANDLE mapping=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,256,nullptr);auto* p=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_ALL_ACCESS,0,0,256));check(p!=nullptr,"retirement mapping");memset(p,0,256);
  memcpy(p,"LVRT",4);linux_vst_bridge::ap1::put(p+4,1,4);linux_vst_bridge::ap1::put(p+8,256,4);linux_vst_bridge::ap1::put(p+12,8,4);p[16]=19;
  std::wstring command=L"\""+std::wstring(exe)+L"\" \""+dir.wstring()+L"\" "+std::to_wstring(mode);STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION child{};
  check(CreateProcessW(exe,command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW,nullptr,nullptr,&si,&child)!=0,"retirement child");
  auto commit=[&]{return InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(p+64),0,0);};
  for(unsigned n=0;n<400&&!commit()&&WaitForSingleObject(child.hProcess,0)==WAIT_TIMEOUT;++n)Sleep(5);
  if(mode==0){check(commit()==1&&linux_vst_bridge::ap1::get(p+192,8)==127,"all retirement milestones externally committed");check(TerminateProcess(child.hProcess,0)!=0,"parent owns exact child containment");}
  else check(commit()==0,"failed prerequisite never commits retirement");
  check(WaitForSingleObject(child.hProcess,3000)==WAIT_OBJECT_0,"child absent");DWORD code=99;GetExitCodeProcess(child.hProcess,&code);check(code==0,"release bomb never called");
  CloseHandle(child.hThread);CloseHandle(child.hProcess);UnmapViewOfFile(p);CloseHandle(mapping);CloseHandle(f);std::filesystem::remove_all(dir);
 }
}
// Real EditorSession fatal destruction in a child. Parent retains mapped
// first-failure custody after process exit; no destructor report is required.
void terminal_child(const char* path,int mode) {
 std::set_terminate([]{ExitProcess(93);});
 HostApplication host;auto* c=new Controller;check(c->initialize(&host)==kResultOk,"IF1 child controller");
 Mapping native{std::filesystem::path(path)};GuiChannel channel(native.dir.wstring(),native.id);
 FaultStatus fault(std::filesystem::path(path).wstring(),native.id);auto* editor=new EditorSession(channel,*c,nullptr,true);editor->fault_status(&fault);
 native.command(AP11::Open);editor->service(true);native.drain();
 check(editor->is_open(),"IF1 child editor attached");
 if(mode==20)c->stats.refuse=true; // persistent removed() refusal
 delete editor; // production destructor commits fatal before std::terminate
 check(mode==21&&c->stats.destroyed==1,"ordinary SDK view release unchanged");
 ExitProcess(0);
}
void terminal_progress_regression() {
 auto dir=std::filesystem::temp_directory_path()/(L"if1-progress-"+std::to_wstring(GetCurrentProcessId()));std::filesystem::create_directory(dir);
 HANDLE f=CreateFileW((dir/L"if1.terminal").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);check(f!=INVALID_HANDLE_VALUE,"progress file");
 HANDLE mapping=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,2048,nullptr);auto* p=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_ALL_ACCESS,0,0,2048));check(p!=nullptr,"progress map");memset(p,0,2048);
 memcpy(p,"LVIF",4);linux_vst_bridge::ap1::put(p+4,1,4);linux_vst_bridge::ap1::put(p+8,2048,4);p[16]=19;
 auto load=[&](size_t at){return uint64_t(InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(p+at),0,0));};
 auto store=[&](size_t at,uint64_t v){InterlockedExchange64(reinterpret_cast<volatile LONG64*>(p+at),LONG64(v));};
 std::array<uint64_t,24> row{1,19,0,7,2,104687,0,9,7,104680,1,2,3,4,0,0,0,11,0,0,0,0,0,0};
 for(size_t i=0;i<row.size();++i)store(1344+i*8,row[i]);store(1024,1);
 HANDLE request=CreateEventW(nullptr,FALSE,FALSE,nullptr),done=CreateEventW(nullptr,FALSE,FALSE,nullptr);check(request&&done,"progress barriers");
 std::thread writer([&]{
  for(unsigned round=0;round<3;++round){
   check(WaitForSingleObject(request,5000)==WAIT_OBJECT_0,"progress requested");
   // Force both slot reuse and commit changes during every bounded read.
   for(unsigned step=1;step<=2;++step){auto n=round*2+step;auto c=load(1024);row[5]=104687+n;row[6]=row[16]=n*256;
    for(size_t i=0;i<row.size();++i)store(1088+((c+1)&1)*256+i*8,row[i]);store(1024,c+1);}
   SetEvent(done);
  }
 });
 {
  std::array<uint8_t,16> id{};id[0]=19;TerminalStatus terminal(dir.wstring(),id);
  check(!TerminalStatusTestAccess::fail(terminal,[&]{SetEvent(request);check(WaitForSingleObject(done,5000)==WAIT_OBJECT_0,"progress committed");}),"unstable fatal context stays pending");
  writer.join();check(load(64)==0,"unstable read publishes no torn failure");
  check(terminal.editor_fatal(999,5),"stable later owner call retries pending failure");
  check(load(64)==2&&load(384+15*8)==AP11::Removal&&load(384+19*8)==2,"first fatal status/domain survive retry");
  check(load(384+5*8)==104693&&load(384+6*8)==1536&&load(384+16*8)==1536,"retry retains one complete progress context");
  check(load(384+7*8)==9&&load(384+9*8)==104680,"confirmed state identity retained");
  check(terminal.editor_fatal(1000)&&load(384+15*8)==AP11::Removal,"later fatal cannot overwrite first");
 }
 CloseHandle(request);CloseHandle(done);UnmapViewOfFile(p);CloseHandle(mapping);CloseHandle(f);std::filesystem::remove_all(dir);
}
void terminal_editor_regression() {
 wchar_t exe[32768]{};check(GetModuleFileNameW(nullptr,exe,32768)!=0,"IF1 executable");
 for(int mode:{20,21}) {
  auto dir=std::filesystem::temp_directory_path()/(L"if1-editor-"+std::to_wstring(GetCurrentProcessId())+L"-"+std::to_wstring(mode));std::filesystem::create_directory(dir);
  HANDLE f=CreateFileW((dir/L"if1.terminal").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);check(f!=INVALID_HANDLE_VALUE,"IF1 file");
  HANDLE mapping=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,2048,nullptr);auto* p=static_cast<uint8_t*>(MapViewOfFile(mapping,FILE_MAP_ALL_ACCESS,0,0,2048));check(p!=nullptr,"IF1 map");memset(p,0,2048);
  memcpy(p,"LVIF",4);linux_vst_bridge::ap1::put(p+4,1,4);linux_vst_bridge::ap1::put(p+8,2048,4);p[16]=19;
  const std::array<uint64_t,24> row{1,19,0,7,2,104687,512,9,7,104680,1,2,3,4,0,0,768,11,0,0,0,0,0,0};
  for(size_t i=0;i<row.size();++i)linux_vst_bridge::ap1::put(p+1344+i*8,row[i],8);linux_vst_bridge::ap1::put(p+1024,1,8);
  std::wstring command=L"\""+std::wstring(exe)+L"\" \""+dir.wstring()+L"\" "+std::to_wstring(mode);STARTUPINFOW startup{};startup.cb=sizeof(startup);PROCESS_INFORMATION child{};
  check(CreateProcessW(exe,command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW,nullptr,nullptr,&startup,&child)!=0,"IF1 child");
  check(WaitForSingleObject(child.hProcess,5000)==WAIT_OBJECT_0,"IF1 child terminal");DWORD code=0;GetExitCodeProcess(child.hProcess,&code);
  auto commit=InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(p+64),0,0);
  if(mode==20){if(code!=93||commit!=2)std::cerr<<"IF1 child exit="<<code<<" commit="<<commit<<"\n";check(code==93&&commit==2,"fatal editor custody survives terminate");check(linux_vst_bridge::ap1::get(p+384+14*8,8)==2&&linux_vst_bridge::ap1::get(p+384+15*8,8)==AP11::Removal,"exact fatal editor class/detail");check(linux_vst_bridge::ap1::get(p+384+5*8,8)==104687,"exact terminal sequence");}
  else check(code==0&&commit==0,"ordinary close never fabricates failure");
  CloseHandle(child.hThread);CloseHandle(child.hProcess);UnmapViewOfFile(p);CloseHandle(mapping);CloseHandle(f);std::filesystem::remove_all(dir);
 }
}
void lifecycle_faults() {
  HostApplication host;
  auto *c=new Controller;
  check(c->initialize(&host)==kResultOk,"lifecycle fixture controller");
  {
    Mapping native;GuiChannel channel(native.dir.wstring(),native.id);EditorSession session(channel,*c);
    for (auto reason:{AP11::Attach,AP11::Size}) {
      c->stats.refuse_attach=reason==AP11::Attach;c->stats.refuse_size=reason==AP11::Size;
      native.command(AP11::Open);session.service(true);
      auto results=native.drain();
      check(!session.is_open() && !channel.failure() && results.size()==1 &&
            results[0].kind==AP11::EditorStatus && results[0].lifecycle==AP11::OpenRefused &&
            results[0].native_view==native.native_view && results[0].result==reason,
            "attachment/size refusal is bound, truthful and editor-only");
      check(session.host_value(42,.6,channel.revision()) && c->getParamNormalized(42)==.6,
            "controller remains usable after editor refusal");
      ++native.native_view;
    }
    c->stats.refuse_attach=c->stats.refuse_size=false;
    c->stats.refuse_scale=true;
    native.command(AP11::Open);session.service(true);native.drain();
    ViewRect native_size{};
    check(session.is_open() && c->last_view->getSize(&native_size)==kResultOk &&
          native_size.getWidth()==400 && native_size.getHeight()==240 &&
          session.view().scale==1.f && session.view().opens==1,
          "declining optional scale preserves a usable native-size editor");
    native.close();session.service(true);native.drain();++native.native_view;
    c->stats.refuse_scale=false;
    c->stats.size_after_attach=true;
    native.command(AP11::Open);session.service(true);native.drain();
    check(session.is_open() && session.view().opens==2,
          "platform size unavailable before attach remains a usable view");
    native.close();session.service(true);native.drain();++native.native_view;
    c->stats.size_after_attach=false;
    // Close while opening cancels only that native lifetime, even when the
    // next generation is already in the same command batch.
    const auto created=c->stats.created;
    native.command(AP11::Open);native.close();++native.native_view;native.command(AP11::Open);
    session.service(true);auto results=native.drain();
    check(session.is_open() && c->stats.created==created+1 && results.size()==1 &&
          results[0].native_view==native.native_view,"pending close does not cancel later native open");
    auto active=results[0];
    for(unsigned cycle=0;cycle<8;++cycle) {
      const auto epoch=session.view().opens;const auto views=c->stats.created;
      native.command(AP11::Open);native.command(AP11::Open);session.service(true);native.drain();
      check(session.view().opens==epoch && c->stats.created==views,"duplicate opens only focus active generation");
      if(cycle%2) native.close();
      else {SendMessageW(session.view().window(),WM_CLOSE,0,0);SendMessageW(session.view().window(),WM_CLOSE,0,0);}
      session.service(true);results=native.drain();
      check(results.size()==1 && !results[0].count && results[0].view_epoch==epoch &&
            results[0].native_view==native.native_view &&
            results[0].lifecycle==(cycle%2?AP11::ClosedByDaw:AP11::ClosedByVendor),
            "one close acknowledgment carries the exact retired epoch and cause");
      session.service(true);check(native.drain().empty(),"duplicate close acknowledgment is absent");
      ++native.native_view;native.command(AP11::Open);session.service(true);results=native.drain();
      check(session.is_open() && session.view().opens==epoch+1 && results.size()==1 &&
            c->getParamNormalized(42)==.6,"reopen advances editor only and preserves controller value");
      auto stale=active;stale.kind=AP11::Focus;stale.focus_result=AP11::FocusConfirmed;
      native.command(stale);session.service(true);
      check(native.drain().empty(),"late focus cannot confirm replacement editor");
      active=results[0];
    }
    c->stats.lost_parent=true;
    DestroyWindow(session.view().window());
    session.service(true);results=native.drain();
    check(!session.is_open() && !channel.failure() && results.size()==1 &&
          results[0].lifecycle==AP11::EditorFailed && results[0].result==AP11::WindowLost,
          "unexpected window loss differs from normal close and whole host failure");
    c->stats.lost_parent=false;
    ++native.native_view;native.command(AP11::Open);session.service(true);native.drain();
    channel.fail(AP11::Backlog);native.close();session.service(true);
    check(!session.is_open() && channel.failure()==AP11::Backlog &&
          channel.close_acknowledged()==channel.close_requested() &&
          session.host_value(42,.7,channel.revision()),"GUI failure retires view and acknowledges close without losing controller");
    const auto views=c->stats.created;
    ++native.native_view;native.command(AP11::Open);session.service(true);
    check(!session.is_open() && c->stats.created==views,"irrecoverable GUI channel refuses reopening");
  }
  // The shared view helper is also used directly by standalone vendor access.
  // It owns no GuiChannel, DAW view token, DSP instance or project snapshot.
  {
    VendorView access;
    c->stats.close_during_attach=true;
    check(access.open(*c) && access.close_requested(),"close during attachment binds the opening epoch");
    check(access.close(),"deferred opening close retires positively");c->stats.close_during_attach=false;
    check(access.open(*c),"standalone owner can open shared mechanical view");
    const auto frameWindow=access.window(); const auto removedBefore=c->stats.removed;
    const auto destroyedBefore=c->stats.destroyed;
    c->stats.refuse_frame=true;
    check(!access.close() && access.window()==frameWindow && IsWindow(frameWindow) &&
          c->stats.removed==removedBefore && c->stats.destroyed==destroyedBefore,
          "refused frame detach cannot remove or release the vendor view");
    c->stats.refuse_frame=false;
    check(access.close() && c->stats.removed==removedBefore+1 &&
          c->stats.destroyed==destroyedBefore+1 && !IsWindow(frameWindow),
          "frame detach retry retires the exact view once");
    check(access.open(*c),"standalone owner can reopen after frame detach retry");
    const auto retainedWindow=access.window(); const auto created=c->stats.created; const auto closesBefore=access.closes;
    access.destruction(controlledDestroy);destroyRefusals=1;
    check(!access.close() && access.window()==retainedWindow && IsWindow(retainedWindow) &&
          GetWindowLongPtrW(retainedWindow,GWLP_USERDATA)!=0 && access.error()==AP11::Removal,
          "failed DestroyWindow retains exact HWND and callback owner");
    check(!access.open(*c) && c->stats.created==created && access.window()==retainedWindow,
          "incomplete parent destruction refuses replacement editor");
    check(access.close() && !access.window() && !access.window_lost() &&
          lastDestroyed==retainedWindow && !IsWindow(retainedWindow) &&
          GetWindowLongPtrW(retainedWindow,GWLP_USERDATA)==0 && access.closes==closesBefore+1 &&
          access.addRef()==2 && access.release()==1,
          "retry destroys exact HWND; intentional NCDESTROY is not WindowLost and clears userdata");
    check(access.open(*c),"standalone owner remains reusable after completed destruction");
    SendMessageW(access.window(),WM_CLOSE,0,0);
    check(access.close_requested() && access.is_open(),"standalone user close remains deferred");
    check(access.close() && !access.is_open(),"standalone positive close receipt");
    check(access.open(*c,AP11::EmbeddedX11),"generic embedded mode opens current vendor view");
    const auto embeddedWindow=access.window();const auto embeddedEpoch=access.opens;
    check(access.x11_window() || (access.presentation()==AP11::Detached &&
          (GetWindowLongPtrW(embeddedWindow,GWL_STYLE)&WS_CAPTION)),
          "missing Wine XID capability restores decorated detached presentation");
    const auto style = GetWindowLongPtrW(embeddedWindow,GWL_STYLE);
    SetWindowLongPtrW(embeddedWindow,GWL_STYLE,style|WS_CLIPCHILDREN);
    check(access.present(AP11::EmbeddedX11) && IsWindowVisible(embeddedWindow),
          "live wrapper enters embedded presentation without losing visibility");
    const auto keysBeforeFallback=c->stats.keys;
    check(access.present(AP11::Detached) && IsWindowVisible(embeddedWindow) &&
          (GetWindowLongPtrW(embeddedWindow,GWL_STYLE)&WS_CLIPCHILDREN) && access.window()==embeddedWindow && access.opens==embeddedEpoch &&
          access.presentation()==AP11::Detached && (GetWindowLongPtrW(embeddedWindow,GWL_STYLE)&WS_CAPTION),
          "fallback preserves shown HWND, clipping, vendor view and epoch while adding decorations");
    SendMessageW(embeddedWindow,WM_CHAR,L'a',0);
    check(c->stats.keys==keysBeforeFallback+1,"live detached fallback still delivers window input to the vendor view");
    ViewRect constraint{0,0,900,700};check(access.host_size(constraint,false) &&
          constraint.getWidth()==400 && constraint.getHeight()==240,"fixed vendor constraints remain native-sized");
    ViewRect invalidSize{0,0,900,700};check(!access.host_size(invalidSize,true),"fixed vendor refuses host resize");
    check(access.close(),"embedded fallback normal close");
    const auto count=c->stats.created;access.opens=UINT32_MAX;
    check(!access.open(*c) && access.error()==AP11::GenerationExhausted &&
          c->stats.created==count,"vendor epoch exhaustion refuses before SDK creation");
  }
  check(c->stats.created==c->stats.destroyed && c->stats.attached==c->stats.removed,
        "all successful and refused views return SDK ownership to baseline");
  c->terminate();c->release();
  Mapping invalid;invalid.put(4,5,4);
  bool refused=false;
  try {GuiChannel bad(invalid.dir.wstring(),invalid.id);} catch(...) {refused=true;}
  check(refused,"retained detached protocol5 cannot be silently paired with new protocol6 host");
}
} // namespace
int main(int argc,char** argv) {
  UiApartment apartment;
  APTTYPE type{};
  APTTYPEQUALIFIER qualifier{};
  check(CoGetApartmentType(&type, &qualifier) == S_OK &&
            (type == APTTYPE_STA || type == APTTYPE_MAINSTA),
        "controller and view own a Windows STA apartment");
  if(argc==3){auto mode=std::stoi(argv[2]);if(mode>=20)terminal_child(argv[1],mode);else retirement_child(argv[1],mode);}
  terminal_progress_regression();
  terminal_editor_regression();
  process_retirement_regression();
  lifecycle_faults();
  retained_lifecycle();
  midi_mapping_stream();
  HostApplication host;
  auto *c = new Controller;
  check(c->initialize(&host) == kResultOk, "controller initialize");
  Mapping native;
  GuiChannel channel(native.dir.wstring(), native.id);
  External external;
  VendorHandler handler;
  handler.external = &external;
  EditorSession session(channel, *c, &handler);
  external.session = &session;
  check(c->setComponentHandler(&handler) == kResultOk,
        "production SDK handler");
  check(c->stats.created == 0, "no editor on scan/restore");
  c->no_view = true;
  native.command(AP11::Open);
  session.service(true);
  auto denied = native.drain();
  check(!session.is_open() && channel.failure() == 0 &&
            std::any_of(denied.begin(), denied.end(),
                        [](const auto &m) {
                          return m.kind == AP11::EditorStatus &&
                                 m.result == AP11::NoView;
                        }),
        "ordinary open failure explicit without poisoning session");
  c->no_view = false;
  ++native.native_view;
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
  check(c->stats.created == 1 && session.view().focuses == 0,
        "repeated open preserves view without claiming desktop activation");
  auto pending = native.drain();
  check(pending.size() == 1 && pending[0].kind == AP11::EditorStatus &&
            pending[0].focus_result == AP11::FocusPending,
        "editor exposes pending desktop handoff");
  auto focus = pending[0];
  focus.kind = AP11::Focus;
  focus.focus_result = AP11::FocusDenied;
  ++focus.view_epoch;
  native.command(focus);
  session.service(true);
  check(native.drain().empty(), "stale view focus result ignored");
  --focus.view_epoch;
  ++focus.activation;
  native.command(focus);
  session.service(true);
  check(native.drain().empty(), "stale activation result ignored");
  --focus.activation;
  native.command(focus);
  session.service(true);
  auto refused = native.drain();
  check(refused.size() == 1 && refused[0].focus_result == AP11::FocusDenied &&
            session.is_open() && channel.failure() == 0 &&
            session.view().focuses == 0 && c->stats.created == 1,
        "refused desktop handoff never counts cached Windows focus or closes "
        "DSP");
  native.command(focus);
  session.service(true);
  check(native.drain().empty(), "duplicate completed focus result ignored");
  SendMessageW(session.view().window(), WM_SETFOCUS, 0, 0);
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
  c->dirty_string_tail=true;
  check(handler.restartComponent(kParamValuesChanged | kParamTitlesChanged) ==
            kResultOk,
        "preset invalidation accepted");
  session.service(true);
  auto refresh = native.drain();
  check(refresh.size() == 3 && refresh[0].kind == AP11::RefreshBegin &&
            refresh[1].kind == AP11::Parameter && refresh[1].id == 42 &&
            refresh[1].value == .4 && refresh[2].kind == AP11::RefreshEnd,
        "SDK metadata and value refresh");
  check(channel.failure()==0 && refresh[1].title[0]=='G' && refresh[1].title[3]=='n' &&
        std::all_of(std::begin(refresh[1].title)+4,std::end(refresh[1].title),[](auto c){return c==0;}) &&
        std::all_of(std::begin(refresh[1].units),std::end(refresh[1].units),[](auto c){return c==0;}),
        "terminated SDK strings ignore and sanitize unused buffer tails");
  c->dirty_string_tail=false;
  c->invalid_readback=true;
  check(handler.restartComponent(kParamValuesChanged)==kResultOk,"genuine invalidation accepted");
  session.service(true);auto unavailable=native.drain();
  check(unavailable.size()==3&&unavailable[1].kind==AP11::Parameter&&unavailable[1].result==1&&unavailable[1].value==0&&unavailable[2].kind==AP11::RefreshEnd,"unavailable SDK getter completes real refresh without a numeric value");
  c->invalid_readback=false;
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
  ++native.native_view;
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
  ++native.native_view;
  native.command(AP11::Open);
  session.service(true);
  check(c->stats.created == 3 && c->getParamNormalized(42) == .4,
        "window close and reopen retains controller sound");
  native.drain();
  const auto replacementOwner=native.native_view;
  const auto replacementEpoch=session.view().opens;
  const auto replacementWindow=session.view().window();
  const auto createdBefore=c->stats.created;
  // The production close fast path is a stable seqlock snapshot. A late A
  // close whose cutoff includes B's queued focus must not retire or cancel B.
  native.command(AP11::Open);
  native.word(120).fetch_add(1);
  native.word(256).store(replacementOwner-1);
  native.put(264,replacementEpoch-1,4);
  native.word(152).store(native.word(64).load());
  native.word(120).fetch_add(1);
  session.service(true);
  check(session.view().window()==replacementWindow && c->stats.created==createdBefore &&
        session.is_open(),"late close from native A cannot affect replacement B");
  native.drain();
  native.word(120).fetch_add(1);
  native.word(256).store(replacementOwner);
  native.put(264,replacementEpoch+1,4);
  native.word(120).fetch_add(1);
  session.service(true);
  check(session.is_open() && native.drain().empty(),"wrong editor epoch close has no result or teardown");
  // A writer interrupted mid-close leaves an odd sequence. Owner never spins
  // and cannot observe a partially replaced token/epoch pair.
  native.word(120).fetch_add(1);
  native.word(256).store(replacementOwner);
  native.put(264,replacementEpoch,4);
  session.service(true);
  check(session.is_open(),"unfinished close publication is deferred without touching editor");
  // Restore an acknowledged even value rather than completing this fixture's
  // synthetic request, retaining the live view for the existing refusal test.
  native.word(120).store(native.word(128).load());
  auto *oldHandler=c->retainedHandler();
  FUnknownPtr<IComponentHandler2> oldGroup(oldHandler);
  check(oldGroup->startGroupEdit()==kResultOk && oldHandler->beginEdit(42)==kResultOk,
        "editor A accepts scoped activity before retirement");
  auto queuedA=native.drain();
  check(queuedA.size()==2 && queuedA[0].native_view==replacementOwner &&
        queuedA[1].view_epoch==replacementEpoch,"editor-originated queued callbacks bind immutable native/epoch");
  check(session.close(),"A retires before B opens");native.drain();
  ++native.native_view;native.command(AP11::Open);session.service(true);native.drain();
  const auto valuesBefore=session.values,gesturesBefore=session.gestures;
  check(oldHandler->performEdit(42,.99)!=kResultOk && oldHandler->endEdit(42)!=kResultOk &&
        oldGroup->finishGroupEdit()!=kResultOk && oldGroup->setDirty(true)!=kResultOk &&
        oldHandler->restartComponent(kParamValuesChanged)!=kResultOk && native.drain().empty() &&
        session.values==valuesBefore && session.gestures==gesturesBefore,
        "retired A handler cannot emit or invalidate replacement B");
  auto *newHandler=c->retainedHandler();
  check(newHandler->beginEdit(42)==kResultOk && newHandler->performEdit(42,.5)==kResultOk &&
        newHandler->endEdit(42)==kResultOk,"B handler remains independently usable");
  newHandler->release();oldGroup=nullptr;oldHandler->release();native.drain();
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
