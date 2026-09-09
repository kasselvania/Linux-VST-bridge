#include "processor.h"
#include "../../vst-state/stream.h"
#include "ap2_backend.h"
#include "ap3_backend.h"
#include "ap4_backend.h"
#include "input_silence.h"
#ifdef AP8_PREVIEW
#include "ap10_backend.h"
#include "ap11_gui.h"
#include "pluginterfaces/vst/ivstprocesscontext.h"
#endif
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include <algorithm>
#include <cmath>
#include <ctime>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <limits>
#include <sys/stat.h>
#include <unistd.h>
namespace AP2 {
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
struct Guard {
  std::atomic_flag &flag;
  bool held;
  explicit Guard(std::atomic_flag &f) : flag(f), held(!f.test_and_set()) {}
  ~Guard() {
    if (held)
      flag.clear();
  }
};
void report() {
  uint8_t detail[385]{};
  ap2_error(detail, sizeof(detail));
  std::fprintf(stderr, "AP2 backend: %s\n", reinterpret_cast<char *>(detail));
}
// Optional bounded test report, written only on the non-RT owner thread. The
// DAW's own plug-in host may redirect stdout; this preserves the same facts.
void diagnostic_report(const char *path, const char *text, size_t size) {
  std::fwrite(text, 1, size, stdout);
  if (!path || !*path) path = std::getenv("LVB_AP3_REPORT");
  if (!path) return;
  int fd = ::open(path, O_WRONLY | O_CREAT | O_APPEND | O_NOFOLLOW | O_CLOEXEC,
                  0600);
  struct stat st{};
  bool valid = fd >= 0 && !::fstat(fd, &st) && S_ISREG(st.st_mode) &&
               st.st_uid == ::getuid() && (st.st_mode & 077) == 0 &&
               st.st_size >= 0 && st.st_size + static_cast<off_t>(size) <= 65536;
  if (!valid || ::write(fd, text, size) != static_cast<ssize_t>(size))
    std::fputs("AP3 diagnostic persistence failed\n", stderr);
  if (fd >= 0)
    ::close(fd);
}
void report_stats(uint64_t handle, const char *path) {
  ap3_stats_t s{};
  if (!ap3_stats(handle, &s)) {
    char text[512];
    auto n = std::snprintf(
        text, sizeof(text),
        "{\"event\":\"ap3_proxy_stats\",\"fault\":%llu,\"first_position\":%llu,"
        "\"processed\":%llu,\"request_high\":%llu,\"result_high\":%llu,"
        "\"position\":%llu,\"epoch\":%llu}\n",
        (unsigned long long)s.fault, (unsigned long long)s.first_position,
        (unsigned long long)s.processed, (unsigned long long)s.request_high,
        (unsigned long long)s.result_high, (unsigned long long)s.position,
        (unsigned long long)s.epoch);
    if (n > 0 && static_cast<size_t>(n) < sizeof(text))
      diagnostic_report(path, text, static_cast<size_t>(n));
  }
}
void sample_progress(uint64_t handle, const char *path) {
  ap5_observation_t observation{};
  if (ap5_observation(handle, &observation)) return;
  const auto &w = observation.comparison;
  char text[768];
  auto n = std::snprintf(text, sizeof(text),
      "{\"event\":\"ap5_sample_progress\",\"samples\":%llu,"
      "\"before_edit_samples\":%llu,\"edits\":%llu,\"nonzero_samples\":%llu,"
      "\"maximum_error\":%.17g,\"restored_gain\":%.9g,"
      "\"input_fnv1a64\":%llu,\"output_fnv1a64\":%llu}\n",
      (unsigned long long)w.samples, (unsigned long long)w.before_edit_samples,
      (unsigned long long)w.edits, (unsigned long long)w.nonzero_samples,
      w.maximum_error, w.restored_gain, (unsigned long long)observation.input_hash,
      (unsigned long long)observation.output_hash);
  if (n > 0 && static_cast<size_t>(n) < sizeof(text))
    diagnostic_report(path, text, static_cast<size_t>(n));
}
#ifndef AP8_PREVIEW
bool parameters(IParameterChanges *p, double &gain, bool &changed) {
  if (!p)
    return true;
  int32 count = p->getParameterCount();
  if (count < 0 || count > 4)
    return false;
  bool gain_seen = false, bypass_seen = false;
  for (int32 i = 0; i < count; ++i) {
    auto *q = p->getParameterData(i);
    if (!q)
      return false;
    int32 n = q->getPointCount();
    if (n < 0 || n > 1)
      return false;
    if (!n)
      continue;
    auto id = q->getParameterId();
    int32 offset = -1;
    ParamValue value = 0.;
    if (q->getPoint(0, offset, value) != kResultOk || offset != 0 ||
        !std::isfinite(value) || value < 0. || value > 1.)
      return false;
    if (id == recoveryID || id == snapshotID) {
      continue; // non-automatable bridge actions run only on the controller thread
    } else if (id == 0 && !gain_seen) {
      gain = value;
      gain_seen = true;
      changed = true;
    } else if (id == 2 && !bypass_seen && value == 0.)
      bypass_seen = true;
    else
      return false;
  }
  return true;
}
#endif
bool outputs(ProcessData &d, int maximum) {
  return d.symbolicSampleSize == kSample32 && d.numSamples > 0 &&
         d.numSamples <= maximum && d.numOutputs == 1 && d.outputs &&
         d.outputs[0].numChannels == 2 && d.outputs[0].channelBuffers32 &&
         d.outputs[0].channelBuffers32[0] && d.outputs[0].channelBuffers32[1];
}
tresult failure(ProcessData &d, int maximum) {
  if (outputs(d, maximum)) {
    for (int ch = 0; ch < 2; ++ch)
      std::fill_n(d.outputs[0].channelBuffers32[ch], d.numSamples, 0.f);
    d.outputs[0].silenceFlags = 3;
  }
  return kResultFalse;
}
bool overlap(const float *a, const float *b, int n) {
  auto x = reinterpret_cast<uintptr_t>(a), y = reinterpret_cast<uintptr_t>(b);
  auto bytes = static_cast<uintptr_t>(n) * sizeof(float);
  return x < y + bytes && y < x + bytes;
}
} // namespace
bool Processor::stateSession() {
  if (!preview_)
    return false;
  if (handle_)
    return true;
  queued_ = true;
  if (
#ifdef AP8_PREVIEW
      ap9_open(AP8::identity,&handle_)
#else
      ap9_open(nullptr,&handle_)
#endif
  ) {
    phase_ = Failed;
    report();
    return false;
  }
  if (ap5_report_path(handle_, reinterpret_cast<uint8_t *>(report_path_), sizeof(report_path_))) {
    phase_ = Failed;
    return false;
  }
  return true;
}
namespace {
void stateReport(const char *path, const char *operation, const std::vector<uint8_t> &blob,
                 double gain) {
  char hex[65]{};
  for (size_t i = 0; i < 32; ++i)
    std::snprintf(hex + i * 2, 3, "%02x", blob[72 + i]);
  char text[256];
  auto n =
      std::snprintf(text, sizeof(text),
                    "{\"event\":\"ap4_native_state\",\"operation\":\"%s\","
                    "\"payload_bytes\":%zu,\"sha256\":\"%s\",\"gain\":%.9g}\n",
                    operation, blob.size() - LVBState::overhead, hex, gain);
  if (n > 0 && static_cast<size_t>(n) < sizeof(text))
    diagnostic_report(path, text, static_cast<size_t>(n));
}
} // namespace

