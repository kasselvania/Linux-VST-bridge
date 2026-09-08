#include "../../vst-state/stream.h"
#include "ap4_backend.h"
#include "processor.h"
#include "public.sdk/source/main/pluginfactory_constexpr.h"
#include <thread>
#include <cstdio>
#ifdef AP3_PREVIEW
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "recovery_view.h"
namespace AP2 {
class Controller final : public Steinberg::Vst::EditController {
public:
  static Steinberg::FUnknown *create(void *) {
    try {
      return static_cast<Steinberg::Vst::IEditController *>(new Controller);
    } catch (...) {
      return nullptr;
    }
  }
  Steinberg::tresult PLUGIN_API
  initialize(Steinberg::FUnknown *context) override {
    auto r = EditController::initialize(context);
    if (r != Steinberg::kResultOk)
      return r;
    owner_ = std::this_thread::get_id();
    // Retained AGain ParamID 0, normalized linear gain, default 1.0. The host
    // delivers changes through ProcessData; this controller never contacts DSP.
    parameters.addParameter(STR16("Gain"), STR16(""), 0, 1.,
                            Steinberg::Vst::ParameterInfo::kCanAutomate, 0);
    parameters.addParameter(STR16("Recover snapshot (loses later edits)"), STR16(""), 1, 0., 0, recoveryID);
    parameters.addParameter(STR16("Confirmed recovery snapshot"), STR16(""), 0, 0.,
                            Steinberg::Vst::ParameterInfo::kIsReadOnly, snapshotID);
    return Steinberg::kResultOk;
  }
  Steinberg::IPlugView *PLUGIN_API createView(Steinberg::FIDString name) override {
    if (!name || std::strcmp(name, Steinberg::Vst::ViewType::kEditor) || owner_ != std::this_thread::get_id()) return nullptr;
    try { return new RecoveryView(this); } catch (...) { return nullptr; }
  }
  Steinberg::tresult PLUGIN_API connect(Steinberg::Vst::IConnectionPoint *peer) override {
    auto r = EditController::connect(peer);
    if (r == Steinberg::kResultOk) command("AP6.status");
    return r;
  }
  Steinberg::tresult PLUGIN_API setParamNormalized(Steinberg::Vst::ParamID id, Steinberg::Vst::ParamValue value) override {
    if (id != recoveryID) return EditController::setParamNormalized(id, value);
    if (owner_ != std::this_thread::get_id()) return Steinberg::kResultFalse;
    EditController::setParamNormalized(id, 0.);
    if (value < .5 || recovering_) return Steinberg::kResultOk;
    recovering_ = true;
    // The explicit user action identifies the current complete snapshot. The
    // processor rejects a revision that changed before the action is delivered.
    command("AP6.status");
    auto r = command("AP6.recover");
    recovering_ = false;
    return r;
  }
  Steinberg::tresult PLUGIN_API getParamStringByValue(Steinberg::Vst::ParamID id,
      Steinberg::Vst::ParamValue value, Steinberg::Vst::String128 text) override {
    if (id != recoveryID && id != snapshotID)
      return EditController::getParamStringByValue(id, value, text);
    const char *source = id == recoveryID ? status_ : snapshot_;
    size_t i = 0;
    for (; i < 127 && source[i]; ++i) text[i] = static_cast<unsigned char>(source[i]);
    text[i] = 0;
    return Steinberg::kResultOk;
  }
  Steinberg::tresult PLUGIN_API notify(Steinberg::Vst::IMessage *message) override {
    using namespace Steinberg;
    if (!message || owner_ != std::this_thread::get_id()) return kResultFalse;
    const char *id = message->getMessageID();
    if (!id) return kResultFalse;
    const void *bytes = nullptr; uint32 size = 0;
    if (!std::strcmp(id, "AP6.snapshot")) {
      if (message->getAttributes()->getBinary("snapshot", bytes, size) == kResultOk && size == sizeof(ap6_snapshot_t)) {
        ap6_snapshot_t info{}; std::memcpy(&info, bytes, size);
        revision_ = info.revision;
        char hash[65]{};
        for (size_t i = 0; i < 32; ++i) std::snprintf(hash + i * 2, 3, "%02x", info.digest[i]);
        std::snprintf(snapshot_, sizeof(snapshot_), "#%llu %s %s%s", (unsigned long long)revision_,
          info.source == 18 ? "component restore" : "component capture", hash,
          info.uncaptured ? "; later changes may be lost" : "");
      }
      if (message->getAttributes()->getBinary("status", bytes, size) == kResultOk && size && size <= 385) {
        auto n = std::min<size_t>(size - 1, sizeof(status_) - 1);
        std::memcpy(status_, bytes, n); status_[n] = 0;
      }
      // Status is bridge metadata. Only a restored control value invalidates
      // the host parameter cache; captures must not dirty a just-saved project.
      return kResultOk;
    }
    if (!std::strcmp(id, "AP6.restored")) {
      if (message->getAttributes()->getBinary("state", bytes, size) != kResultOk ||
          size > LVBState::payloadLimit + LVBState::overhead) return kResultFalse;
      const auto *first = static_cast<const uint8_t *>(bytes);
      LVBState::Stream stream(std::vector<uint8_t>(first, first + size), LVBState::payloadLimit + LVBState::overhead);
      if (setComponentState(&stream) != kResultOk || !componentHandler ||
          componentHandler->restartComponent(Vst::kParamValuesChanged) != kResultOk) return kResultFalse;
      return command("AP6.synced");
    }
    return EditController::notify(message);
  }
  Steinberg::tresult PLUGIN_API
  setComponentState(Steinberg::IBStream *stream) override {
    if (owner_ != std::this_thread::get_id())
      return Steinberg::kResultFalse;
    try {
      std::vector<uint8_t> bytes;
      double gain = 0.;
      if (!LVBState::readEnvelope(stream, bytes) ||
          ap4_validate(bytes.data(), static_cast<uint32_t>(bytes.size()),
                       &gain))
        return Steinberg::kResultFalse;
      return setParamNormalized(0, gain); // no beginEdit/performEdit/endEdit
    } catch (...) {
      return Steinberg::kResultFalse;
    }
  }
  Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream *stream) override {
    uint8_t empty[8] = {'L', 'V', 'B', 'C', 1, 0, 0, 0};
    return owner_ == std::this_thread::get_id() &&
                   LVBState::transfer(stream, empty, 8, true)
               ? Steinberg::kResultOk
               : Steinberg::kResultFalse;
  }
  Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream *stream) override {
    uint8_t bytes[8]{}, expected[8] = {'L', 'V', 'B', 'C', 1, 0, 0, 0};
    return owner_ == std::this_thread::get_id() &&
                   LVBState::transfer(stream, bytes, 8, false) &&
                   std::equal(bytes, bytes + 8, expected)
               ? Steinberg::kResultOk
               : Steinberg::kResultFalse;
  }