void Processor::stateFailure(const char *operation, const char *stage) {
  if (state_error_reported_ || owner_ != std::this_thread::get_id())
    return;
  state_error_reported_ = true;
  ap4_failure_t failure{};
  failure.first_position = UINT64_MAX;
  if (handle_)
    ap4_failure(handle_, &failure);
  uint8_t fallback[385]{};
  ap2_error(fallback, sizeof(fallback));
  const auto *raw = std::strcmp(stage, "state_response") == 0 ||
                            !failure.detail[0]
                        ? fallback
                        : failure.detail;
  // Keep the bounded existing backend explanation as JSON text, never locals,
  // command arguments, environment or stream contents.
  char escaped[771]{};
  size_t used = 0;
  for (size_t i = 0; i < 384 && raw[i]; ++i) {
    unsigned char ch = raw[i];
    if (ch == '"' || ch == '\\')
      escaped[used++] = '\\';
    escaped[used++] = ch < 32 || ch > 126 ? '?' : char(ch);
  }
  char text[1400];
  auto n = std::snprintf(text, sizeof(text),
      "{\"event\":\"ap4_native_error\",\"operation\":\"%s\","
      "\"stage\":\"%s\",\"fault\":%llu,\"first_position\":%llu,"
      "\"processed\":%llu,\"callback_rejections\":%llu,\"detail\":\"%s\"}\n",
      operation, stage, (unsigned long long)failure.fault,
      (unsigned long long)failure.first_position,
      (unsigned long long)failure.processed,
      (unsigned long long)callback_rejections_.load(std::memory_order_relaxed),
      escaped);
  if (n > 0 && static_cast<size_t>(n) < sizeof(text))
    diagnostic_report(report_path_, text, static_cast<size_t>(n));
}
tresult PLUGIN_API Processor::getState(IBStream *stream) {
  if (!preview_)
    return kNotImplemented;
  if (phase_ == Failed)
    stateFailure("get", "failed_instance");
  if (!stream || owner_ != std::this_thread::get_id() || phase_ == New ||
      phase_ == Failed || phase_ == Terminated)
    return kResultFalse;
  try {
    if (!stateSession())
      return kResultFalse;
#ifdef AP8_PREVIEW
    // Deliver already accepted UI gestures before the existing DSP/state
    // barrier. Never wait here for work owned by this native UI thread.
    uint64_t generation=0;
    if(!ap11_gui_generation(handle_,&generation)&&guiPoll(generation,512)!=kResultOk)return kResultFalse;
#endif
    std::vector<uint8_t> blob(LVBState::payloadLimit + LVBState::overhead);
    uint32_t size = 0;
    auto state_result=ap4_state(handle_, nullptr, 0, blob.data(),
                  static_cast<uint32_t>(blob.size()), &size);
#ifdef AP8_PREVIEW
    if(state_result==0||state_result==5){
      auto*m=allocateMessage();if(m){m->setMessageID("AP12.persistence");m->getAttributes()->setInt("available",state_result==0?1:0);sendMessage(m);m->release();}
    }
#endif
    if (state_result) {
      if(state_result!=5)phase_ = Failed;
      stateFailure("get", "state_response");
      if(state_result!=5)report();
      return kResultFalse;
    }
    blob.resize(size);
#ifndef AP8_PREVIEW
    double gain = 0.;
#endif
#ifdef AP8_PREVIEW
    if(ap8_validate(AP8::identity,blob.data(),size)||!LVBState::transfer(stream,blob.data(),blob.size(),true))return kResultFalse;
    state_readback_=blob;
    stateReport(report_path_,"opaque_get",blob,0.);
#else
    if (ap4_validate(blob.data(), size, &gain) ||
        !LVBState::transfer(stream, blob.data(), blob.size(), true))
      return kResultFalse;
    stateReport(report_path_, "get", blob, gain);
    snapshotStatus("Captured complete state; not necessarily a project save");
    sample_progress(handle_, report_path_);
#endif
    return kResultOk;
  } catch (...) {
    return kResultFalse;
  }
}
tresult PLUGIN_API Processor::setState(IBStream *stream) {
  if (!preview_)
    return kNotImplemented;
  if (phase_ == Failed)
    stateFailure("set", "failed_instance");
  if (!stream || owner_ != std::this_thread::get_id() || phase_ == New ||
      phase_ == Running || phase_ == Failed || phase_ == Terminated)
    return kResultFalse;
  try {
    std::vector<uint8_t> blob;
#ifndef AP8_PREVIEW
    double restored = 0.;
#endif
    if (!LVBState::readEnvelope(stream, blob) ||
#ifdef AP8_PREVIEW
        ap8_validate(AP8::identity,blob.data(),static_cast<uint32_t>(blob.size())))
#else
        ap4_validate(blob.data(), static_cast<uint32_t>(blob.size()), &restored))
#endif
      {phase_=Failed;return kResultFalse;}
    if (!stateSession())
      return kResultFalse;
    std::vector<uint8_t> readback(LVBState::payloadLimit + LVBState::overhead);
    uint32_t size = 0;
    if (ap4_state(handle_, blob.data(), static_cast<uint32_t>(blob.size()),
                  readback.data(), static_cast<uint32_t>(readback.size()),
                  &size)) {
      phase_ = Failed;
      stateFailure("set", "state_response");
      report();
      return kResultFalse;
    }
    readback.resize(size);
#ifdef AP8_PREVIEW
    if(ap8_validate(AP8::identity,readback.data(),size)){phase_=Failed;return kResultFalse;}
    state_readback_=std::move(readback);this->readback();
    stateReport(report_path_,"opaque_set",state_readback_,0.);
#else
    if (readback != blob) {
      phase_ = Failed;
      return kResultFalse;
    }
    gain_ = restored;
    stateReport(report_path_, "set", readback, restored);
    snapshotStatus("Confirmed project/component restore");
#endif
    return kResultOk;
  } catch (...) {
    phase_ = Failed;
    return kResultFalse;
  }
}
void Processor::snapshotStatus(const char *status) {
  ap6_snapshot_t snapshot{};
  const bool confirmed = handle_ && !ap6_snapshot(handle_, &snapshot);
  char hash[65]{}, escaped[771]{};
  if (confirmed) for (size_t i = 0; i < 32; ++i)
    std::snprintf(hash + i * 2, 3, "%02x", snapshot.digest[i]);
  size_t used = 0;
  for (size_t i = 0; i < 384 && status[i]; ++i) {
    unsigned char ch = status[i];
    if (ch == '"' || ch == '\\') escaped[used++] = '\\';
    escaped[used++] = ch < 32 || ch > 126 ? '?' : char(ch);
  }
  char text[1200];
  auto n = std::snprintf(text, sizeof(text),
      "{\"event\":\"ap6_status\",\"snapshot_revision\":%llu,\"source\":%u,\"generation\":%llu,"
      "\"uncaptured_changes_possible\":%u,\"envelope_sha256\":\"%s\",\"detail\":\"%s\"}\n",
      (unsigned long long)snapshot.revision, snapshot.source, (unsigned long long)snapshot.generation,
      snapshot.uncaptured, hash, escaped);
  if (n > 0 && static_cast<size_t>(n) < sizeof(text)) diagnostic_report(report_path_, text, static_cast<size_t>(n));
  auto *message = allocateMessage();
  if (!message) return;
  message->setMessageID("AP6.snapshot");
  if (confirmed)
    message->getAttributes()->setBinary("snapshot", &snapshot, sizeof(snapshot));
  message->getAttributes()->setBinary("status", status, static_cast<uint32>(std::strlen(status) + 1));
  sendMessage(message);
  message->release();
}
tresult PLUGIN_API Processor::notify(IMessage *message) {
  if (!preview_ || !message || owner_ != std::this_thread::get_id())
    return kResultFalse;
  const auto *id = message->getMessageID();
  if (!id) return kResultFalse;
#ifdef AP8_PREVIEW
  if(!std::strcmp(id,"AP10.capabilities")){int64 enabled=0;if(message->getAttributes()->getInt("notifications",enabled)!=kResultOk)return kResultFalse;notifications_=enabled==1;return kResultOk;}
  if(!std::strcmp(id,"AP10.poll")){
    uint32_t notice[3]{};if(!handle_||ap10_notices(handle_,notice)||!notice[0])return kResultOk;
    if(notice[1]>UINT32_MAX-(latency_-vendor_latency_))return kResultFalse;
    latency_=latency_-vendor_latency_+notice[1];vendor_latency_=notice[1];tail_=notice[2];
    auto*m=allocateMessage();if(!m)return kResultFalse;m->setMessageID("AP10.restart");m->getAttributes()->setInt("flags",notice[0]);auto r=sendMessage(m);m->release();return r;
  }
  if(!std::strcmp(id,"AP11.bind")){
    if(!stateSession())return kResultFalse;
    uint64_t generation=0;if(ap11_gui_generation(handle_,&generation))return kResultFalse;
    auto*m=allocateMessage();if(!m)return kResultFalse;m->setMessageID("AP11.identity");m->getAttributes()->setInt("generation",int64(generation));auto r=sendMessage(m);m->release();return r;
  }
  if(!std::strncmp(id,"AP11.",5)){
    int64 generation=0;if(message->getAttributes()->getInt("generation",generation)!=kResultOk||generation<=0||!handle_)return kResultFalse;
    if(!std::strcmp(id,"AP11.capabilities")){int64 caps=0;if(message->getAttributes()->getInt("caps",caps)!=kResultOk||caps<0||caps>7)return kResultFalse;if(ap11_gui_capabilities(handle_,uint64_t(generation),uint32_t(caps)))return kResultFalse;gui_consumer_=(caps&1)!=0;return kResultOk;}
    if(!std::strcmp(id,"AP11.failure")){int64 code=0;if(message->getAttributes()->getInt("code",code)!=kResultOk||code<1||code>11)return kResultFalse;ap11_gui_failure(handle_,uint64_t(generation),uint32_t(code));return kResultOk;}
    if(!std::strcmp(id,"AP11.poll"))return guiPoll(uint64_t(generation),64);
    if(!std::strcmp(id,"AP11.command")){
      const void* bytes=nullptr;uint32 size=0;
      if(message->getAttributes()->getBinary("command",bytes,size)!=kResultOk||size!=sizeof(ap11_gui_message_t))return kResultFalse;
      ap11_gui_message_t command{};std::memcpy(&command,bytes,sizeof(command));
      // Hosts may disconnect this side before the controller retires. A close
      // still reaches the owned view, but no reply can reach the departed UI.
      // Other commands require the live return route before they are applied.
      if(!getPeer() && command.kind!=AP11::Close)return kResultFalse;
      command.result=ap11_gui_command(handle_,uint64_t(generation),&command);
      if(!getPeer())return command.result==0?kResultOk:kResultFalse;
      auto*m=allocateMessage();if(!m)return kResultFalse;m->setMessageID("AP11.result");m->getAttributes()->setInt("generation",generation);m->getAttributes()->setBinary("command",&command,sizeof(command));auto r=sendMessage(m);m->release();return r;
    }
    return kResultFalse;
  }
  if(!std::strcmp(id,"AP8.readback"))return readback();
#endif
  if (!std::strcmp(id, "AP6.synced")) {
    controller_synced_ = true;
    return kResultOk;
  }
  if (!std::strcmp(id, "AP6.status")) {
    snapshotStatus("Last confirmed snapshot; later edits may be lost");
    return kResultOk;
  }
  if (!std::strcmp(id, "AP6.recover")) {
    int64 revision = 0;
    if (message->getAttributes()->getInt("revision", revision) != kResultOk || revision < 1)
      return kResultFalse;
    return recover(static_cast<uint64_t>(revision));
  }
  return AudioEffect::notify(message);
}
tresult Processor::recover(uint64_t revision) {
  Guard guard(busy_);
  if (!guard.held || !handle_) return kResultFalse;
  ap4_failure_t fault{};
  if (ap4_failure(handle_, &fault) || !fault.fault) {
    snapshotStatus("Recovery refused: this endpoint has not failed");
    return kResultFalse;
  }
  phase_ = Failed; // callbacks remain silent until state AND controller agree
  try {
    std::vector<uint8_t> state(LVBState::payloadLimit + LVBState::overhead);
    uint32_t written = 0;
    if (ap6_recover(handle_, revision, state.data(), static_cast<uint32_t>(state.size()), &written)) {
      uint8_t detail[385]{}; ap2_error(detail, sizeof(detail));
      snapshotStatus(reinterpret_cast<const char *>(detail));
      return kResultFalse;
    }
    state.resize(written);
    if (ap5_report_path(handle_, reinterpret_cast<uint8_t *>(report_path_), sizeof(report_path_)))
      return kResultFalse;
    double restored = 0.;
    if (ap4_validate(state.data(), written, &restored)) return kResultFalse;
    controller_synced_ = false;
    auto *message = allocateMessage();
    if (!message) return kResultFalse;
    message->setMessageID("AP6.restored");
    message->getAttributes()->setBinary("state", state.data(), written);
    auto result = sendMessage(message);
    message->release();
    if (result != kResultOk || !controller_synced_) {
      snapshotStatus("Restore halted: controller/host synchronization not confirmed");
      return kResultFalse;
    }
    gain_ = restored; // existing reference validator; recovery transports opaque bytes
    phase_ = Deactivated;
    if (want_active_) {
      if (ap4_activate(handle_, static_cast<uint32_t>(std::min(maximum_,256)), static_cast<uint32_t>(process_mode_))) {
        phase_ = Failed; snapshotStatus("Recovered state but activation failed"); return kResultFalse;
      }
      phase_ = Active;
      if (want_processing_) {
        if (ap3_transition(handle_, 10)) { phase_ = Failed; return kResultFalse; }
        phase_ = Running;
      }
    }
    state_error_reported_ = false;
    stateReport(report_path_, "recover", state, restored);
    snapshotStatus("Recovered identified snapshot; later edits were not restored");
    return kResultOk;
  } catch (...) {
    phase_ = Failed;
    snapshotStatus("Recovery failed; no default state substituted");
    return kResultFalse;
  }
}
Processor::~Processor() {
  if (handle_)
    queued_ ? (void)ap3_close(handle_) : (void)ap2_close(handle_);
}
tresult PLUGIN_API Processor::initialize(FUnknown *context) {
#ifdef AP8_PREVIEW
  if(ap10_results_abi_version()!=1)return kResultFalse;
#endif
  Guard g(busy_);
  if (!g.held || phase_ != New || ap2_abi_version() != 1)
    return kResultFalse;
  auto result = AudioEffect::initialize(context);
  if (result != kResultOk)
    return result;
#ifdef AP8_PREVIEW
  size_t ordinal=0;
  for(const auto& b:AP8::buses){
    bool enabled=b.type==kMain&&(b.flags&BusInfo::kDefaultActive);
    bus_active_[ordinal++]=enabled;
    auto flags=b.flags;
    const auto* name=reinterpret_cast<const TChar*>(b.name);
    if(b.media==kAudio){if(b.direction==kInput)addAudioInput(name,b.arrangement,b.type,flags);else addAudioOutput(name,b.arrangement,b.type,flags);}
    else {if(b.direction==kInput)addEventInput(name,b.channels,b.type,flags);else addEventOutput(name,b.channels,b.type,flags);}
  }
#else
  addAudioInput(STR16("Offline Stereo In"), SpeakerArr::kStereo);
#endif
#ifndef AP8_PREVIEW
  addAudioOutput(STR16("Offline Stereo Out"), SpeakerArr::kStereo);
#endif
  owner_ = std::this_thread::get_id();
  phase_ = Initialized;
  return kResultOk;
}
#ifdef AP8_PREVIEW
bool Processor::setupBuses(uint32_t maximum,uint32_t mode,double rate,uint32_t* traits){
 std::vector<uint8_t> bytes;auto put=[&](uint64_t v,int n){for(int i=0;i<n;++i)bytes.push_back(uint8_t(v>>(8*i)));};
 put(std::size(AP8::buses),4);size_t i=0;
 for(const auto& b:AP8::buses){for(auto v:{b.media,b.direction,b.index,b.channels,b.type,uint32_t(bus_active_[i++])})put(v,4);put(b.arrangement,8);}
 return stateSession()&&!ap10_setup(handle_,maximum,mode,rate,bytes.data(),uint32_t(bytes.size()),notifications_?1:0,traits);
}
#endif
tresult PLUGIN_API Processor::activateBus(MediaType media,BusDirection direction,int32 index,TBool active){
 Guard g(busy_);if(!g.held||owner_!=std::this_thread::get_id()||(phase_!=Initialized&&phase_!=Setup&&phase_!=Deactivated))return kResultFalse;
#ifdef AP8_PREVIEW
 size_t ordinal=0;for(const auto&b:AP8::buses){
  if(int(b.media)==media&&int(b.direction)==direction&&int(b.index)==index){
   if(active&&b.type!=kMain){
    // This owner-thread, inactive SDK operation is outside process().
    char text[256];auto n=std::snprintf(text,sizeof(text),
      "{\"event\":\"ap10_unsupported_bus_activation\",\"media\":%d,\"direction\":%d,\"index\":%d,\"active\":true}\n",media,direction,index);
    if(n>0&&size_t(n)<sizeof(text))diagnostic_report(report_path_,text,size_t(n));
    return kResultFalse;
   }
   auto r=AudioEffect::activateBus(media,direction,index,active);if(r==kResultOk)bus_active_[ordinal]=active!=0;return r;
  }++ordinal;
 }return kResultFalse;
#else
 if(media!=kAudio||index!=0||(direction!=kInput&&direction!=kOutput))return kResultFalse;
 auto r=AudioEffect::activateBus(media,direction,index,active);if(r==kResultOk)(direction==kInput?input_active_:output_active_)=active!=0;return r;
#endif
}
tresult PLUGIN_API Processor::setBusArrangements(SpeakerArrangement* in,int32 ni,SpeakerArrangement* out,int32 no){
 Guard g(busy_);if(!g.held||owner_!=std::this_thread::get_id()||(phase_!=Initialized&&phase_!=Setup&&phase_!=Deactivated))return kResultFalse;
#ifdef AP8_PREVIEW
 int inputs=0,outputs=0;for(const auto&b:AP8::buses)if(b.media==kAudio){if(b.direction==kInput){if(!in||inputs>=ni||in[inputs++]!=b.arrangement)return kResultFalse;}else{if(!out||outputs>=no||out[outputs++]!=b.arrangement)return kResultFalse;}}
 if(inputs!=ni||outputs!=no)return kResultFalse;
#else
 if(ni!=1||no!=1||!in||!out||*in!=SpeakerArr::kStereo||*out!=SpeakerArr::kStereo)return kResultFalse;
#endif
 return AudioEffect::setBusArrangements(in,ni,out,no);
}
tresult PLUGIN_API Processor::canProcessSampleSize(int32 size) {
  return size == kSample32 ? kResultTrue : kResultFalse;
}
tresult PLUGIN_API Processor::setupProcessing(ProcessSetup &setup) {
  Guard g(busy_);
  requested_maximum_ = setup.maxSamplesPerBlock;
  requested_rate_ = setup.sampleRate;
  requested_mode_ = setup.processMode;
  if (!g.held || owner_ != std::this_thread::get_id() ||
      (phase_ != Initialized && phase_ != Setup && phase_ != Deactivated) ||
      (setup.processMode != kOffline &&
       !(preview_ && setup.processMode == kRealtime)) ||
      setup.symbolicSampleSize != kSample32 ||
      (!preview_ && setup.sampleRate != 48000.) || !std::isfinite(setup.sampleRate) ||
      setup.maxSamplesPerBlock < 1 || setup.maxSamplesPerBlock > (preview_ ? 1024 : 256))
    return kResultFalse;
  if(preview_){
    uint32_t traits[3]{};
    #ifdef AP8_PREVIEW
    if(!setupBuses(uint32_t(setup.maxSamplesPerBlock),uint32_t(setup.processMode),setup.sampleRate,traits))return kResultFalse;
#else
    if(!stateSession()||ap9_setup(handle_,static_cast<uint32_t>(setup.maxSamplesPerBlock),static_cast<uint32_t>(setup.processMode),setup.sampleRate,traits))return kResultFalse;
#endif
    latency_=traits[0];tail_=traits[1];
#ifdef AP8_PREVIEW
    vendor_latency_=traits[2];
#endif
  }
  auto r = AudioEffect::setupProcessing(setup);
  if (r == kResultOk) {
    maximum_ = setup.maxSamplesPerBlock;
    process_mode_ = setup.processMode;
    queued_ = preview_ || process_mode_ == kRealtime;
    phase_ = Setup;
  }
  return r;
}
tresult PLUGIN_API Processor::setActive(TBool active) {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id())
    return kResultFalse;
  if (!active && phase_ == Failed) {
    want_active_ = false;
    return kResultOk;
  }
  if (active) {
    if(phase_!=Setup&&phase_!=Deactivated)return kResultFalse;
#ifdef AP8_PREVIEW
    size_t i=0;for(const auto& b:AP8::buses){if(b.media==kAudio&&b.type==kMain&&!bus_active_[i])return kResultFalse;++i;}
    uint32_t traits[3]{};if(!setupBuses(uint32_t(maximum_),uint32_t(process_mode_),requested_rate_,traits))return kResultFalse;
    latency_=traits[0];tail_=traits[1];
#ifdef AP8_PREVIEW
    vendor_latency_=traits[2];
#endif
#endif
    if ((phase_ != Setup && phase_ != Deactivated) || !input_active_ ||
        !output_active_)
      return kResultFalse;
    if (preview_
            ? (!stateSession() ||
               ap4_activate(handle_, static_cast<uint32_t>(std::min(maximum_,256)),
                            static_cast<uint32_t>(process_mode_)))
            : (queued_ ? ap3_open(static_cast<uint32_t>(maximum_), &handle_)
                       : ap2_open(static_cast<uint32_t>(maximum_), &handle_))) {
      report();
      phase_ = Failed;
      return kResultFalse;
    }
    phase_ = Active;
    want_active_ = true;
    return kResultOk;
  }
  if (phase_ != Stopped && !(preview_ && phase_ == Active))
    return kResultFalse;
  if (preview_ ? ap4_deactivate(handle_)
               : (queued_ ? ap3_transition(handle_, 14)
                          : ap2_transition(handle_, 14))) {
    report();
    phase_ = Failed;
    return kResultFalse;
  }
  phase_ = Deactivated;
  want_active_ = false;
  return kResultOk;
}
tresult PLUGIN_API Processor::setProcessing(TBool running) {
  Guard g(busy_);
  if (!g.held)
    return kResultFalse;
#ifdef AP8_PREVIEW
  if(!running)returned_.release_requested=true;
#endif
  if (!running && phase_ == Failed) {
    want_processing_ = false;
    return kResultOk;
  }
  if (running ? (phase_ != Active && !(queued_ && phase_ == Stopped))
              : phase_ != Running)
    return kResultFalse;
  if (queued_ ? ap3_transition(handle_, running ? 10 : 12)
              : ap2_transition(handle_, running ? 10 : 12)) {
    if (!queued_)
      report();
    phase_ = Failed;
    return kResultFalse;
  }
  phase_ = running ? Running : Stopped;
  want_processing_ = running != 0;
  return kResultOk;
}
// A discontinuity is one contiguous run of rejected non-empty callbacks.
// Successful silence and latency priming are counted separately at return.
tresult Processor::rejected(ProcessData &d) {
  callback_rejections_.fetch_add(1, std::memory_order_relaxed);
  if (outputs(d, maximum_)) {
    rejected_frames_.fetch_add(static_cast<uint64_t>(d.numSamples), std::memory_order_relaxed);
    if (!last_callback_rejected_.exchange(true, std::memory_order_relaxed))
      discontinuities_.fetch_add(1, std::memory_order_relaxed);
  }
  return failure(d, maximum_);
}
tresult PLUGIN_API Processor::process(ProcessData &d) {
#ifdef AP8_PREVIEW
  timespec entered{};clock_gettime(CLOCK_MONOTONIC,&entered);
  const uint64_t entered_ns=uint64_t(entered.tv_sec)*1000000000+uint64_t(entered.tv_nsec);
#endif
  Guard g(busy_);
  auto reject = [&] { return rejected(d); };
  if (!g.held) return reject();
#ifdef AP8_PREVIEW
  returned_.beginCallback();
  if(d.numSamples>=0&&returned_.release_requested){
    auto rejected=returned_.rejected;returned_.release(d,[&](int bus){return eventOutputActive(bus);});
    if(returned_.rejected!=rejected){ap10_fail_results(handle_);phase_=Failed;return reject();}
  }
#endif
  if (phase_ != Running || d.processMode != process_mode_ ||
      d.symbolicSampleSize != kSample32 || d.numSamples < 0 ||
      d.numSamples > maximum_)
    return reject();
  double pending = gain_;
#ifdef AP8_PREVIEW
  ap8_event_t events[256]{};uint32_t event_count=0;
  auto append=[&](ap8_event_t e){if(event_count==256)return false;events[event_count++]=e;return true;};
  if(d.inputEvents){auto n=d.inputEvents->getEventCount();if(n<0||n>256)return reject();for(int i=0;i<n;++i){Event e{};if(d.inputEvents->getEvent(i,e)!=kResultOk||e.busIndex!=0)return reject();
    ap8_event_t v{};v.offset=static_cast<uint32_t>(e.sampleOffset);
    if(e.type==Event::kNoteOnEvent){v.kind=0;v.id=static_cast<uint32_t>(e.noteOn.noteId);v.channel=e.noteOn.channel;v.pitch=e.noteOn.pitch;v.value=e.noteOn.velocity;v.tuning=e.noteOn.tuning;}
    else if(e.type==Event::kNoteOffEvent){v.kind=1;v.id=static_cast<uint32_t>(e.noteOff.noteId);v.channel=e.noteOff.channel;v.pitch=e.noteOff.pitch;v.value=e.noteOff.velocity;v.tuning=e.noteOff.tuning;}else return reject();
    if(!append(v))return reject();}}
  if(d.inputParameterChanges){auto n=d.inputParameterChanges->getParameterCount();if(n<0||n>256)return reject();for(int i=0;i<n;++i){auto*q=d.inputParameterChanges->getParameterData(i);if(!q)return reject();auto id=q->getParameterId();bool known=false;for(const auto&p:AP8::parameters)if(p.id==id){known=true;break;}if(!known)return reject();auto points=q->getPointCount();if(points<0||points>256)return reject();int previous=-1;for(int j=0;j<points;++j){int offset=0;double value=0;if(q->getPoint(j,offset,value)!=kResultOk||offset<previous)return reject();previous=offset;if(!append({static_cast<uint32_t>(offset),2,id,0,0,value,0,0}))return reject();}}}
  if(d.numSamples==0){
    if(d.numInputs||d.numOutputs)return reject();
    float dummy=0;uint64_t flags=0;ap7_delivery_t delivery{};ap10_context_t context{};
    if(ap13_process(handle_,0,events,event_count,&context,0,&dummy,&dummy,&dummy,&dummy,&flags,&delivery,entered_ns)||!deliverResults(d)){phase_=Failed;returned_.release_requested=true;returned_.release(d,[&](int bus){return eventOutputActive(bus);});return reject();}return kResultOk;
  }
#else
  bool changed = false;
  if (!parameters(d.inputParameterChanges, pending, changed) ||
      (d.inputEvents && d.inputEvents->getEventCount() != 0))
    return reject();
  if (d.numSamples == 0) {
    if (d.numInputs != 0 || d.numOutputs != 0 || d.inputs || d.outputs)
      return reject();
    if (preview_ && changed) {
      float dummy = 0.f;
      uint64_t flags = 0;
      if (ap3_process(handle_, 0, pending, 0, &dummy, &dummy, &dummy, &dummy,
                      &flags)) {
        phase_ = Failed;
        return reject();
      }
    }
    gain_ = pending;
    return kResultOk;
  }
  #endif
  if ((!queued_ && blocks_ >= 64) || !outputs(d, maximum_) ||
#ifdef AP8_PREVIEW
      d.numInputs != int(std::count_if(std::begin(AP8::buses),std::end(AP8::buses),[](const auto& b){return b.media==kAudio&&b.direction==kInput;})))return reject();
  float silent_input[1024]{};float* in[2]={silent_input,silent_input};uint64_t input_flags=3;
  if(AP8::effect){
    if(!d.inputs||d.inputs[0].numChannels!=2||!d.inputs[0].channelBuffers32||!d.inputs[0].channelBuffers32[0]||!d.inputs[0].channelBuffers32[1]||(d.inputs[0].silenceFlags&~uint64_t(3)))return reject();
    in[0]=d.inputs[0].channelBuffers32[0];in[1]=d.inputs[0].channelBuffers32[1];input_flags=d.inputs[0].silenceFlags;
    for(int i=1;i<d.numInputs;++i)if(d.inputs[i].numChannels!=2)return reject();
  }
#else
      d.numInputs != 1 || !d.inputs || d.inputs[0].numChannels != 2 ||
      !d.inputs[0].channelBuffers32 || !d.inputs[0].channelBuffers32[0] ||
      !d.inputs[0].channelBuffers32[1] ||
      (d.inputs[0].silenceFlags & ~uint64(3)))
    return reject();
  auto **in = d.inputs[0].channelBuffers32;
  auto input_flags = d.inputs[0].silenceFlags;
  #endif
  auto **out = d.outputs[0].channelBuffers32;
#ifdef AP8_PREVIEW
  if(overlap(out[0],out[1],d.numSamples))return reject();
  if(AP8::effect&&(overlap(in[0],in[1],d.numSamples)||overlap(out[0],in[1],d.numSamples)||overlap(out[1],in[0],d.numSamples)||(in[0]!=out[0]&&overlap(in[0],out[0],d.numSamples))||(in[1]!=out[1]&&overlap(in[1],out[1],d.numSamples))))return reject();
#else
  if (overlap(out[0], out[1], d.numSamples) ||
      overlap(in[0], in[1], d.numSamples) ||
      overlap(out[0], in[1], d.numSamples) ||
      overlap(out[1], in[0], d.numSamples) ||
      (in[0] != out[0] && overlap(in[0], out[0], d.numSamples)) ||
      (in[1] != out[1] && overlap(in[1], out[1], d.numSamples)))
    return reject();
  #endif
#ifdef AP8_PREVIEW
  if constexpr (AP8::effect) {
#endif
  const auto hints = inputSilence(in[0], in[1], d.numSamples, input_flags);
  if (hints.flags != input_flags) {
    ++input_hint_adjustments_;
    input_hint_samples_ += hints.nonzero;
    if (!input_hint_first_bits_) input_hint_first_bits_ = hints.first_bits;
    input_hint_peak_ = std::max(input_hint_peak_, hints.finite_peak);
    input_flags = hints.flags;
  }
#ifdef AP8_PREVIEW
  }