private:
  Steinberg::tresult command(const char *id) {
    auto *message = allocateMessage();
    if (!message) return Steinberg::kResultFalse;
    message->setMessageID(id);
    message->getAttributes()->setInt("revision", static_cast<Steinberg::int64>(revision_));
    auto result = sendMessage(message);
    message->release();
    return result;
  }
  uint64_t revision_ = 0;
  bool recovering_ = false;
  char status_[128] = "Set to On to recover; later edits may be lost";
  char snapshot_[128] = "No confirmed complete snapshot";
  std::thread::id owner_;
};
static constexpr Steinberg::TUID controlID =
    INLINE_UID(0xD1444DE3, 0x38814391, 0xA916DC9C, 0xFCC67008);
} // namespace AP2
#endif
namespace AP2 {
// Exact retained Windows AGain processor. The private bundle has a deliberately
// limited label; it is never published alongside a DAW installation.
static constexpr Steinberg::TUID processorID =
    INLINE_UID(0x84E8DE5F, 0x92554F53, 0x96FAE413, 0x3C935A18);
} // namespace AP2
#ifdef AP8_PREVIEW
#include "commercial_controller.h"
#define AP8_UID(...) INLINE_UID(__VA_ARGS__)
namespace AP8 {static constexpr Steinberg::TUID processorID=AP8_UID(AP8_PROCESSOR_UID);static constexpr Steinberg::TUID controlID=AP8_UID(AP8_CONTROLLER_UID);}
BEGIN_FACTORY_DEF("Kasselvania Research", "https://github.com/kasselvania/Linux-VST-bridge", "", 2)
DEF_CLASS(AP8::processorID,Steinberg::PClassInfo::kManyInstances,kVstAudioEffectClass,AP8::class_name,0,AP8::effect?"Fx":"Instrument|Synth","0.11.0",kVstVersionString,AP2::Processor::create,nullptr)
DEF_CLASS(AP8::controlID,Steinberg::PClassInfo::kManyInstances,kVstComponentControllerClass,AP8::class_name,0,"","0.11.0",kVstVersionString,AP8::Controller::create,nullptr)
#elif defined(AP3_PREVIEW)
BEGIN_FACTORY_DEF("Kasselvania Research",
                  "https://github.com/kasselvania/Linux-VST-bridge", "", 2)
DEF_CLASS(AP2::processorID, Steinberg::PClassInfo::kManyInstances, kVstAudioEffectClass, "AGain Queued Preview", 0,
          "Fx", "0.3.0", kVstVersionString, AP2::Processor::create, nullptr)
DEF_CLASS(AP2::controlID, Steinberg::PClassInfo::kManyInstances, kVstComponentControllerClass,
          "Bridge Reference Gain", 0, "", "0.3.0", kVstVersionString,
          AP2::Controller::create, nullptr)
#else
BEGIN_FACTORY_DEF("Kasselvania Research",
                  "https://github.com/kasselvania/Linux-VST-bridge", "", 1)
DEF_CLASS(AP2::processorID, 1, kVstAudioEffectClass, "AGain Offline Bridge", 0,
          "Fx|OnlyOfflineProcess", "0.1.0", kVstVersionString,
          AP2::Processor::create, nullptr)
#endif
END_FACTORY