#endif
  uint64_t silence = 0;
  ap7_delivery_t delivery{};
#ifdef AP8_PREVIEW
  ap10_context_t c{};
  if(d.processContext){const auto& p=*d.processContext;c.present=1;c.state=p.state&0x2bf0e;c.rate=p.sampleRate;c.project=p.projectTimeSamples;
   if(c.rate!=requested_rate_)return reject();
   if(c.state&0x100)c.system=p.systemTime;
   if(c.state&0x20000)c.continuous=p.continousTimeSamples;
   if(c.state&0x200)c.music=p.projectTimeMusic;
   if(c.state&0x800)c.bar=p.barPositionMusic;
   if(c.state&0x1000){c.cycle_start=p.cycleStartMusic;c.cycle_end=p.cycleEndMusic;}
   if(c.state&0x400)c.tempo=p.tempo;
   if(c.state&0x2000){c.numerator=p.timeSigNumerator;c.denominator=p.timeSigDenominator;}
   if(c.state&0x8000)c.clock=p.samplesToNextClock;
  }
#endif
  auto r =
#ifdef AP8_PREVIEW
      ap13_process(handle_,static_cast<uint32_t>(d.numSamples),events,event_count,&c,input_flags,in[0],in[1],out[0],out[1],&silence,&delivery,entered_ns);
#else
      queued_
          ? static_cast<int32_t>(ap7_process(
                handle_, static_cast<uint32_t>(d.numSamples),
                preview_ && !changed ? std::numeric_limits<double>::quiet_NaN()
                                     : pending,
                input_flags, in[0], in[1], out[0], out[1],
                &silence, &delivery))
          : ap2_process(handle_, static_cast<uint32_t>(d.numSamples), pending,
                        input_flags, in[0], in[1], out[0], out[1],
                        &silence);
  #endif
  if (r) {
#ifdef AP8_PREVIEW
    returned_.release_requested=true;returned_.release(d,[&](int bus){return eventOutputActive(bus);});
    if (!admission_failure_.code)
      admission_failure_ = {uint32_t(r), c.state, d.numSamples, input_flags,
                            c.rate, c.cycle_start, c.cycle_end};
#endif
    if (!queued_)
      report();
    phase_ = Failed;
    return reject();
  }
  last_callback_rejected_.store(false, std::memory_order_relaxed);
  if (silence == 3 && !delivery.missing_frames) {
    ++silent_callbacks_;
    silent_frames_ += static_cast<uint64_t>(d.numSamples);
  }
  priming_frames_ += delivery.priming_frames;
  underrun_frames_ += delivery.missing_frames;
  underrun_gaps_ += delivery.gaps;
  expired_frames_ += delivery.expired_frames;
  delivered_frames_ += delivery.delivered_frames;
  underrun_callbacks_ += delivery.missing_frames != 0;
  gain_ = pending;
  frames_ += d.numSamples;
  zero_gain_blocks_ += pending == 0.;
  gain_min_ = std::min(gain_min_, pending);
  gain_max_ = std::max(gain_max_, pending);
  ++blocks_;
  d.outputs[0].silenceFlags = silence;
#ifdef AP8_PREVIEW
  if(!deliverResults(d)){phase_=Failed;returned_.release_requested=true;returned_.release(d,[&](int bus){return eventOutputActive(bus);});return reject();}
#endif
  return kResultOk;
}
#ifdef AP8_PREVIEW
int Processor::eventOutputActive(int index)const{
 size_t i=0;for(const auto&b:AP8::buses){if(b.media==kEvent&&b.direction==kOutput&&int(b.index)==index)return bus_active_[i]?1:0;++i;}return -1;
}
bool Processor::deliverResults(ProcessData&d){
 for(size_t i=0;i<1537;++i){
  if(ap10_take_results(handle_,&returned_.packet))return false;
  if(!returned_.packet.events&&!returned_.packet.points)return true;
  if(!returned_.deliver(d,[&](int bus){return eventOutputActive(bus);},[](uint32_t id){for(const auto&p:AP8::parameters)if(p.id==id)return true;return false;})){ap10_fail_results(handle_);return false;}
 }
 ap10_fail_results(handle_);return false;
}
#endif
tresult PLUGIN_API Processor::terminate() {
  Guard g(busy_);
  if (!g.held || owner_ != std::this_thread::get_id() || phase_ == New ||
      phase_ == Terminated)
    return kResultFalse;
  bool clean =
      phase_ == Initialized || phase_ == Setup || phase_ == Deactivated;
  if (input_hint_adjustments_) {
    char text[320];
    const auto n = std::snprintf(text, sizeof(text),
        "{\"event\":\"ap10_input_silence_hints\",\"callbacks\":%llu,"
        "\"nonzero_samples\":%llu,\"first_nonzero_bits\":%u,\"finite_peak\":%.17g}\n",
        (unsigned long long)input_hint_adjustments_,
        (unsigned long long)input_hint_samples_, input_hint_first_bits_, input_hint_peak_);
    if (n > 0 && static_cast<size_t>(n) < sizeof(text))
      diagnostic_report(report_path_, text, static_cast<size_t>(n));
  }
#ifdef AP8_PREVIEW
  if(handle_){ap10_result_stats_t stats{};auto status=ap10_result_stats(handle_,&stats);char text[768];
   auto n=std::snprintf(text,sizeof(text),"{\"event\":\"ap10_process_results\",\"stats_status\":%u,\"events_delivered\":%llu,\"points_delivered\":%llu,\"late_events\":%llu,\"late_points\":%llu,\"host_unrequested_events\":%llu,\"host_unrequested_points\":%llu,\"rejected\":%llu,\"cleanup_sent\":%llu,\"cleanup_unavailable\":%llu,\"active_notes_unreleased\":%zu,\"discarded_on_reset\":%llu,\"pending_events\":%llu,\"pending_points\":%llu}\n",status,(unsigned long long)returned_.events,(unsigned long long)returned_.points,(unsigned long long)stats.late_events,(unsigned long long)stats.late_points,(unsigned long long)returned_.unrequested_events,(unsigned long long)returned_.unrequested_points,(unsigned long long)returned_.rejected,(unsigned long long)returned_.cleanup_sent,(unsigned long long)returned_.cleanup_unavailable,returned_.active_notes(),(unsigned long long)stats.discarded_on_reset,(unsigned long long)stats.pending_events,(unsigned long long)stats.pending_points);
   if(n>0&&size_t(n)<sizeof(text))diagnostic_report(report_path_,text,size_t(n));}
  if (admission_failure_.code) {
    const auto& f = admission_failure_;
    char text[384];
    const auto n = std::snprintf(text, sizeof(text),
        "{\"event\":\"ap10_admission_failure\",\"code\":%u,\"frames\":%d,"
        "\"input_flags\":%llu,\"context_state\":%u,\"rate\":%.17g,"
        "\"cycle_start\":%.17g,\"cycle_end\":%.17g,\"nonfinite_fields\":%u}\n",
        f.code, f.frames, (unsigned long long)f.input_flags, f.context_state,
        std::isfinite(f.rate) ? f.rate : 0.,
        std::isfinite(f.cycle_start) ? f.cycle_start : 0.,
        std::isfinite(f.cycle_end) ? f.cycle_end : 0.,
        unsigned(!std::isfinite(f.rate)) | (unsigned(!std::isfinite(f.cycle_start)) << 1) |
            (unsigned(!std::isfinite(f.cycle_end)) << 2));
    if (n > 0 && static_cast<size_t>(n) < sizeof(text))
      diagnostic_report(report_path_, text, static_cast<size_t>(n));
  }
#endif
  if (handle_) {
    if (queued_)
      report_stats(handle_, report_path_);
    if (preview_) {
      sample_progress(handle_, report_path_);
      ap4_witness_t w{};
      if (!ap4_witness(handle_, &w)) {
        char text[512];
        auto n = std::snprintf(
            text, sizeof(text),
            "{\"event\":\"ap4_sample_comparison\",\"samples\":%llu,\"restored_"
            "samples\":%llu,\"before_edit_samples\":%llu,\"restores\":%llu,"
            "\"edits\":%llu,\"nonzero_samples\":%llu,\"maximum_error\":%.17g,"
            "\"restored_gain\":%.9g}\n",
            (unsigned long long)w.samples,
            (unsigned long long)w.restored_samples,
            (unsigned long long)w.before_edit_samples,
            (unsigned long long)w.restores, (unsigned long long)w.edits,
            (unsigned long long)w.nonzero_samples, w.maximum_error,
            w.restored_gain);
        if (n > 0 && static_cast<size_t>(n) < sizeof(text))
          diagnostic_report(report_path_, text, static_cast<size_t>(n));
      }
    }
    clean =
        (queued_ ? ap3_close(handle_) == 0 : ap2_close(handle_) == 0) && clean;
    if (!clean)
      report();
    handle_ = 0;
  }
  auto r = AudioEffect::terminate();
  if (preview_) {
    char text[1280];
    auto n = std::snprintf(
        text, sizeof(text),
        "{\"event\":\"ap3_proxy_lifecycle\",\"phase\":%d,\"requested_maximum\":"
        "%d,"
        "\"requested_rate\":%.0f,\"requested_mode\":%d,\"frames\":%llu,"
        "\"blocks\":%u,\"callback_rejections\":%llu,"
        "\"rejected_silent_frames\":%llu,\"discontinuities\":%llu,"
        "\"successful_silent_callbacks\":%llu,\"successful_silent_frames\":%llu,"
        "\"priming_frames\":%llu,\"underrun_frames\":%llu,\"underrun_gaps\":%llu,"
        "\"underrun_callbacks\":%llu,\"expired_output_frames\":%llu,\"delivered_frames\":%llu,"
        "\"zero_gain_blocks\":%llu,"
        "\"gain_min\":%.9g,\"gain_"
        "max\":%.9g,"
        "\"clean\":%s}\n",
        static_cast<int>(phase_.load()), requested_maximum_, requested_rate_,
        requested_mode_, (unsigned long long)frames_, blocks_,
        (unsigned long long)callback_rejections_.load(
            std::memory_order_relaxed),
        (unsigned long long)rejected_frames_.load(std::memory_order_relaxed),
        (unsigned long long)discontinuities_.load(std::memory_order_relaxed),
        (unsigned long long)silent_callbacks_, (unsigned long long)silent_frames_,
        (unsigned long long)priming_frames_,
        (unsigned long long)underrun_frames_, (unsigned long long)underrun_gaps_,
        (unsigned long long)underrun_callbacks_, (unsigned long long)expired_frames_,
        (unsigned long long)delivered_frames_,
        (unsigned long long)zero_gain_blocks_, gain_min_, gain_max_,
        clean ? "true" : "false");
    if (n > 0 && static_cast<size_t>(n) < sizeof(text))
      diagnostic_report(report_path_, text, static_cast<size_t>(n));
  }
  phase_ = Terminated;
  return clean ? r : kResultFalse;
}
} // namespace AP2

#ifdef AP8_PREVIEW
namespace AP2 {
Steinberg::tresult Processor::readback(){
 if(state_readback_.empty())return Steinberg::kResultOk; // fresh UI bootstraps via its own refresh

 auto*m=allocateMessage();if(!m)return Steinberg::kResultFalse;m->setMessageID("AP8.readback");m->getAttributes()->setBinary("state",state_readback_.data(),static_cast<uint32_t>(state_readback_.size()));auto r=sendMessage(m);m->release();return r;
}
}
#endif

#ifdef AP8_PREVIEW
Steinberg::tresult AP2::Processor::guiPoll(uint64_t generation,unsigned limit){
 using namespace Steinberg;
 // A host may capture state after disconnecting the controller. Leave final
 // UI acknowledgements for session retirement; there is no host UI consumer.
 if(gui_polling_ || !gui_consumer_ || !getPeer())return kResultOk;
 struct PollGuard{bool&flag;explicit PollGuard(bool&f):flag(f){flag=true;}~PollGuard(){flag=false;}}guard(gui_polling_);
 for(unsigned i=0;i<limit;++i){
  ap11_gui_message_t event{};if(ap11_gui_take(handle_,generation,&event))return kResultFalse;if(!event.kind)break;
  if(event.kind==AP11::EditorStatus){
   const auto observed=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();
   char text[512];auto n=std::snprintf(text,sizeof(text),"{\"event\":\"ap11_editor_status\",\"monotonic_ns\":%llu,\"focus_result\":%u,\"activation\":%llu,\"user_time\":%u,\"requestor_x11\":%u,\"target_x11\":%u,\"view_epoch\":%u,\"focus_flags\":%u,\"open\":%u,\"result\":%u}\n",(unsigned long long)observed,event.focus_result,(unsigned long long)event.activation,event.user_time,event.requestor_x11,event.target_x11,event.view_epoch,event.focus_flags,event.count,event.result);
   if(n>0&&static_cast<size_t>(n)<sizeof(text))diagnostic_report(report_path_,text,static_cast<size_t>(n));
  }
  auto*m=allocateMessage();if(!m){ap11_gui_failure(handle_,generation,AP11::Host);return kResultFalse;}
  m->setMessageID("AP11.event");m->getAttributes()->setInt("generation",int64(generation));m->getAttributes()->setBinary("event",&event,sizeof(event));auto r=sendMessage(m);m->release();
  if(r!=kResultOk){ap11_gui_failure(handle_,generation,AP11::Host);return r;}
 }
 auto code=ap11_gui_failure(handle_,generation,0);
 if(code){auto*m=allocateMessage();if(!m)return kResultFalse;m->setMessageID("AP11.failed");m->getAttributes()->setInt("generation",int64(generation));m->getAttributes()->setInt("code",code);auto r=sendMessage(m);m->release();return r;}
 return kResultOk;
}
#endif
